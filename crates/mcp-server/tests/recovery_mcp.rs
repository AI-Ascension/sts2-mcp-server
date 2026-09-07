// SPDX-License-Identifier: MIT

use std::io::Write;
use std::net::TcpListener;
use std::process::{Command, Stdio};

use serde_json::Value;
use sts2_mcp_server::{
    GatewayAdapter, GatewayError, GatewayMethod, GatewayRequest, GatewayResponse, JsonValue,
    McpServer, RECOVERY_RUNTIME_V3_SCHEMA_DIGEST, ToolCatalog, parse_json,
    validate_recovery_request, validate_recovery_response,
};

const BOOTSTRAP_REQUEST: &str = include_str!(
    "../../../protocol-artifact/watchdog-recovery-v1/fixtures/valid/bootstrap-request.json"
);
const BOOTSTRAP_RESPONSE: &str = include_str!(
    "../../../protocol-artifact/watchdog-recovery-v1/fixtures/valid/bootstrap-response.json"
);
const LEASE_ACQUIRE_REQUEST: &str = include_str!(
    "../../../protocol-artifact/watchdog-recovery-v1/fixtures/valid/lease-acquire-request.json"
);
const LEASE_ACQUIRE_RESPONSE: &str = include_str!(
    "../../../protocol-artifact/watchdog-recovery-v1/fixtures/valid/lease-acquire-response.json"
);
const OPERATION_INTENT_REQUEST: &str = include_str!(
    "../../../protocol-artifact/watchdog-recovery-v1/fixtures/valid/operation-intent-request.json"
);

#[path = "support/recovery_mcp_support.rs"]
mod support;
use support::{HttpRequest, read_http_request};

#[derive(Clone)]
enum GatewayOutcome {
    Error(GatewayError),
    Response {
        status: u16,
        body: JsonValue,
        replace_correlation: bool,
    },
}

struct RecordingGateway {
    requests: Vec<GatewayRequest>,
    outcome: GatewayOutcome,
}

impl RecordingGateway {
    fn error(error: GatewayError) -> Self {
        Self {
            requests: Vec::new(),
            outcome: GatewayOutcome::Error(error),
        }
    }

    fn response(body: JsonValue, replace_correlation: bool) -> Self {
        Self {
            requests: Vec::new(),
            outcome: GatewayOutcome::Response {
                status: 200,
                body,
                replace_correlation,
            },
        }
    }
}

impl GatewayAdapter for RecordingGateway {
    fn forward(&mut self, request: GatewayRequest) -> Result<GatewayResponse, GatewayError> {
        self.requests.push(request);
        match self.outcome.clone() {
            GatewayOutcome::Error(error) => Err(error),
            GatewayOutcome::Response {
                status,
                mut body,
                replace_correlation,
            } => {
                if replace_correlation {
                    let correlation = self
                        .requests
                        .last()
                        .map(|request| request.correlation.mcp_request_id.stable_text())
                        .ok_or(GatewayError::MalformedResponse)?;
                    body.as_object_mut()
                        .ok_or(GatewayError::MalformedResponse)?
                        .insert("correlation_id".to_owned(), JsonValue::string(correlation));
                }
                Ok(GatewayResponse { status, body })
            }
        }
    }
}

fn server(gateway: RecordingGateway) -> McpServer<RecordingGateway> {
    McpServer::with_catalog_and_sessions(
        gateway,
        ToolCatalog::watchdog_recovery_v1(),
        "gateway-session",
        "mcp-session",
    )
    .with_recovery_identity(
        "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
        "harness",
        Some(String::from("recovery-proof")),
    )
}

fn fixture_payload(fixture: &str) -> Result<JsonValue, String> {
    let frame = parse_json(fixture)?;
    frame
        .as_object()
        .and_then(|object| object.get("payload"))
        .cloned()
        .ok_or_else(|| String::from("fixture has no payload"))
}

