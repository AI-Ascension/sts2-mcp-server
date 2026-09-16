// SPDX-License-Identifier: MIT
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

const FRAMES: &str = include_str!("../../../protocol-artifact/exact-restore-v1/golden/frames.json");

struct ChildProcess(Child);

impl Drop for ChildProcess {
    fn drop(&mut self) {
        if matches!(self.0.try_wait(), Ok(None)) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}

fn frames() -> Vec<Value> {
    serde_json::from_str::<Value>(FRAMES).unwrap()["frames"]
        .as_array()
        .unwrap()
        .clone()
}

fn gateway_wrapper(frame: &Value, request: bool) -> Value {
    let role = if request { "harness" } else { "gateway" };
    json!({
        "contract": "sts2-exact-restore-gateway-v1",
        "schema_digest": sts2_mcp_server::EXACT_RESTORE_GATEWAY_SCHEMA_DIGEST,
        "message_id": frame["message_id"],
        "correlation_id": frame["correlation_id"],
        "actor": {"principal_id":"harness","role":role},
        "auth":{"principal_id":"harness","capability":"exact_restore","proof":null},
        "kind": if request {"exact_restore_request"} else {"exact_restore_response"},
        "payload":{"frame":frame},
    })
}

fn read_gateway_request(
    stream: &mut TcpStream,
) -> (String, String, BTreeMap<String, String>, Vec<u8>) {
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 2048];
    let header_end = loop {
        let count = stream.read(&mut buffer).unwrap();
        assert_ne!(count, 0);
        bytes.extend_from_slice(&buffer[..count]);
        if let Some(offset) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
            break offset;
        }
        assert!(bytes.len() <= 8 * 1024);
    };
    let header = String::from_utf8(bytes[..header_end].to_vec()).unwrap();
    let mut lines = header.split("\r\n");
    let mut first = lines.next().unwrap().split_whitespace();
    let method = first.next().unwrap().to_owned();
    let path = first.next().unwrap().to_owned();
    let headers: BTreeMap<String, String> = lines
        .map(|line| {
            let (name, value) = line.split_once(':').unwrap();
            (name.to_ascii_lowercase(), value.trim().to_owned())
        })
        .collect();
    let length = headers["content-length"].parse::<usize>().unwrap();
    let body_start = header_end + 4;
    while bytes.len() - body_start < length {
        let count = stream.read(&mut buffer).unwrap();
        assert_ne!(count, 0);
        bytes.extend_from_slice(&buffer[..count]);
    }
    (
        method,
        path,
        headers,
        bytes[body_start..body_start + length].to_vec(),
    )
}

fn wait_for_peer(listener: TcpListener) -> thread::JoinHandle<Result<(), String>> {
    thread::spawn(move || {
        listener.set_nonblocking(true).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        let (mut stream, _) = loop {
            match listener.accept() {
                Ok(connection) => break connection,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(
                        Instant::now() < deadline,
                        "MCP process never connected to Gateway"
                    );
                    thread::sleep(Duration::from_millis(10));
                }
                Err(error) => return Err(format!("Gateway fixture accept failed: {error}")),
            }
        };
        let (method, path, headers, body) = read_gateway_request(&mut stream);
        let frames = frames();
        assert_eq!(method, "POST");
        assert_eq!(path, "/v1/exact-restore/begin");
        assert_eq!(
            headers.get("x-sts2-correlation-id").map(String::as_str),
            frames[9]["correlation_id"].as_str()
        );
        assert_eq!(
            headers.get("authorization").map(String::as_str),
            Some("Bearer recovery-secret")
        );
        assert_eq!(
            headers
                .get("x-sts2-recovery-capability")
                .map(String::as_str),
            Some("exact_restore")
        );
        assert_eq!(
            serde_json::from_slice::<Value>(&body).unwrap(),
            gateway_wrapper(&frames[9], true)
        );
        let response = gateway_wrapper(&frames[22], false).to_string();
        write!(
            stream,
            "HTTP/1.1 422 Unprocessable Content\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            response.len(),
            response
        )
        .unwrap();
        Ok(())
    })
}

#[test]
fn shipped_stdio_process_maps_begin_to_strict_gateway_and_keeps_typed_refusal()
-> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let gateway_address = listener.local_addr().unwrap().to_string();
    let peer = wait_for_peer(listener);
    let frames = frames();
    let begin = gateway_wrapper(&frames[9], true);
    let initialize = json!({
        "jsonrpc":"2.0",
        "id":1,
        "method":"initialize",
        "params":{
            "protocolVersion":"2025-03-26",
            "capabilities":{},
            "clientInfo":{"name":"exact-restore-process-test","version":"1"}
        }
    });
    let begin_call = json!({
        "jsonrpc":"2.0",
        "id":2,
        "method":"tools/call",
        "params":{"name":"sts2.exact_restore.begin","arguments":begin}
    });
    let mut child = ChildProcess(
        Command::new(env!("CARGO_BIN_EXE_sts2-mcp-server"))
            .env_clear()
            .env("STS2_RUNTIME_PROFILE", "exact-restore-v1")
            .env("STS2_GATEWAY_ADDR", gateway_address)
            .env("STS2_RECOVERY_TOKEN", "recovery-secret")
            .env("STS2_INSTANCE_ID", "02ab8278-c166-4557-bd6d-8f7575484a55")
            .env("STS2_CALLER_ID", "harness")
            .env("STS2_SESSION_ID", "session-example")
            .env("STS2_MCP_SESSION_ID", "mcp-session-1")
            .env("STS2_LEASE_ID", "c0f5a147-1ba1-4a1c-9a25-1b46935403ef")
            .env("STS2_LEASE_EPOCH", "8")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    {
        let stdin = child.0.stdin.as_mut().unwrap();
        writeln!(stdin, "{initialize}").unwrap();
        writeln!(stdin, "{begin_call}").unwrap();
    }
    drop(child.0.stdin.take());
    let deadline = Instant::now() + Duration::from_secs(8);
    let status = loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            break status;
        }
        assert!(
            Instant::now() < deadline,
            "MCP process exceeded its test deadline"
        );
        thread::sleep(Duration::from_millis(10));
    };
    let mut output = String::new();
    child
        .0
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut output)
        .unwrap();
    let mut error_output = String::new();
    child
        .0
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut error_output)
        .unwrap();
    assert!(status.success(), "MCP process failed: {error_output}");
    peer.join()
        .map_err(|_| "Gateway fixture thread panicked")??;

    let responses: Vec<Value> = output
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(responses.len(), 2);
    assert_eq!(responses[1]["result"]["isError"], true);
    let text = responses[1]["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(text).unwrap()["payload"]["error_code"],
        "no_restore_adapter"
    );
    Ok(())
}
