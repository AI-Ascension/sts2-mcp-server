// SPDX-License-Identifier: MIT
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

#[path = "support/negotiated_composition_runtime_process_limits.rs"]
mod limits;
#[path = "support/negotiated_runtime_process_support.rs"]
mod support;

const LOOKUP_REQUEST: &str = r#"{"operation":"discovery","scope":{"project_id":"proj-1","run_id":"run-42","episode_id":"episode-7","agent_id":"agent-3"},"authority_epoch":7,"correlation_id":"game-information-binding-discovery"}"#;
const LOOKUP_SCHEMA: &str = "f10f9af01d6be1de104069ba842e7971971e88f27553e782e81174ee7aa1cd58";
const GAME_INFORMATION_SCHEMA: &str =
    "376845b0c86b4afcd2c79ffba753eb7e7e416f5410da26b4dae970cfee2221d9";
const RUNTIME_SCHEMA: &str = "8e99cea36b7ede97532348fd8efe302ca79260895265a7bf14ddf7e006d8ff63";
const BINDING_ID: &str = "58fea90991138ea6fb635df1f5eadd08973ec63eba456d135578677ffee61cfc";

struct ChildProcess(Child);

impl Drop for ChildProcess {
    fn drop(&mut self) {
        if matches!(self.0.try_wait(), Ok(None)) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}

struct GatewayRequest {
    method: String,
    path: String,
    headers: BTreeMap<String, String>,
    body: Vec<u8>,
}

fn spawn_mcp(address: String, lookup_request: &str) -> ChildProcess {
    ChildProcess(
        Command::new(env!("CARGO_BIN_EXE_sts2-mcp-server"))
            .env_clear()
            .env("STS2_RUNTIME_PROFILE", "negotiated-composition-v1")
            .env("STS2_GATEWAY_ADDR", address)
            .env("STS2_GATEWAY_TOKEN", "gateway-test-token")
            .env("STS2_INSTANCE_ID", "instance-1")
            .env("STS2_CALLER_ID", "harness")
            .env("STS2_SESSION_ID", "session-1")
            .env("STS2_MCP_SESSION_ID", "mcp-session-1")
            .env("STS2_LEASE_ID", "lease-1")
            .env("STS2_LEASE_EPOCH", "1")
            .env("STS2_LOOKUP_BINDING_DISCOVERY_REQUEST_JSON", lookup_request)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap(),
    )
}

#[test]
fn shipped_stdio_process_discovers_and_routes_negotiated_profile() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap().to_string();
    let gateway = thread::spawn(move || support::serve_gateway(listener));
    let mut child = spawn_mcp(address, LOOKUP_REQUEST);
    let mut stdin = child.0.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.0.stdout.take().unwrap());

    let initialized = mcp_request(
        &mut stdin,
        &mut stdout,
        &mut child.0,
        json!({
            "jsonrpc":"2.0",
            "id":"initialize",
            "method":"initialize",
            "params":{
                "protocolVersion":"2025-06-18",
                "capabilities":{},
                "clientInfo":{"name":"negotiated-startup-test","version":"1"}
            }
        }),
    );
    assert!(initialized["result"].is_object(), "{initialized}");

    let listed = mcp_request(
        &mut stdin,
        &mut stdout,
        &mut child.0,
        json!({"jsonrpc":"2.0","id":"list","method":"tools/list","params":{}}),
    );
    let tools = listed["result"]["tools"].as_array().unwrap();
    for expected in [
        "sts2.observe",
        "sts2.legal_actions",
        "sts2.wait_for_transition",
        "sts2.reobserve",
        "sts2.game_information_capabilities",
        "sts2.game_information_binding",
        "sts2.game_information.live_observation_bootstrap",
    ] {
        assert!(
            tools.iter().any(|tool| tool["name"] == expected),
            "missing negotiated tool {expected}"
        );
    }
    assert!(
        !tools.iter().any(|tool| tool["name"] == "sts2.map_snapshot"),
        "negotiated startup must not synthesize a map tool without a mapped Gateway offer"
    );

    for (id, name, arguments) in [
        (
            "capabilities-call",
            "sts2.game_information_capabilities",
            json!({
                "instance_id":"instance-1",
                "mcp_session_id":"mcp-session-1",
                "lease_id":"lease-1",
                "lease_epoch":1
            }),
        ),
        (
            "state-call",
            "sts2.observe",
            json!({
                "instance_id":"instance-1",
                "mcp_session_id":"mcp-session-1",
                "lease_id":"lease-1",
                "lease_epoch":1,
                "generation":0
            }),
        ),
        (
            "binding-call",
            "sts2.game_information_binding",
            json!({
                "instance_id":"instance-1",
                "mcp_session_id":"mcp-session-1",
                "lease_id":"lease-1",
                "lease_epoch":1,
                "operation":"observe",
                "project_id":"proj-1",
                "run_id":"run-42",
                "episode_id":"episode-7",
                "agent_id":"agent-3",
                "authority_epoch":7
            }),
        ),
        (
            "bootstrap-call",
            "sts2.game_information.live_observation_bootstrap",
            json!({
                "instance_id":"instance-1",
                "mcp_session_id":"mcp-session-1",
                "lease_id":"lease-1",
                "lease_epoch":1,
                "run_id":"run-42",
                "authority_epoch":7,
                "content_manifest_id":"content-1",
                "locale":"en-US",
                "definition_ref":{
                    "content_manifest_id":"content-1",
                    "entity_kind":"card",
                    "namespaced_id":"ironclad:strike",
                    "variant":null
                },
                "instance_ref":null,
                "max_visible_entities":2,
                "max_item_bytes":4096,
                "max_message_bytes":262144
            }),
        ),
    ] {
        let response = mcp_request(
            &mut stdin,
            &mut stdout,
            &mut child.0,
            json!({
                "jsonrpc":"2.0",
                "id":id,
                "method":"tools/call",
                "params":{"name":name,"arguments":arguments}
            }),
        );
        assert_eq!(response["result"]["isError"], false, "{response}");
    }

    drop(stdin);
    wait_child(&mut child.0);
    gateway.join().unwrap().unwrap();
}

