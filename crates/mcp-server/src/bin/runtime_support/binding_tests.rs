// SPDX-License-Identifier: MIT
#![allow(clippy::unwrap_used)] // Fail immediately if disposable socket test setup fails.

use super::*;
use std::collections::BTreeMap;
use std::net::SocketAddr;
use sts2_mcp_server::{Correlation, GatewayAdapter, RequestId};

fn config() -> RuntimeConfig {
    RuntimeConfig {
        gateway_address: SocketAddr::from(([127, 0, 0, 1], 15525)),
        gateway_token: String::from("token"),
        instance_id: String::from("instance"),
        caller_id: String::from("caller"),
        session_id: String::from("session"),
        mcp_session_id: String::from("mcp-session"),
        lease_id: String::from("lease"),
        lease_epoch: 1,
    }
}

#[test]
fn executable_rejects_foreign_v1_response_before_returning_success() {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;
    use std::time::Duration;

    for session in ["session", "foreign-session"] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let mut config = config();
        config.gateway_address = listener.local_addr().unwrap();
        let worker = thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(1)))
                .unwrap();
            let mut bytes = [0; 8192];
            assert!(socket.read(&mut bytes).unwrap() > 0);
            let body = format!(
                "{{\"instance_id\":\"instance\",\"session_id\":\"{session}\",\"lease_id\":\"lease\",\"lease_epoch\":1,\"correlation_id\":\"request\",\"kind\":\"state_response\"}}"
            );
            socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}", body.len()).as_bytes()).unwrap();
        });
        let mut request = request();
        request.path = String::from("/v1/instances/instance/state");
        let mut adapter = super::super::RuntimeGatewayAdapter::new(
            config,
            super::super::http::LEGACY_MAX_RESPONSE_BYTES,
        );
        let result = adapter.forward(request);
        if session == "session" {
            assert!(result.is_ok());
        } else {
            assert_eq!(result, Err(GatewayError::MalformedResponse));
        }
        worker.join().unwrap();
    }
}

#[test]
fn semantic_http_uncertainty_retains_the_received_error_origin() {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;
    use std::time::Duration;
    for status in [408, 502, 503, 504] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let mut config = config();
        config.gateway_address = listener.local_addr().unwrap();
        let worker = thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(1)))
                .unwrap();
            let mut bytes = [0; 8192];
            assert!(socket.read(&mut bytes).unwrap() > 0);
            let body = r#"{"protocol_version":"runtime-v3-gameplay","kind":"dispatch_action_response","status":"unknown","error_code":"sts2.game-mod/settlement_unproven"}"#;
            socket.write_all(format!("HTTP/1.1 {status} Unknown\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}", body.len()).as_bytes()).unwrap();
        });
        let mut request = request();
        request.path = String::from("/v3/instances/instance/action");
        request.method = GatewayMethod::Post;
        let mut adapter = super::super::RuntimeGatewayAdapter::new(
            config,
            super::super::http::LEGACY_MAX_RESPONSE_BYTES,
        );
        let result = adapter.forward(request).unwrap();
        assert_eq!(result.status, status);
        assert!(result.body.to_json().contains("settlement_unproven"));
        // Full-envelope validation belongs to semantic projection; this checks only HTTP classification.
        worker.join().unwrap();
    }
}

fn request() -> GatewayRequest {
    GatewayRequest {
        method: GatewayMethod::Get,
        path: String::from("/v2/instances/instance/operations/operation"),
        headers: BTreeMap::from([
            (
                String::from("x-mcp-session-id"),
                String::from("mcp-session"),
            ),
            (String::from("x-sts2-instance-id"), String::from("instance")),
            (String::from("x-sts2-session-id"), String::from("session")),
            (String::from("x-sts2-lease-id"), String::from("lease")),
            (String::from("x-sts2-lease-epoch"), String::from("1")),
        ]),
        body: None,
        correlation: Correlation {
            mcp_session_id: String::from("mcp-session"),
            mcp_request_id: RequestId::String(String::from("request")),
        },
    }
}

#[test]
fn bodyless_authority_mismatch_is_rejected_before_connect() {
    let mut adapter = super::super::RuntimeGatewayAdapter::new(
        config(),
        super::super::http::LEGACY_MAX_RESPONSE_BYTES,
    );
    assert_eq!(admit(&config(), &request()), Ok(()));
    for name in [
        "x-sts2-instance-id",
        "x-sts2-session-id",
        "x-sts2-lease-id",
        "x-sts2-lease-epoch",
    ] {
        let mut changed = request();
        changed
            .headers
            .insert(String::from(name), String::from("wrong"));
        assert_eq!(adapter.forward(changed), Err(GatewayError::Rejected));
        let mut missing = request();
        missing.headers.remove(name);
        assert_eq!(adapter.forward(missing), Err(GatewayError::Rejected));
    }
    let mut changed = request();
    changed.correlation.mcp_session_id = String::from("foreign-session");
    assert_eq!(adapter.forward(changed), Err(GatewayError::Rejected));
    let mut changed = request();
    changed.headers.remove("x-mcp-session-id");
    assert_eq!(adapter.forward(changed), Err(GatewayError::Rejected));
    let mut changed = request();
    changed
        .headers
        .insert(String::from("x-mcp-session-id"), String::from("foreign"));
    assert_eq!(adapter.forward(changed), Err(GatewayError::Rejected));
    let mut changed = request();
    changed.path = String::from("/v2/instances/foreign/operations/operation");
    assert_eq!(adapter.forward(changed), Err(GatewayError::Rejected));
}