fn call(tool: &str, payload: &JsonValue, session: &str) -> String {
    format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":\"call-1\",\"method\":\"tools/call\",\"params\":{{\"name\":\"{tool}\",\"arguments\":{{\"mcp_session_id\":\"{session}\",\"payload\":{}}}}}}}",
        payload.to_json()
    )
}

fn operation_payload_with_approved_action_digest() -> Result<JsonValue, String> {
    let mut payload = fixture_payload(OPERATION_INTENT_REQUEST)?;
    let operation = payload
        .as_object_mut()
        .and_then(|object| object.get_mut("operation"))
        .and_then(JsonValue::as_object_mut)
        .ok_or_else(|| String::from("operation fixture is missing operation"))?;
    let action = operation
        .get_mut("action")
        .and_then(JsonValue::as_object_mut)
        .ok_or_else(|| String::from("operation fixture is missing action"))?;
    action.insert(
        String::from("schema_digest"),
        JsonValue::string(RECOVERY_RUNTIME_V3_SCHEMA_DIGEST),
    );
    Ok(payload)
}

fn operation_dispatch_payload() -> Result<JsonValue, String> {
    let mut payload = operation_payload_with_approved_action_digest()?;
    let operation = payload
        .as_object_mut()
        .and_then(|object| object.get_mut("operation"))
        .and_then(JsonValue::as_object_mut)
        .ok_or_else(|| String::from("operation fixture is missing operation"))?;
    operation.remove("expected_boundary");
    operation.remove("action");
    Ok(payload)
}

#[test]
fn recovery_catalog_is_exactly_the_nine_sideband_tools() -> Result<(), String> {
    let mut server = server(RecordingGateway::error(GatewayError::Timeout));
    let response = server
        .handle_frame("{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\",\"params\":{}}");
    let value: Value = serde_json::from_str(&response).map_err(|error| error.to_string())?;
    let tools = value
        .get("result")
        .and_then(|result| result.get("tools"))
        .and_then(Value::as_array)
        .ok_or_else(|| String::from("tools/list did not return a tool array"))?;
    let mut names = tools
        .iter()
        .filter_map(|tool| tool.get("name").and_then(Value::as_str))
        .collect::<Vec<_>>();
    names.sort_unstable();
    assert_eq!(
        names,
        vec![
            "watchdog.bootstrap",
            "watchdog.host_fence",
            "watchdog.lease_acquire",
            "watchdog.lease_renew",
            "watchdog.lease_revoke",
            "watchdog.operation_dispatch",
            "watchdog.operation_intent",
            "watchdog.operation_lookup",
            "watchdog.operation_reconcile",
        ]
    );
    Ok(())
}

#[test]
fn recovery_mapping_uses_fixed_route_capability_and_no_retry_after_timeout() -> Result<(), String> {
    let payload = operation_dispatch_payload()?;
    let mut server = server(RecordingGateway::error(GatewayError::Timeout));
    let response = server.handle_frame(&call(
        "watchdog.operation_dispatch",
        &payload,
        "mcp-session",
    ));
    assert!(
        response.contains("\\\"status\\\":\\\"UNKNOWN\\\""),
        "{response}"
    );
    assert!(
        response.contains("\\\"mutation_resubmitted\\\":false"),
        "{response}"
    );
    assert_eq!(server.gateway().requests.len(), 1);
    let request = &server.gateway().requests[0];
    assert_eq!(request.method, GatewayMethod::Post);
    assert_eq!(request.path, "/v1/recovery/operation/dispatch");
    assert_eq!(request.correlation.mcp_session_id, "mcp-session");
    assert_eq!(
        request
            .headers
            .get("x-sts2-recovery-capability")
            .map(String::as_str),
        Some("operation_submit")
    );
    let frame = request
        .body
        .as_ref()
        .and_then(JsonValue::as_object)
        .ok_or_else(|| String::from("recovery request body is not an object"))?;
    assert_eq!(frame.len(), 9);
    assert!(frame.get("instance_id").is_none());
    assert!(frame.get("session_id").is_none());
    assert!(frame.get("lease_id").is_none());
    assert!(frame.get("lease_epoch").is_none());
    Ok(())
}

