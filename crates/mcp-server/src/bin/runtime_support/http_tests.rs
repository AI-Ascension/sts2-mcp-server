// SPDX-License-Identifier: MIT
#![allow(clippy::unwrap_used)] // Test setup failures must fail the test immediately.

use super::*;
use std::net::TcpListener;
use std::thread;

fn socket_pair() -> (TcpStream, TcpStream) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    let (server, _) = listener.accept().unwrap();
    (client, server)
}

#[test]
fn strict_json_framing_rejects_ambiguous_or_unsupported_headers() {
    for extra in [
        "content-length: 2\r\n",
        "Transfer-Encoding: chunked\r\n",
        "Content-Type: application/json\r\n",
        "Content-Encoding: gzip\r\n",
        "Bad Header: value\r\n",
        "X-Invalid: a\0b\r\n",
        " folded: value\r\n",
        "X-Test: one\r\nx-test: two\r\n",
    ] {
        let wire = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nContent-Type: application/json\r\n{extra}"
        );
        assert_eq!(
            parse_headers(wire.trim_end_matches("\r\n")),
            Err(ReadError::Malformed),
            "{extra:?}"
        );
    }
    for length in ["+2", "-2", "2, 2", "", "2 2"] {
        assert_eq!(
            parse_headers(&format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {length}"
            )),
            Err(ReadError::Malformed)
        );
    }
    for content_type in [
        "text/html",
        "application/json; charset=latin1",
        "application/json; x=a",
        "application/json; charset=utf-8; charset=utf-8",
    ] {
        assert_eq!(
            parse_headers(&format!(
                "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: 2"
            )),
            Err(ReadError::Malformed)
        );
    }
    assert_eq!(
        parse_headers("HTTP/1.1 200 OK\r\nContent-Length: 2"),
        Err(ReadError::Malformed)
    );
    for status in ["0200", "+200", "100", "600", "200\tOK"] {
        assert!(
            parse_headers(&format!(
                "HTTP/1.1 {status} OK\r\nContent-Type: application/json\r\nContent-Length: 2"
            ))
            .is_err()
        );
    }
    for value in [
        "application/json",
        "Application/JSON; charset=utf-8",
        "application/json; charset=\"UTF-8\"",
    ] {
        assert!(json_content_type(value));
    }
}

#[test]
fn semantic_response_budget_is_128_kibibytes() {
    let wire = |size| {
        format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {size}")
    };
    assert!(parse_headers(&wire(128 * 1024)).is_ok());
    assert_eq!(
        parse_headers(&wire(128 * 1024 + 1)),
        Err(ReadError::Oversized)
    );
}

#[test]
fn bounded_response_accepts_json_and_rejects_truncation_and_header_overflow() {
    let (mut client, mut server) = socket_pair();
    server
        .write_all(
            b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nContent-Type: application/json\r\n\r\n{}",
        )
        .unwrap();
    let response = read_response(&mut client, Instant::now() + Duration::from_secs(1)).unwrap();
    assert_eq!(response.status, 200);
    assert_eq!(response.body, b"{}");
    for wire in [
        format!(
            "HTTP/1.1 200 OK\r\nX-Pad: {}\r\nContent-Length: 2\r\nContent-Type: application/json\r\n\r\n{{}}",
            "a".repeat(MAX_HEADER_BYTES)
        ),
        String::from(
            "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nContent-Type: application/json\r\n\r\n{",
        ),
    ] {
        let (mut client, mut server) = socket_pair();
        server.write_all(wire.as_bytes()).unwrap();
        drop(server);
        assert!(read_response(&mut client, Instant::now() + Duration::from_secs(1)).is_err());
    }
}

#[test]
fn slow_drip_header_and_body_share_one_total_deadline() {
    for prefix in [
        b"".as_slice(),
        b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 100\r\n\r\n"
            .as_slice(),
    ] {
        let (mut client, mut server) = socket_pair();
        server.write_all(prefix).unwrap();
        let worker = thread::spawn(move || {
            for _ in 0..30 {
                if server.write_all(b" ").is_err() {
                    break;
                }
                thread::sleep(Duration::from_millis(10));
            }
        });
        let start = Instant::now();
        assert_eq!(
            read_response(&mut client, start + Duration::from_millis(60)).err(),
            Some(ReadError::Timeout)
        );
        assert!(start.elapsed() < Duration::from_millis(250));
        drop(client);
        worker.join().unwrap();
    }
}

#[test]
fn expired_deadline_and_blocked_writer_return_timeout() {
    let (mut client, _server) = socket_pair();
    assert_eq!(
        write_bytes(&mut client, b"x", Instant::now()),
        Err(ReadError::Timeout)
    );
    // A single oversized send can complete immediately on some platforms
    // (Windows loopback copies the whole user buffer), so write in bounded
    // chunks until the unread peer forces the writer to block.
    let bytes = vec![0; 1024 * 1024];
    let start = Instant::now();
    let deadline = start + Duration::from_millis(60);
    let mut outcome = Ok(());
    for _ in 0..64 {
        outcome = write_bytes(&mut client, &bytes, deadline);
        if outcome.is_err() {
            break;
        }
    }
    assert_eq!(outcome, Err(ReadError::Timeout));
    assert!(start.elapsed() < Duration::from_millis(250));
}

#[test]
fn request_rejects_header_injection_and_case_collisions() {
    let (mut client, _server) = socket_pair();
    for headers in [
        BTreeMap::from([(
            String::from("Authorization"),
            String::from("a\r\nInjected: b"),
        )]),
        BTreeMap::from([
            (String::from("Content-Length"), String::from("2")),
            (String::from("content-length"), String::from("2")),
        ]),
        BTreeMap::from([(String::from("Transfer-Encoding"), String::from("chunked"))]),
    ] {
        assert_eq!(
            write_request(
                &mut client,
                "GET",
                "/state",
                &headers,
                b"",
                Instant::now() + Duration::from_secs(1)
            ),
            Err(ReadError::Malformed)
        );
    }
}