#[test]
fn v4_reconcile_rejects_stale_lease_and_epoch_before_http() {
    use std::io::ErrorKind;
    use std::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let mut config = config();
    config.gateway_address = listener.local_addr().unwrap();
    let expected_lease_id = config.lease_id.clone();
    let mut adapter = super::super::RuntimeGatewayAdapter::new(
        config,
        super::super::http::LEGACY_MAX_RESPONSE_BYTES,
    );
    let mut reconcile = request();
    reconcile.path = String::from("/v4/instances/instance/expert-actions/operation");

    reconcile
        .headers
        .insert(String::from("x-sts2-lease-id"), String::from("stale-lease"));
    assert_eq!(
        adapter.forward(reconcile.clone()),
        Err(GatewayError::Rejected)
    );
    assert!(matches!(listener.accept(), Err(error) if error.kind() == ErrorKind::WouldBlock));

    reconcile
        .headers
        .insert(String::from("x-sts2-lease-id"), expected_lease_id);
    reconcile
        .headers
        .insert(String::from("x-sts2-lease-epoch"), String::from("0"));
    assert_eq!(adapter.forward(reconcile), Err(GatewayError::Rejected));
    assert!(matches!(listener.accept(), Err(error) if error.kind() == ErrorKind::WouldBlock));
}

#[test]
fn v4_rest_reconcile_route_reaches_the_loopback_gateway() {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;
    use std::time::Duration;

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let mut config = config();
    config.gateway_address = listener.local_addr().unwrap();
    let worker = thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(1)))
            .unwrap();
        let mut bytes = [0; 8192];
        assert!(socket.read(&mut bytes).unwrap() > 0);
        let body = r#"{"protocol_version":"runtime-v4-expert-rest-action-v1","kind":"action_response","instance_id":"instance","session_id":"session","lease_id":"lease","lease_epoch":1,"correlation_id":"request","operation_id":"operation","status":"unknown","action":{"action_id":"rest-option:7:heal","action":{"kind":"rest_option","rest_option_id":"heal"}},"generation":7,"state_id":"live:7","observation":null,"transition":null,"effect_witness":null,"error_code":"sts2.runtime/unknown"}"#;
        socket
            .write_all(format!("HTTP/1.1 503 Unknown\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}", body.len()).as_bytes())
            .unwrap();
    });
    let mut request = request();
    request.path = String::from("/v4/instances/instance/expert-rest-actions/operation");
    let mut adapter = super::super::RuntimeGatewayAdapter::new(
        config,
        super::super::http::LEGACY_MAX_RESPONSE_BYTES,
    );
    let response = adapter.forward(request).unwrap();
    assert_eq!(response.status, 503);
    assert!(response.body.to_json().contains("unknown"));
    worker.join().unwrap();
}

#[test]
fn response_is_bound_to_configured_identity_request_and_route() {
    let fields = BTreeMap::from([
        (String::from("instance_id"), JsonValue::string("instance")),
        (String::from("session_id"), JsonValue::string("session")),
        (String::from("lease_id"), JsonValue::string("lease")),
        (String::from("lease_epoch"), JsonValue::Number(1)),
        (String::from("correlation_id"), JsonValue::string("request")),
        (String::from("kind"), JsonValue::string("state_response")),
    ]);
    assert_eq!(
        response(
            &config(),
            &JsonValue::Object(fields.clone()),
            "request",
            "state_response"
        ),
        Ok(())
    );
    for name in fields.keys() {
        let mut wrong = fields.clone();
        wrong.insert(name.clone(), JsonValue::string("wrong"));
        assert_eq!(
            response(
                &config(),
                &JsonValue::Object(wrong),
                "request",
                "state_response"
            ),
            Err(GatewayError::MalformedResponse)
        );
    }
    let mut request = request();
    request.path = String::from("/v1/instances/instance/state");
    assert_eq!(response_kind(&config(), &request), Some("state_response"));
    request.path = String::from("/v1/instances/instance/action");
    request.method = GatewayMethod::Post;
    assert_eq!(response_kind(&config(), &request), Some("action_response"));

    request.path = String::from("/v2/instances/instance/seeded-run");
    request.body = None;
    assert_eq!(response_kind(&config(), &request), None);
    request.body = Some(JsonValue::object([]));
    assert_eq!(response_kind(&config(), &request), Some("start_response"));
}