#[test]
fn mixed_runtime_digest_and_foreign_mcp_session_are_rejected_before_gateway() -> Result<(), String>
{
    let mixed_payload = fixture_payload(OPERATION_INTENT_REQUEST)?;
    let mut mixed = server(RecordingGateway::error(GatewayError::Timeout));
    let mixed_response = mixed.handle_frame(&call(
        "watchdog.operation_intent",
        &mixed_payload,
        "mcp-session",
    ));
    assert!(mixed_response.contains("unsupported Runtime-v3 schema"));
    assert!(mixed.gateway().requests.is_empty());

    let bootstrap = fixture_payload(BOOTSTRAP_REQUEST)?;
    let mut foreign = server(RecordingGateway::error(GatewayError::Timeout));
    let foreign_response =
        foreign.handle_frame(&call("watchdog.bootstrap", &bootstrap, "foreign-session"));
    assert!(foreign_response.contains("MCP session identity does not match"));
    assert!(foreign.gateway().requests.is_empty());
    Ok(())
}

#[test]
fn not_found_is_typed_without_claiming_non_execution_and_response_secrets_are_redacted()
-> Result<(), String> {
    let bootstrap = fixture_payload(BOOTSTRAP_REQUEST)?;
    let mut not_found = server(RecordingGateway::error(GatewayError::NotFound));
    let not_found_response =
        not_found.handle_frame(&call("watchdog.bootstrap", &bootstrap, "mcp-session"));
    assert!(not_found_response.contains("NOT_FOUND"));
    assert!(!not_found_response.contains("\"status\":\"UNKNOWN\""));

    let lease_request = fixture_payload(LEASE_ACQUIRE_REQUEST)?;
    let lease_response = parse_json(LEASE_ACQUIRE_RESPONSE)?;
    let mut redacted = server(RecordingGateway::response(lease_response, true));
    let response = redacted.handle_frame(&call(
        "watchdog.lease_acquire",
        &lease_request,
        "mcp-session",
    ));
    assert!(response.contains("\"isError\":false"), "{response}");
    assert!(!response.contains("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"));
    assert!(!response.contains("contract-test-proof"));
    Ok(())
}

#[test]
fn recovery_frame_identity_and_response_correlation_are_checked() -> Result<(), String> {
    let mut request = parse_json(BOOTSTRAP_REQUEST)?;
    let correlation = request
        .as_object()
        .and_then(|object| object.get("correlation_id"))
        .and_then(JsonValue::as_string)
        .ok_or_else(|| String::from("bootstrap correlation is missing"))?
        .to_owned();
    assert!(
        validate_recovery_request(
            &request,
            "bootstrap",
            &correlation,
            Some("22222222-2222-4222-8222-222222222222")
        )
        .is_ok()
    );
    assert!(
        validate_recovery_request(&request, "bootstrap", &correlation, Some("instance-1")).is_err()
    );
    if let Some(value) = request
        .as_object_mut()
        .and_then(|object| object.get_mut("correlation_id"))
    {
        *value = JsonValue::string("66666666-6666-4666-8666-666666666667");
    }
    assert!(validate_recovery_request(&request, "bootstrap", &correlation, None).is_err());

    let response = parse_json(BOOTSTRAP_RESPONSE)?;
    let response_correlation = response
        .as_object()
        .and_then(|object| object.get("correlation_id"))
        .and_then(JsonValue::as_string)
        .ok_or_else(|| String::from("bootstrap response correlation is missing"))?;
    assert!(validate_recovery_response(&response, "bootstrap", response_correlation).is_ok());
    assert!(
        validate_recovery_response(
            &response,
            "bootstrap",
            "66666666-6666-4666-8666-666666666667"
        )
        .is_err()
    );
    Ok(())
}