#[test]
fn v1_gateway_fallback_preserves_legacy_catalog_without_live_bootstrap() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap().to_string();
    let gateway = thread::spawn(move || support::serve_gateway_v1(listener));
    let mut child = spawn_mcp(address, LOOKUP_REQUEST);
    let mut stdin = child.0.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.0.stdout.take().unwrap());
    let _ = mcp_request(
        &mut stdin,
        &mut stdout,
        &mut child.0,
        json!({
            "jsonrpc":"2.0",
            "id":"initialize",
            "method":"initialize",
            "params":{
                "protocolVersion":"2025-06-18",
                "capabilities":{},
                "clientInfo":{"name":"negotiated-v1-fallback-test","version":"1"}
            }
        }),
    );
    let listed = mcp_request(
        &mut stdin,
        &mut stdout,
        &mut child.0,
        json!({"jsonrpc":"2.0","id":"list","method":"tools/list","params":{}}),
    );
    assert!(
        !listed["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == "sts2.game_information.live_observation_bootstrap")
    );
    drop(stdin);
    wait_child(&mut child.0);
    gateway.join().unwrap().unwrap();
}

#[test]
fn malformed_owner_discovery_fails_before_gateway_io() {
    let duplicate = LOOKUP_REQUEST.replace(
        r#""authority_epoch":7"#,
        r#""authority_epoch":7,"authority_epoch":8"#,
    );
    let oversized = format!("{}{}", LOOKUP_REQUEST, " ".repeat(2048));
    let unknown = LOOKUP_REQUEST.replace(
        r#""correlation_id":"game-information-binding-discovery""#,
        r#""correlation_id":"game-information-binding-discovery","grant":"read""#,
    );
    for input in [duplicate, oversized, unknown] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap().to_string();
        let mut child = spawn_mcp(address, &input);
        drop(child.0.stdin.take());
        let status = wait_child(&mut child.0);
        assert!(!status.success(), "malformed owner discovery was accepted");
        assert!(
            matches!(
                listener.accept(),
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock
            ),
            "invalid owner discovery must fail before contacting Gateway"
        );
    }
}

#[test]
fn stale_schema_and_oversized_startup_snapshot_fail_closed() {
    for mutation in [
        "stale-lease",
        "schema",
        "unsupported-version",
        "oversized",
        "identity",
        "producer-run",
        "witness",
        "duplicate-offer",
        "recovery",
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap().to_string();
        let peer = thread::spawn(move || support::serve_invalid_snapshot(listener, mutation));
        let mut child = spawn_mcp(address, LOOKUP_REQUEST);
        drop(child.0.stdin.take());
        assert!(
            !wait_child(&mut child.0).success(),
            "{mutation} was accepted"
        );
        let mut stderr = String::new();
        child
            .0
            .stderr
            .take()
            .unwrap()
            .read_to_string(&mut stderr)
            .unwrap();
        assert!(
            stderr.contains("runtime failed"),
            "invalid snapshot failure was not reported: {stderr}"
        );
        peer.join().unwrap().unwrap();
    }
}

fn mcp_request(
    stdin: &mut ChildStdin,
    stdout: &mut BufReader<ChildStdout>,
    child: &mut Child,
    request: Value,
) -> Value {
    writeln!(stdin, "{request}").unwrap();
    stdin.flush().unwrap();
    let mut line = String::new();
    stdout.read_line(&mut line).unwrap();
    if line.is_empty() {
        let _ = child.wait();
        let mut stderr = String::new();
        let _ = child.stderr.take().unwrap().read_to_string(&mut stderr);
        assert!(
            !line.is_empty(),
            "MCP process closed stdout before replying: {stderr}"
        );
    }
    serde_json::from_str(&line).unwrap()
}

