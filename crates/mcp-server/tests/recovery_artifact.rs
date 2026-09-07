// SPDX-License-Identifier: MIT

use std::io::Write;
use std::net::TcpListener;
use std::process::{Command, Stdio};

use jsonschema::{PatternOptions, draft202012::options};
use serde_json::Value;
use sts2_mcp_server::{
    JsonValue, RECOVERY_ARTIFACT, RECOVERY_PROTOCOL_VERSION, RECOVERY_SCHEMA_DIGEST, parse_json,
    validate_recovery_request, verify_recovery_artifact,
};

#[path = "support/recovery_mcp_support.rs"]
mod support;
use support::{HttpRequest, read_http_request};

const MANIFEST: &str =
    include_str!("../../../protocol-artifact/watchdog-recovery-v1/manifest.json");
const SCHEMA: &[u8] = include_bytes!("../../../protocol-artifact/watchdog-recovery-v1/schema.json");
const CONFORMANCE: &str =
    include_str!("../../../protocol-artifact/watchdog-recovery-v1/conformance.json");
const RCJ_VECTORS: &str =
    include_str!("../../../protocol-artifact/watchdog-recovery-v1/rcj-vectors.json");
const BOOTSTRAP_REQUEST: &str = include_str!(
    "../../../protocol-artifact/watchdog-recovery-v1/fixtures/valid/bootstrap-request.json"
);
const BOOTSTRAP_RESPONSE: &str = include_str!(
    "../../../protocol-artifact/watchdog-recovery-v1/fixtures/valid/bootstrap-response.json"
);
const VALID_FIXTURES: [(&str, &str); 18] = [
    (
        "bootstrap-request",
        include_str!(
            "../../../protocol-artifact/watchdog-recovery-v1/fixtures/valid/bootstrap-request.json"
        ),
    ),
    (
        "bootstrap-response",
        include_str!(
            "../../../protocol-artifact/watchdog-recovery-v1/fixtures/valid/bootstrap-response.json"
        ),
    ),
    (
        "host-fence-request",
        include_str!(
            "../../../protocol-artifact/watchdog-recovery-v1/fixtures/valid/host-fence-request.json"
        ),
    ),
    (
        "host-fence-response",
        include_str!(
            "../../../protocol-artifact/watchdog-recovery-v1/fixtures/valid/host-fence-response.json"
        ),
    ),
    (
        "lease-acquire-request",
        include_str!(
            "../../../protocol-artifact/watchdog-recovery-v1/fixtures/valid/lease-acquire-request.json"
        ),
    ),
    (
        "lease-acquire-response",
        include_str!(
            "../../../protocol-artifact/watchdog-recovery-v1/fixtures/valid/lease-acquire-response.json"
        ),
    ),
    (
        "lease-renew-request",
        include_str!(
            "../../../protocol-artifact/watchdog-recovery-v1/fixtures/valid/lease-renew-request.json"
        ),
    ),
    (
        "lease-renew-response",
        include_str!(
            "../../../protocol-artifact/watchdog-recovery-v1/fixtures/valid/lease-renew-response.json"
        ),
    ),
    (
        "lease-revoke-request",
        include_str!(
            "../../../protocol-artifact/watchdog-recovery-v1/fixtures/valid/lease-revoke-request.json"
        ),
    ),
    (
        "lease-revoke-response",
        include_str!(
            "../../../protocol-artifact/watchdog-recovery-v1/fixtures/valid/lease-revoke-response.json"
        ),
    ),
    (
        "operation-dispatch-request",
        include_str!(
            "../../../protocol-artifact/watchdog-recovery-v1/fixtures/valid/operation-dispatch-request.json"
        ),
    ),
    (
        "operation-dispatch-response",
        include_str!(
            "../../../protocol-artifact/watchdog-recovery-v1/fixtures/valid/operation-dispatch-response.json"
        ),
    ),
    (
        "operation-intent-request",
        include_str!(
            "../../../protocol-artifact/watchdog-recovery-v1/fixtures/valid/operation-intent-request.json"
        ),
    ),
    (
        "operation-intent-response",
        include_str!(
            "../../../protocol-artifact/watchdog-recovery-v1/fixtures/valid/operation-intent-response.json"
        ),
    ),
    (
        "operation-lookup-request",
        include_str!(
            "../../../protocol-artifact/watchdog-recovery-v1/fixtures/valid/operation-lookup-request.json"
        ),
    ),
    (
        "operation-lookup-response",
        include_str!(
            "../../../protocol-artifact/watchdog-recovery-v1/fixtures/valid/operation-lookup-response.json"
        ),
    ),
    (
        "operation-reconcile-request",
        include_str!(
            "../../../protocol-artifact/watchdog-recovery-v1/fixtures/valid/operation-reconcile-request.json"
        ),
    ),
    (
        "operation-reconcile-response",
        include_str!(
            "../../../protocol-artifact/watchdog-recovery-v1/fixtures/valid/operation-reconcile-response.json"
        ),
    ),
];
const INVALID_FIXTURES: [(&str, &str); 3] = [
    (
        "oversized-action",
        include_str!(
            "../../../protocol-artifact/watchdog-recovery-v1/fixtures/invalid/oversized-action.json"
        ),
    ),
    (
        "stale-contract",
        include_str!(
            "../../../protocol-artifact/watchdog-recovery-v1/fixtures/invalid/stale-contract.json"
        ),
    ),
    (
        "unknown-field",
        include_str!(
            "../../../protocol-artifact/watchdog-recovery-v1/fixtures/invalid/unknown-field.json"
        ),
    ),
];

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