#[test]
fn forbidden_scope_receipt_remains_a_sanitized_authorization_error() {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;
    use std::time::Duration;

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let mut config = config();
    config.gateway_address = listener.local_addr().unwrap();
    let worker = thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(1)))
            .unwrap();
        let mut bytes = [0; 8192];
        assert!(socket.read(&mut bytes).unwrap() > 0);
        let body = r#"{"error_code":"insufficient_scope","private_detail":"do-not-forward"}"#;
        socket.write_all(format!("HTTP/1.1 403 Forbidden\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}", body.len()).as_bytes()).unwrap();
    });
    let mut adapter = super::super::RuntimeGatewayAdapter::new(
        config,
        super::super::http::LEGACY_MAX_RESPONSE_BYTES,
    );
    assert_eq!(adapter.forward(request()), Err(GatewayError::Forbidden));
    worker.join().unwrap();
}

#[test]
fn co_op_bodyless_read_cannot_inject_missing_or_foreign_authority() {
    let mut request = request();
    request.path = "/v1/instances/instance/coop/synchronization".to_owned();
    request.method = GatewayMethod::Get;
    request.body = None;
    for (name, value) in [
        ("x-sts2-instance-id", "instance"),
        ("x-sts2-session-id", "session"),
        ("x-sts2-lease-id", "lease"),
        ("x-sts2-lease-epoch", "1"),
    ] {
        request.headers.insert(name.to_owned(), value.to_owned());
    }
    assert_eq!(admit(&config(), &request), Ok(()));
    assert_eq!(
        response_kind(&config(), &request),
        Some("synchronization_response")
    );
    for name in [
        "x-sts2-instance-id",
        "x-sts2-session-id",
        "x-sts2-lease-id",
        "x-sts2-lease-epoch",
    ] {
        let original = request.headers.remove(name).unwrap();
        assert_eq!(admit(&config(), &request), Err(GatewayError::Rejected));
        request
            .headers
            .insert(name.to_owned(), "foreign".to_owned());
        assert_eq!(admit(&config(), &request), Err(GatewayError::Rejected));
        request.headers.insert(name.to_owned(), original);
    }
    request.body = Some(JsonValue::Null);
    assert_eq!(admit(&config(), &request), Err(GatewayError::Rejected));
    request.body = None;
    request.method = GatewayMethod::Post;
    assert_eq!(admit(&config(), &request), Err(GatewayError::Rejected));
}

#[test]
fn native_routes_are_exactly_allowlisted_and_schema_bound() {
    let mut observation = request();
    observation.path = String::from("/v1/instances/instance/coop/native/observation");
    observation.method = GatewayMethod::Get;
    observation.body = None;
    assert_eq!(admit(&config(), &observation), Ok(()));

    let native_body = JsonValue::object([
        (
            String::from("protocol_version"),
            JsonValue::string(sts2_mcp_server::COOP_NATIVE_PROTOCOL_VERSION),
        ),
        (
            String::from("schema_digest"),
            JsonValue::string(sts2_mcp_server::COOP_NATIVE_SCHEMA_DIGEST),
        ),
        (String::from("instance_id"), JsonValue::string("instance")),
        (String::from("session_id"), JsonValue::string("session")),
        (String::from("lease_id"), JsonValue::string("lease")),
        (String::from("lease_epoch"), JsonValue::Number(1)),
    ]);
    for suffix in ["legal-catalog", "action", "vote", "rejoin", "recover"] {
        let mut native = request();
        native.path = format!("/v1/instances/instance/coop/native/{suffix}");
        native.method = GatewayMethod::Post;
        native.body = Some(native_body.clone());
        assert_eq!(admit(&config(), &native), Ok(()), "{suffix}");
    }

    let mut extra = observation.clone();
    extra.path.push_str("/extra");
    assert_eq!(admit(&config(), &extra), Err(GatewayError::Rejected));
    let mut wrong_schema = request();
    wrong_schema.path = String::from("/v1/instances/instance/coop/native/action");
    wrong_schema.method = GatewayMethod::Post;
    let mut wrong_body = native_body;
    if let JsonValue::Object(object) = &mut wrong_body {
        object.insert(
            String::from("schema_digest"),
            JsonValue::string("0000000000000000000000000000000000000000000000000000000000000000"),
        );
    }
    wrong_schema.body = Some(wrong_body);
    assert_eq!(admit(&config(), &wrong_schema), Err(GatewayError::Rejected));
}

#[test]
fn v4_bodyless_reads_allow_the_http_adapter_to_inject_gateway_authority() {
    let mut request = request();
    request.path = "/v4/instances/instance/expert-state".to_owned();
    for name in [
        "x-sts2-instance-id",
        "x-sts2-session-id",
        "x-sts2-lease-id",
        "x-sts2-lease-epoch",
    ] {
        request.headers.remove(name);
    }
    assert_eq!(admit(&config(), &request), Ok(()));
    assert_eq!(response_kind(&config(), &request), Some("state_response"));
    request.path = "/v4/instances/instance/expert-actions/potion-op-1".to_owned();
    assert_eq!(admit(&config(), &request), Ok(()));
    assert_eq!(response_kind(&config(), &request), Some("action_response"));
}