fn wait_child(child: &mut Child) -> std::process::ExitStatus {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            return status;
        }
        assert!(
            Instant::now() < deadline,
            "MCP child exceeded its test deadline"
        );
        thread::sleep(Duration::from_millis(10));
    }
}

fn assert_startup_identity(request: &GatewayRequest, correlation: &str) {
    assert_eq!(
        request.headers.get("authorization").map(String::as_str),
        Some("Bearer gateway-test-token")
    );
    assert_eq!(
        request
            .headers
            .get("x-sts2-instance-id")
            .map(String::as_str),
        Some("instance-1")
    );
    assert_eq!(
        request.headers.get("x-sts2-caller-id").map(String::as_str),
        Some("harness")
    );
    assert_eq!(
        request.headers.get("x-sts2-session-id").map(String::as_str),
        Some("session-1")
    );
    assert_eq!(
        request.headers.get("x-mcp-session-id").map(String::as_str),
        Some("mcp-session-1")
    );
    assert_eq!(
        request.headers.get("x-sts2-lease-id").map(String::as_str),
        Some("lease-1")
    );
    assert_eq!(
        request
            .headers
            .get("x-sts2-lease-epoch")
            .map(String::as_str),
        Some("1")
    );
    assert_eq!(
        request
            .headers
            .get("x-sts2-correlation-id")
            .map(String::as_str),
        Some(correlation)
    );
}

fn assert_tool_identity(request: &GatewayRequest, correlation: &str) {
    assert_startup_identity(request, correlation);
    assert_eq!(
        request.headers.get("x-mcp-session-id").map(String::as_str),
        Some("mcp-session-1")
    );
}

fn accept_bounded(listener: &TcpListener) -> Result<(TcpStream, std::net::SocketAddr), String> {
    listener
        .set_nonblocking(true)
        .map_err(|error| error.to_string())?;
    let deadline = Instant::now() + Duration::from_secs(6);
    loop {
        match listener.accept() {
            Ok(connection) => return Ok(connection),
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                if Instant::now() >= deadline {
                    return Err(String::from(
                        "shipped MCP process did not connect to expected Gateway route",
                    ));
                }
                thread::sleep(Duration::from_millis(10));
            }
            Err(error) => return Err(error.to_string()),
        }
    }
}

fn read_request(stream: &mut TcpStream) -> Result<GatewayRequest, String> {
    let mut bytes = Vec::new();
    let mut chunk = [0_u8; 2048];
    let header_end = loop {
        let count = stream.read(&mut chunk).map_err(|error| error.to_string())?;
        if count == 0 {
            return Err(String::from("Gateway client closed before sending headers"));
        }
        bytes.extend_from_slice(&chunk[..count]);
        if let Some(offset) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
            break offset;
        }
        if bytes.len() > 8 * 1024 {
            return Err(String::from("Gateway request headers exceeded test bound"));
        }
    };
    let header = std::str::from_utf8(&bytes[..header_end]).map_err(|error| error.to_string())?;
    let mut lines = header.split("\r\n");
    let mut first = lines
        .next()
        .ok_or("Gateway request line missing")?
        .split_whitespace();
    let method = first.next().ok_or("Gateway method missing")?.to_owned();
    let path = first.next().ok_or("Gateway path missing")?.to_owned();
    let headers: BTreeMap<String, String> = lines
        .map(|line| {
            let (name, value) = line.split_once(':').ok_or("malformed Gateway header")?;
            Ok((name.to_ascii_lowercase(), value.trim().to_owned()))
        })
        .collect::<Result<_, &str>>()?;
    let length = headers
        .get("content-length")
        .ok_or("Gateway content length missing")?
        .parse::<usize>()
        .map_err(|error| error.to_string())?;
    if length > 16 * 1024 {
        return Err(String::from("Gateway request exceeded fixture bound"));
    }
    let body_start = header_end + 4;
    while bytes.len() - body_start < length {
        let count = stream.read(&mut chunk).map_err(|error| error.to_string())?;
        if count == 0 {
            return Err(String::from("Gateway request body truncated"));
        }
        bytes.extend_from_slice(&chunk[..count]);
    }
    Ok(GatewayRequest {
        method,
        path,
        headers,
        body: bytes[body_start..body_start + length].to_vec(),
    })
}

fn write_json_response(stream: &mut TcpStream, body: &Value) -> Result<(), String> {
    write_raw_json_response(stream, &body.to_string())
}

fn write_raw_json_response(stream: &mut TcpStream, body: &str) -> Result<(), String> {
    write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    )
    .map_err(|error| error.to_string())
}
