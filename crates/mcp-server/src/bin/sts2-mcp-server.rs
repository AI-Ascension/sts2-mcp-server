// SPDX-License-Identifier: MIT

use std::io::{self, BufRead, Write};

use sts2_mcp_server::McpServer;

#[path = "runtime_support/mod.rs"]
mod runtime_http;

fn main() {
    if let Err(error) = run() {
        eprintln!("sts2-mcp-server runtime failed: {error}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let (config, profile) = runtime_http::startup_from_environment()?;
    let gateway_session_id = config.session_id.clone();
    let mcp_session_id = config.mcp_session_id.clone();
    let native_peer_id = config.native_peer_id().map(str::to_owned);
    let enforce_wire_limits = !profile.wire_limits.is_empty();
    let adapter = runtime_http::RuntimeGatewayAdapter::new_with_wire_limits(
        config,
        profile.max_response_bytes,
        profile.wire_limits,
        enforce_wire_limits,
    );
    let server = McpServer::with_catalog_and_sessions(
        adapter,
        profile.catalog,
        gateway_session_id,
        mcp_session_id,
    );
    let mut server = match native_peer_id {
        Some(peer_id) => server.with_native_peer_id(peer_id),
        None => server,
    };
    let max_frame_bytes = server.catalog().max_frame_bytes();
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let stdout = io::stdout();
    let mut output = stdout.lock();
    loop {
        let Some(frame) = read_frame(&mut input, max_frame_bytes)? else {
            return Ok(());
        };
        if let Some(response) = server.handle_message(&frame) {
            write_line(&mut output, &response)?;
        }
        for notification in server.take_notifications() {
            write_line(&mut output, &notification)?;
        }
    }
}

fn write_line(output: &mut impl Write, value: &str) -> Result<(), String> {
    output
        .write_all(value.as_bytes())
        .and_then(|_| output.write_all(b"\n"))
        .and_then(|_| output.flush())
        .map_err(|error| format!("MCP output failed: {error}"))
}

fn read_frame(input: &mut impl BufRead, max_frame_bytes: usize) -> Result<Option<String>, String> {
    let mut bytes = Vec::with_capacity(max_frame_bytes);
    for _ in 0..=max_frame_bytes {
        let mut byte = [0_u8; 1];
        let read = input
            .read(&mut byte)
            .map_err(|error| format!("MCP input failed: {error}"))?;
        if read == 0 {
            if bytes.is_empty() {
                return Ok(None);
            }
            break;
        }
        if byte[0] == b'\n' {
            break;
        }
        bytes.push(byte[0]);
    }
    if bytes.len() > max_frame_bytes {
        return Err(String::from("MCP frame exceeds the byte limit"));
    }
    let frame = String::from_utf8(bytes).map_err(|_| String::from("MCP frame is not UTF-8"))?;
    Ok(Some(frame))
}