fn bootstrap_payload_with_policy(ttl: i64, renewal: i64) -> Result<JsonValue, String> {
    let mut payload = fixture_payload(BOOTSTRAP_REQUEST)?;
    let policy = payload
        .as_object_mut()
        .and_then(|object| object.get_mut("lease_policy"))
        .and_then(JsonValue::as_object_mut)
        .ok_or_else(|| String::from("bootstrap fixture is missing lease policy"))?;
    policy.insert("ttl_seconds".to_owned(), JsonValue::Number(ttl));
    policy.insert(
        "renewal_interval_seconds".to_owned(),
        JsonValue::Number(renewal),
    );
    Ok(payload)
}

#[test]
fn recovery_profile_consumes_the_exact_published_artifact() -> Result<(), String> {
    verify_recovery_artifact().map_err(|error| error.to_string())?;
    let manifest: Value = serde_json::from_str(MANIFEST).map_err(|error| error.to_string())?;
    assert_eq!(manifest["artifact"], RECOVERY_ARTIFACT);
    assert_eq!(manifest["protocol_version"], RECOVERY_PROTOCOL_VERSION);
    assert_eq!(manifest["schema_digest"], RECOVERY_SCHEMA_DIGEST);
    assert_eq!(manifest["schema"], "schema.json");
    assert!(SCHEMA.starts_with(b"{\n  \"$schema\""));
    serde_json::from_str::<Value>(CONFORMANCE).map_err(|error| error.to_string())?;
    serde_json::from_str::<Value>(RCJ_VECTORS).map_err(|error| error.to_string())?;
    Ok(())
}

#[test]
fn recovery_schema_accepts_all_valid_frames_and_rejects_invalid_shapes() -> Result<(), String> {
    let schema: Value = serde_json::from_slice(SCHEMA).map_err(|error| error.to_string())?;
    let validator = options()
        .with_pattern_options(PatternOptions::fancy_regex().size_limit(1_000_000_000))
        .build(&schema)
        .map_err(|error| error.to_string())?;
    for (name, fixture) in VALID_FIXTURES {
        let value: Value = serde_json::from_str(fixture).map_err(|error| error.to_string())?;
        if !validator.is_valid(&value) {
            return Err(format!("valid recovery fixture was rejected: {name}"));
        }
    }
    for (name, fixture) in INVALID_FIXTURES {
        let value: Value = serde_json::from_str(fixture).map_err(|error| error.to_string())?;
        if validator.is_valid(&value) {
            return Err(format!("invalid recovery fixture was accepted: {name}"));
        }
    }
    Ok(())
}

#[test]
fn semantic_conformance_rejects_lease_renewal_at_or_above_ttl() -> Result<(), String> {
    for (ttl, renewal) in [(30, 30), (30, 31)] {
        let mut frame = parse_json(BOOTSTRAP_REQUEST)?;
        let correlation = frame
            .as_object()
            .and_then(|object| object.get("correlation_id"))
            .and_then(JsonValue::as_string)
            .ok_or_else(|| String::from("bootstrap correlation is missing"))?
            .to_owned();
        let policy = frame
            .as_object_mut()
            .and_then(|object| object.get_mut("payload"))
            .and_then(JsonValue::as_object_mut)
            .and_then(|payload| payload.get_mut("lease_policy"))
            .and_then(JsonValue::as_object_mut)
            .ok_or_else(|| String::from("bootstrap lease policy is missing"))?;
        policy.insert("ttl_seconds".to_owned(), JsonValue::Number(ttl));
        policy.insert(
            "renewal_interval_seconds".to_owned(),
            JsonValue::Number(renewal),
        );
        assert!(
            validate_recovery_request(&frame, "bootstrap", &correlation, None).is_err(),
            "{ttl}/{renewal} was accepted by the semantic conformance validator"
        );
    }
    Ok(())
}

#[test]
fn executable_rejects_lease_renewal_at_or_above_ttl_before_gateway()
-> Result<(), Box<dyn std::error::Error>> {
    for (ttl, renewal) in [(30, 30), (30, 31)] {
        let mut child = Command::new(env!("CARGO_BIN_EXE_sts2-mcp-server"))
            .env_clear()
            .envs(std::env::var_os("SystemRoot").map(|root| ("SystemRoot", root)))
            .env("STS2_RUNTIME_PROFILE", "watchdog-recovery-v1")
            .env("STS2_RECOVERY_TOKEN", "recovery-token")
            .env("STS2_RECOVERY_PROOF", "recovery-proof")
            .env("STS2_GATEWAY_ADDR", "127.0.0.1:1")
            .env("STS2_INSTANCE_ID", "22222222-2222-4222-8222-222222222222")
            .env("STS2_SESSION_ID", "gateway-session")
            .env("STS2_MCP_SESSION_ID", "mcp-subprocess")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;
        let mut stdin = child.stdin.take().ok_or("subprocess stdin unavailable")?;
        let payload = bootstrap_payload_with_policy(ttl, renewal)?;
        writeln!(
            stdin,
            "{}",
            call("watchdog.bootstrap", &payload, "mcp-subprocess")
        )?;
        drop(stdin);
        let output = child.wait_with_output()?;
        assert!(output.status.success());
        let stdout = String::from_utf8(output.stdout)?;
        assert!(
            stdout.contains("\"code\":-32602"),
            "{ttl}/{renewal}: {stdout}"
        );
        assert!(
            stdout.contains("recovery lease policy is outside bounds"),
            "{ttl}/{renewal}: {stdout}"
        );
        assert!(!stdout.contains("\"status\":\"UNKNOWN\""), "{stdout}");
    }
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
