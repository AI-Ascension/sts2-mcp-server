// SPDX-License-Identifier: MIT

use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant};

const MAX_HEADER_BYTES: usize = 8 * 1024;
const MAX_RESPONSE_BYTES: usize = 64 * 1024;

pub(super) struct HttpResponse {
    pub(super) status: u16,
    pub(super) body: Vec<u8>,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum ReadError {
    Timeout,
    Malformed,
    Oversized,
    Unavailable,
}

fn remaining(deadline: Instant) -> Result<Duration, ReadError> {
    deadline
        .checked_duration_since(Instant::now())
        .filter(|duration| !duration.is_zero())
        .ok_or(ReadError::Timeout)
}

pub(super) fn write_request(
    stream: &mut TcpStream,
    method: &str,
    path: &str,
    headers: &BTreeMap<String, String>,
    body: &[u8],
    deadline: Instant,
) -> Result<(), ReadError> {
    if !matches!(method, "GET" | "POST")
        || !path.starts_with('/')
        || path.len() > 1024
        || !path.bytes().all(|byte| byte.is_ascii_graphic())
    {
        return Err(ReadError::Malformed);
    }
    let mut request = format!("{method} {path} HTTP/1.1\r\n");
    let mut names = BTreeSet::new();
    for (name, value) in headers {
        if !header_name(name)
            || !header_value(value)
            || !names.insert(name.to_ascii_lowercase())
            || name.eq_ignore_ascii_case("transfer-encoding")
            || name.eq_ignore_ascii_case("connection")
        {
            return Err(ReadError::Malformed);
        }
        request.push_str(&format!("{name}: {value}\r\n"));
        if request.len() > MAX_HEADER_BYTES - 21 {
            return Err(ReadError::Oversized);
        }
    }
    request.push_str("Connection: close\r\n\r\n");
    write_bytes(stream, request.as_bytes(), deadline)?;
    write_bytes(stream, body, deadline)
}

fn write_bytes(
    stream: &mut TcpStream,
    mut bytes: &[u8],
    deadline: Instant,
) -> Result<(), ReadError> {
    while !bytes.is_empty() {
        stream
            .set_write_timeout(Some(remaining(deadline)?))
            .map_err(classify_io)?;
        let written = stream.write(bytes).map_err(classify_io)?;
        if written == 0 {
            return Err(ReadError::Unavailable);
        }
        bytes = &bytes[written..];
    }
    remaining(deadline)?;
    Ok(())
}

fn read_bytes(
    stream: &mut TcpStream,
    buffer: &mut [u8],
    deadline: Instant,
) -> Result<usize, ReadError> {
    stream
        .set_read_timeout(Some(remaining(deadline)?))
        .map_err(classify_io)?;
    let count = stream.read(buffer).map_err(classify_io)?;
    remaining(deadline)?;
    if count == 0 {
        return Err(ReadError::Malformed);
    }
    Ok(count)
}

pub(super) fn read_response(
    stream: &mut TcpStream,
    deadline: Instant,
) -> Result<HttpResponse, ReadError> {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 2048];
    let header_end = loop {
        if let Some(end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
            if end + 4 > MAX_HEADER_BYTES {
                return Err(ReadError::Oversized);
            }
            break end;
        }
        let capacity = MAX_HEADER_BYTES
            .saturating_sub(bytes.len())
            .min(buffer.len());
        if capacity == 0 {
            return Err(ReadError::Oversized);
        }
        let read = read_bytes(stream, &mut buffer[..capacity], deadline)?;
        bytes.extend_from_slice(&buffer[..read]);
    };
    let header = std::str::from_utf8(&bytes[..header_end]).map_err(|_| ReadError::Malformed)?;
    let (status, content_length) = parse_headers(header)?;
    let body_start = header_end + 4;
    if bytes.len() - body_start > content_length {
        return Err(ReadError::Malformed);
    }
    let mut body = bytes[body_start..].to_vec();
    while body.len() < content_length {
        let capacity = (content_length - body.len()).min(buffer.len());
        let read = read_bytes(stream, &mut buffer[..capacity], deadline)?;
        body.extend_from_slice(&buffer[..read]);
    }
    Ok(HttpResponse { status, body })
}

fn parse_headers(header: &str) -> Result<(u16, usize), ReadError> {
    let mut lines = header.split("\r\n");
    let status_line = lines.next().ok_or(ReadError::Malformed)?;
    let parts: Vec<_> = status_line.splitn(3, ' ').collect();
    if parts.len() != 3
        || parts[0] != "HTTP/1.1"
        || parts[1].len() != 3
        || !parts[1].bytes().all(|byte| byte.is_ascii_digit())
        || !header_value(parts[2])
    {
        return Err(ReadError::Malformed);
    }
    let status: u16 = parts[1].parse().map_err(|_| ReadError::Malformed)?;
    if !(200..=599).contains(&status) {
        return Err(ReadError::Malformed);
    }
    let mut fields = BTreeMap::new();
    for line in lines {
        let (name, value) = line.split_once(':').ok_or(ReadError::Malformed)?;
        if !header_name(name)
            || !header_value(value)
            || fields
                .insert(name.to_ascii_lowercase(), value.trim_matches([' ', '\t']))
                .is_some()
        {
            return Err(ReadError::Malformed);
        }
    }
    if fields.contains_key("transfer-encoding")
        || fields.contains_key("content-encoding")
        || !fields
            .get("content-type")
            .is_some_and(|value| json_content_type(value))
    {
        return Err(ReadError::Malformed);
    }
    let length = fields.get("content-length").ok_or(ReadError::Malformed)?;
    if length.is_empty() || !length.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(ReadError::Malformed);
    }
    let length = length.parse::<usize>().map_err(|_| ReadError::Malformed)?;
    if length > MAX_RESPONSE_BYTES {
        return Err(ReadError::Oversized);
    }
    Ok((status, length))
}

fn json_content_type(value: &str) -> bool {
    let mut parts = value.split(';');
    parts
        .next()
        .is_some_and(|mime| mime.trim().eq_ignore_ascii_case("application/json"))
        && match (parts.next(), parts.next()) {
            (None, None) => true,
            (Some(parameter), None) => {
                parameter.trim().eq_ignore_ascii_case("charset=utf-8")
                    || parameter.trim().eq_ignore_ascii_case("charset=\"utf-8\"")
            }
            _ => false,
        }
}

fn header_name(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&byte))
}

fn header_value(value: &str) -> bool {
    value
        .bytes()
        .all(|byte| byte == b'\t' || (b' '..=b'~').contains(&byte))
}

pub(super) fn classify_io(error: std::io::Error) -> ReadError {
    if matches!(
        error.kind(),
        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
    ) {
        ReadError::Timeout
    } else {
        ReadError::Unavailable
    }
}

#[cfg(test)]
#[path = "http_tests.rs"]
mod tests;
