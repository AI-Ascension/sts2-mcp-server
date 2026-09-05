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
        let mut adapter = super::super::RuntimeGatewayAdapter::new(config);
        let result = adapter.forward(request);
        if session == "session" {
            assert!(result.is_ok());
        } else {
            assert_eq!(result, Err(GatewayError::MalformedResponse));
        }
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
    let mut adapter = super::super::RuntimeGatewayAdapter::new(config());
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
    let mut adapter = super::super::RuntimeGatewayAdapter::new(config);
    assert_eq!(adapter.forward(request()), Err(GatewayError::Forbidden));
    worker.join().unwrap();
}
