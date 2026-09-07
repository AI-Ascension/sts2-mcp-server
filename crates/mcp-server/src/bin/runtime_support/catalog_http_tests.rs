// SPDX-License-Identifier: MIT

use super::http::MAP_MAX_RESPONSE_BYTES;
use super::*;
use std::io::{BufRead, Read, Write};
use std::net::TcpListener;

#[test]
fn compact_catalog_refusal_survives_the_real_http_adapter() -> Result<(), String> {
    for (status, code, correlation, tool, extra, accepted) in [
        (
            409,
            "stale_generation",
            "request-1",
            "sts2.legal_actions",
            false,
            true,
        ),
        (
            503,
            "host_not_configured",
            "request-1",
            "sts2.legal_actions",
            false,
            true,
        ),
        (
            503,
            "host_observation_unavailable",
            "request-1",
            "sts2.legal_actions",
            false,
            true,
        ),
        (
            409,
            "stale_generation",
            "wrong",
            "sts2.legal_actions",
            false,
            false,
        ),
        (
            409,
            "stale_generation",
            "request-1",
            "sts2.legal_actions",
            true,
            false,
        ),
        (
            409,
            "stale_generation",
            "request-1",
            "sts2.observe",
            false,
            false,
        ),
        (
            503,
            "secret-marker",
            "request-1",
            "sts2.legal_actions",
            false,
            false,
        ),
    ] {
        let body = format!(
            "{{\"correlation_id\":\"{correlation}\",\"error_code\":\"{code}\",\"recovery\":\"reobserve\"{}}}",
            if extra {
                ",\"private\":\"secret-marker\""
            } else {
                ""
            }
        );
        let listener = TcpListener::bind("127.0.0.1:0").map_err(|error| error.to_string())?;
        let mut config = config();
        config.gateway_address = listener.local_addr().map_err(|error| error.to_string())?;
        let server_thread = std::thread::spawn(move || serve(listener, status, body));
        let adapter = RuntimeGatewayAdapter::new(config, 128 * 1024);
        let mut server = sts2_mcp_server::McpServer::with_catalog_and_sessions(
            adapter,
            sts2_mcp_server::ToolCatalog::runtime_v3_gameplay(),
            "configured-session",
            "configured-session",
        );
        let state = if tool == "sts2.legal_actions" {
            ",\"state_id\":\"live:1\""
        } else {
            ""
        };
        let result = server.handle_frame(&format!("{{\"jsonrpc\":\"2.0\",\"id\":\"request-1\",\"method\":\"tools/call\",\"params\":{{\"name\":\"{tool}\",\"arguments\":{{\"instance_id\":\"configured-instance\",\"mcp_session_id\":\"configured-session\",\"lease_id\":\"configured-lease\",\"lease_epoch\":7,\"generation\":1{state}}}}}}}"));
        server_thread
            .join()
            .map_err(|_| "HTTP fixture thread failed".to_owned())??;
        assert!(result.contains("\"isError\":true"));
        assert_eq!(result.contains("reobserve"), accepted);
        assert!(!result.contains("secret-marker"));
    }
    Ok(())
}

#[test]
fn map_profile_crosses_the_real_http_adapter_with_a_complete_response() -> Result<(), String> {
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|error| error.to_string())?;
    let mut config = config();
    config.gateway_address = listener.local_addr().map_err(|error| error.to_string())?;
    let body = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../protocol-artifact/runtime-map-v1/golden/snapshot-response.json"
    ))
    .replace("\"instance-1\"", "\"configured-instance\"")
    .replace("\"session-1\"", "\"configured-session\"")
    .replace("\"lease-1\"", "\"configured-lease\"");
    let server_thread = std::thread::spawn(move || serve(listener, 200, body));
    let adapter = RuntimeGatewayAdapter::new(config, MAP_MAX_RESPONSE_BYTES);
    let mut server = sts2_mcp_server::McpServer::with_catalog_and_sessions(
        adapter,
        sts2_mcp_server::ToolCatalog::runtime_map_v1(),
        "configured-session",
        "configured-session",
    );
    let output = server.handle_frame(
        r#"{"jsonrpc":"2.0","id":"corr-42","method":"tools/call","params":{"name":"sts2.map_snapshot","arguments":{"instance_id":"configured-instance","mcp_session_id":"configured-session","lease_id":"configured-lease","lease_epoch":7,"generation":42}}}"#,
    );
    let fixture = server_thread
        .join()
        .map_err(|_| "HTTP fixture thread failed".to_owned())?;
    if let Err(error) = fixture {
        return Err(format!("{error}; MCP output: {output}"));
    }
    assert!(output.contains("\"isError\":false"), "{output}");
    assert!(output.contains("map:1:1:0"), "{output}");
    Ok(())
}

fn serve(listener: TcpListener, status: u16, body: String) -> Result<(), String> {
    listener
        .set_nonblocking(true)
        .map_err(|error| error.to_string())?;
    let start = std::time::Instant::now();
    let mut stream = loop {
        if let Ok((stream, _)) = listener.accept() {
            break stream;
        }
        if start.elapsed() > std::time::Duration::from_secs(2) {
            return Err("HTTP fixture accept deadline".into());
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    };
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(2)))
        .map_err(|error| error.to_string())?;
    let mut reader = std::io::BufReader::new(&mut stream);
    let mut length = 0;
    for _ in 0..64 {
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .map_err(|error| error.to_string())?;
        if line == "\r\n" {
            break;
        }
        if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
            length = value
                .trim()
                .parse::<usize>()
                .map_err(|error| error.to_string())?;
        }
    }
    if length > 262_144 {
        return Err("HTTP fixture request bound".into());
    }
    reader
        .read_exact(&mut vec![0; length])
        .map_err(|error| error.to_string())?;
    let response = format!(
        "HTTP/1.1 {status} Fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream
        .write_all(response.as_bytes())
        .map_err(|error| error.to_string())
}
