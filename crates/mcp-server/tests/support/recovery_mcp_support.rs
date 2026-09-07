// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;
use std::io::Read;
use std::net::TcpStream;
use std::time::Duration;

pub(crate) type HttpRequest = (String, BTreeMap<String, String>, Vec<u8>);

pub(crate) fn read_http_request(stream: &mut TcpStream) -> Result<HttpRequest, String> {
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|error| error.to_string())?;
    let mut bytes = Vec::new();
    loop {
        let mut byte = [0_u8; 1];
        stream
            .read_exact(&mut byte)
            .map_err(|error| error.to_string())?;
        bytes.push(byte[0]);
        if bytes.len() > 8192 {
            return Err(String::from("HTTP request headers are oversized"));
        }
        if bytes.ends_with(b"\r\n\r\n") {
            break;
        }
    }
    let header_end = bytes.len() - 4;
    let header = std::str::from_utf8(&bytes[..header_end]).map_err(|error| error.to_string())?;
    let mut lines = header.split("\r\n");
    let request_line = lines
        .next()
        .ok_or_else(|| String::from("HTTP request line is missing"))?
        .to_owned();
    let mut headers = BTreeMap::new();
    for line in lines {
        let (name, value) = line
            .split_once(':')
            .ok_or_else(|| String::from("HTTP header is malformed"))?;
        headers.insert(name.to_ascii_lowercase(), value.trim().to_owned());
    }
    let length = headers
        .get("content-length")
        .ok_or_else(|| String::from("HTTP body length is missing"))?
        .parse::<usize>()
        .map_err(|error| error.to_string())?;
    if length > 262_144 {
        return Err(String::from("HTTP body is oversized"));
    }
    let mut body = vec![0_u8; length];
    stream
        .read_exact(&mut body)
        .map_err(|error| error.to_string())?;
    Ok((request_line, headers, body))
}
