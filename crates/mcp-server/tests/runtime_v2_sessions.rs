// SPDX-License-Identifier: MIT

#[path = "runtime_v2_support/mod.rs"]
#[allow(dead_code)]
mod support;

use sts2_mcp_server::{GatewayResponse, JsonValue, McpServer, ToolCatalog};
use support::{RecordingGateway, reconcile_call, state_call, state_response, submit_call};

#[test]
fn executable_uses_configured_mcp_session_and_gateway_session()
-> Result<(), Box<dyn std::error::Error>> {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::process::{Command, Stdio};
    use std::time::Duration;
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let address = listener.local_addr()?;
    listener.set_nonblocking(true)?;
    let peer = std::thread::spawn(move || -> Result<String, String> {
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        let mut stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(error)
                    if error.kind() == std::io::ErrorKind::WouldBlock
                        && std::time::Instant::now() < deadline =>
                {
                    std::thread::sleep(Duration::from_millis(5))
                }
                Err(error) => return Err(error.to_string()),
            }
        };
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .map_err(|e| e.to_string())?;
        let mut bytes = Vec::new();
        loop {
            let mut byte = [0];
            stream.read_exact(&mut byte).map_err(|e| e.to_string())?;
            bytes.push(byte[0]);
            if bytes.len() > 16384 {
                return Err("request too large".to_owned());
            }
            if bytes.ends_with(b"\r\n\r\n") {
                break;
            }
        }
        let headers = String::from_utf8(bytes).map_err(|e| e.to_string())?;
        let length = headers
            .lines()
            .find_map(|line| line.strip_prefix("Content-Length: "))
            .ok_or("no length")?
            .trim()
            .parse::<usize>()
            .map_err(|e| e.to_string())?;
        if length > 16384 {
            return Err("body too large".to_owned());
        }
        let mut body = vec![0; length];
        stream.read_exact(&mut body).map_err(|e| e.to_string())?;
        let response = state_response("state-1", 4).to_json();
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            response.len(),
            response
        )
        .map_err(|e| e.to_string())?;
        Ok(format!(
            "{headers}{}",
            String::from_utf8(body).map_err(|e| e.to_string())?
        ))
    });
    let mut child = Command::new(env!("CARGO_BIN_EXE_sts2-mcp-server"))
        .env_clear()
        // Windows sockets cannot initialize in a child without `SystemRoot`;
        // pass only that variable through (absent, and therefore a no-op, elsewhere).
        .envs(std::env::var_os("SystemRoot").map(|root| ("SystemRoot", root)))
        .env("STS2_RUNTIME_PROFILE", "runtime-v2")
        .env("STS2_GATEWAY_TOKEN", "test-only")
        .env("STS2_GATEWAY_ADDR", address.to_string())
        .env("STS2_SESSION_ID", "session-1")
        .env("STS2_MCP_SESSION_ID", "mcp-1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    let call = state_call("state-1", "instance-1", "mcp-1", "lease-1", 1, 4);
    writeln!(child.stdin.take().ok_or("no stdin")?, "{call}")?;
    let output = child.wait_with_output()?;
    let request = peer.join().map_err(|_| "peer panicked")??;
    assert!(output.status.success());
    assert!(String::from_utf8(output.stdout)?.contains("\"isError\":false"));
    assert!(request.contains("x-mcp-session-id: mcp-1\r\n"));
    assert!(request.contains("x-sts2-session-id: session-1\r\n"));
    assert!(request.contains("\"session_id\":\"session-1\""));
    Ok(())
}

#[test]
fn separate_mcp_and_gateway_sessions_preserve_both_namespaces() {
    let mut server = McpServer::with_catalog_and_sessions(
        RecordingGateway::new([Ok(GatewayResponse {
            status: 200,
            body: state_response("state-1", 4),
        })]),
        ToolCatalog::runtime_v2(),
        "session-1",
        "mcp-1",
    );
    let result = server.handle_frame(&state_call(
        "state-1",
        "instance-1",
        "mcp-1",
        "lease-1",
        1,
        4,
    ));
    assert!(result.contains("\"isError\":false"), "{result}");
    let _ = server.handle_frame(&submit_call(
        "action-1",
        "instance-1",
        "mcp-1",
        "lease-1",
        1,
        4,
        "op-1",
    ));
    let _ = server.handle_frame(&reconcile_call(
        "reconcile-1",
        "instance-1",
        "mcp-1",
        "lease-1",
        1,
        4,
        "op-1",
    ));
    assert_eq!(server.gateway().requests.len(), 3);
    for request in &server.gateway().requests {
        assert_eq!(request.correlation.mcp_session_id, "mcp-1");
        assert_eq!(
            request.headers.get("x-mcp-session-id").map(String::as_str),
            Some("mcp-1")
        );
        assert_eq!(
            request.headers.get("x-sts2-session-id").map(String::as_str),
            Some("session-1")
        );
        if let Some(JsonValue::Object(body)) = &request.body {
            assert_eq!(
                body.get("session_id"),
                Some(&JsonValue::string("session-1"))
            );
        }
    }
}

#[test]
fn a_foreign_mcp_session_is_rejected_before_forwarding() {
    let mut server = McpServer::with_catalog_and_sessions(
        RecordingGateway::new([]),
        ToolCatalog::runtime_v2(),
        "session-1",
        "mcp-1",
    );
    for call in [
        state_call("state-1", "instance-1", "foreign", "lease-1", 1, 4),
        submit_call("action-1", "instance-1", "foreign", "lease-1", 1, 4, "op-1"),
        reconcile_call(
            "reconcile-1",
            "instance-1",
            "foreign",
            "lease-1",
            1,
            4,
            "op-1",
        ),
    ] {
        assert!(
            server
                .handle_frame(&call)
                .contains("MCP session identity does not match")
        );
    }
    assert!(server.gateway().requests.is_empty());
}