#[test]
fn executable_recovery_profile_round_trips_a_real_subprocess_and_loopback_http_peer()
-> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let address = listener.local_addr()?;
    let peer = std::thread::spawn(move || -> Result<HttpRequest, String> {
        let (mut stream, _) = listener.accept().map_err(|error| error.to_string())?;
        let (request_line, headers, body) = read_http_request(&mut stream)?;
        let request = parse_json(std::str::from_utf8(&body).map_err(|error| error.to_string())?)?;
        let correlation = request
            .as_object()
            .and_then(|object| object.get("correlation_id"))
            .and_then(JsonValue::as_string)
            .ok_or_else(|| String::from("subprocess request has no correlation"))?;
        let mut response = parse_json(BOOTSTRAP_RESPONSE)?;
        response
            .as_object_mut()
            .ok_or_else(|| String::from("bootstrap response is not an object"))?
            .insert("correlation_id".to_owned(), JsonValue::string(correlation));
        let bytes = response.to_json().into_bytes();
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
            bytes.len()
        )
        .map_err(|error| error.to_string())?;
        stream
            .write_all(&bytes)
            .map_err(|error| error.to_string())?;
        Ok((request_line, headers, body))
    });

    let mut child = Command::new(env!("CARGO_BIN_EXE_sts2-mcp-server"))
        .env_clear()
        .envs(std::env::var_os("SystemRoot").map(|root| ("SystemRoot", root)))
        .env("STS2_RUNTIME_PROFILE", "watchdog-recovery-v1")
        .env("STS2_RECOVERY_TOKEN", "recovery-token")
        .env("STS2_RECOVERY_PROOF", "recovery-proof")
        .env("STS2_GATEWAY_ADDR", address.to_string())
        .env("STS2_INSTANCE_ID", "22222222-2222-4222-8222-222222222222")
        .env("STS2_SESSION_ID", "gateway-session")
        .env("STS2_MCP_SESSION_ID", "mcp-subprocess")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    let mut stdin = child.stdin.take().ok_or("subprocess stdin unavailable")?;
    let payload = fixture_payload(BOOTSTRAP_REQUEST)?;
    writeln!(
        stdin,
        "{}",
        call("watchdog.bootstrap", &payload, "mcp-subprocess")
    )?;
    drop(stdin);
    let output = child.wait_with_output()?;
    let (request_line, headers, body) = peer.join().map_err(|_| "HTTP peer panicked")??;
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("\"isError\":false"), "{stdout}");
    assert!(stdout.contains("BOOT_READY"), "{stdout}");
    assert_eq!(request_line, "POST /v1/recovery/bootstrap HTTP/1.1");
    assert_eq!(
        headers.get("authorization").map(String::as_str),
        Some("Bearer recovery-token")
    );
    assert_eq!(
        headers.get("x-mcp-session-id").map(String::as_str),
        Some("mcp-subprocess")
    );
    assert_eq!(
        headers
            .get("x-sts2-recovery-capability")
            .map(String::as_str),
        Some("bootstrap")
    );
    for name in [
        "x-sts2-instance-id",
        "x-sts2-caller-id",
        "x-sts2-session-id",
        "x-sts2-lease-id",
        "x-sts2-lease-epoch",
    ] {
        assert!(
            !headers.contains_key(name),
            "recovery request revived a legacy authority header: {name}"
        );
    }
    let body = parse_json(std::str::from_utf8(&body)?)?;
    let object = body
        .as_object()
        .ok_or("subprocess sent a non-object recovery frame")?;
    assert_eq!(object.len(), 9);
    assert!(object.get("instance_id").is_none());
    assert!(object.get("session_id").is_none());
    assert!(object.get("lease_id").is_none());
    assert!(object.get("lease_epoch").is_none());
    assert_eq!(
        object.get("kind").and_then(JsonValue::as_string),
        Some("bootstrap_request")
    );
    Ok(())
}
