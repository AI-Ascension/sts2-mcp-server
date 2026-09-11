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
        coop_native_peer_binding: None,
    }
}

fn native_config() -> RuntimeConfig {
    let mut config = config();
    config.coop_native_peer_binding = Some(super::super::CoopNativePeerBinding {
        token: String::from("private-route-token"),
        peer_id: String::from("peer:local-1"),
    });
    config
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

fn native_action_request(actor_peer: &str) -> GatewayRequest {
    let mut request = request();
    request.method = GatewayMethod::Post;
    request.path = String::from("/v1/instances/instance/coop/native/action");
    request.body = Some(JsonValue::object([
        (
            String::from("protocol_version"),
            JsonValue::string(COOP_NATIVE_PROTOCOL_VERSION),
        ),
        (
            String::from("schema_digest"),
            JsonValue::string(sts2_mcp_server::COOP_NATIVE_SCHEMA_DIGEST),
        ),
        (String::from("actor_peer"), JsonValue::string(actor_peer)),
    ]));
    request
}

#[test]
fn native_routes_are_exactly_allowlisted_and_schema_bound() {
    let mut observation = request();
    observation.path = String::from("/v1/instances/instance/coop/native/observation");
    observation.method = GatewayMethod::Get;
    observation.body = None;
    assert_eq!(admit(&config(), &observation), Err(GatewayError::Rejected));
    assert_eq!(admit(&native_config(), &observation), Ok(()));

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
        (
            String::from("actor_peer"),
            JsonValue::string("peer:local-1"),
        ),
    ]);
    for suffix in ["legal-catalog", "action", "vote", "rejoin", "recover"] {
        let mut native = request();
        native.path = format!("/v1/instances/instance/coop/native/{suffix}");
        native.method = GatewayMethod::Post;
        let mut body = native_body.clone();
        if suffix == "recover"
            && let JsonValue::Object(object) = &mut body
        {
            object.insert(String::from("actor_peer"), JsonValue::Null);
        }
        native.body = Some(body);
        assert_eq!(admit(&native_config(), &native), Ok(()), "{suffix}");
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

    let native_body = JsonValue::object([
        (
            String::from("protocol_version"),
            JsonValue::string(sts2_mcp_server::COOP_NATIVE_PROTOCOL_VERSION),
        ),
        (
            String::from("schema_digest"),
            JsonValue::string(sts2_mcp_server::COOP_NATIVE_SCHEMA_DIGEST),
        ),
    ]);
    let mut action = observation;
    action.path = String::from("/v1/instances/instance/coop/native/action");
    action.method = GatewayMethod::Post;
    action.body = Some(native_body.clone());
    assert_eq!(response_kind(&config(), &action), Some("effect_response"));
    action.path = String::from("/v1/instances/instance/coop/native/recover");
    assert_eq!(response_kind(&config(), &action), Some("recovery_response"));
}

#[test]
fn native_peer_binding_is_private_header_transport_and_fences_actor_identity() {
    use std::io::ErrorKind;
    use std::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let mut without_binding = config();
    without_binding.gateway_address = listener.local_addr().unwrap();
    let request = native_action_request("peer:local-1");
    let mut adapter = super::super::RuntimeGatewayAdapter::new(
        without_binding,
        super::super::http::LEGACY_MAX_RESPONSE_BYTES,
    );
    assert_eq!(adapter.forward(request), Err(GatewayError::Rejected));
    assert!(matches!(listener.accept(), Err(error) if error.kind() == ErrorKind::WouldBlock));

    let configured = native_config();
    let request = native_action_request("peer:local-1");
    assert_eq!(admit(&configured, &request), Ok(()));
    let attached = attach_native_peer_token(&configured, request).unwrap();
    assert_eq!(
        attached.headers.get("x-sts2-peer-token"),
        Some(&String::from("private-route-token"))
    );
    assert!(
        !attached
            .body
            .as_ref()
            .unwrap()
            .to_json()
            .contains("private-route-token")
    );
    for (method, route, body) in [
        (GatewayMethod::Get, "observation", None),
        (GatewayMethod::Post, "legal-catalog", Some("peer:local-1")),
        (GatewayMethod::Post, "action", Some("peer:local-1")),
        (GatewayMethod::Post, "vote", Some("peer:local-1")),
        (GatewayMethod::Post, "rejoin", Some("peer:local-1")),
        (GatewayMethod::Post, "recover", None),
    ] {
        let mut request = native_action_request("peer:local-1");
        request.method = method;
        request.path = format!("/v1/instances/instance/coop/native/{route}");
        if route == "recover" {
            if let Some(JsonValue::Object(object)) = &mut request.body {
                object.insert(String::from("actor_peer"), JsonValue::Null);
            }
        } else if body.is_none() {
            request.body = None;
        }
        assert_eq!(admit(&configured, &request), Ok(()), "{route}");
        assert_eq!(
            attach_native_peer_token(&configured, request)
                .unwrap()
                .headers
                .get("x-sts2-peer-token"),
            Some(&String::from("private-route-token")),
            "{route}"
        );
    }

    let secret_as_actor = native_action_request("private-route-token");
    assert_eq!(
        admit(&configured, &secret_as_actor),
        Err(GatewayError::Rejected)
    );
    let foreign_actor = native_action_request("peer:foreign-1");
    assert_eq!(
        admit(&configured, &foreign_actor),
        Err(GatewayError::Rejected)
    );
    let mut recover_with_actor = native_action_request("peer:local-1");
    recover_with_actor.path = String::from("/v1/instances/instance/coop/native/recover");
    assert_eq!(
        admit(&configured, &recover_with_actor),
        Err(GatewayError::Rejected)
    );
    let mut recover_without_actor = recover_with_actor;
    if let Some(JsonValue::Object(object)) = &mut recover_without_actor.body {
        object.remove("actor_peer");
    }
    assert_eq!(
        admit(&configured, &recover_without_actor),
        Err(GatewayError::Rejected)
    );
    for header_name in ["x-sts2-peer-token", "X-Sts2-Peer-Token"] {
        let mut supplied_header = native_action_request("peer:local-1");
        supplied_header.headers.insert(
            String::from(header_name),
            String::from("private-route-token"),
        );
        assert_eq!(
            admit(&configured, &supplied_header),
            Err(GatewayError::Rejected),
            "{header_name}"
        );
    }
}

#[test]
fn native_peer_binding_configuration_requires_a_distinct_canonical_peer() {
    assert!(
        super::super::coop_native_config::from_values(false, None, None)
            .unwrap()
            .is_none()
    );
    assert!(super::super::coop_native_config::from_values(true, None, None).is_err());
    assert!(super::super::coop_native_config::from_values(false, Some("token"), None).is_err());
    assert!(
        super::super::coop_native_config::from_values(false, None, Some("peer:local-1")).is_err()
    );
    assert!(
        super::super::coop_native_config::from_values(false, Some("token"), Some("local-1"))
            .is_err()
    );
    assert!(
        super::super::coop_native_config::from_values(
            false,
            Some("peer:local-1"),
            Some("peer:local-1")
        )
        .is_err()
    );
    let peer = format!("peer:{}", "a".repeat(507));
    let binding =
        super::super::coop_native_config::from_values(false, Some("private-token"), Some(&peer))
            .unwrap()
            .unwrap();
    assert_eq!(binding.peer_id, peer);
    assert_eq!(binding.token, "private-token");
}
