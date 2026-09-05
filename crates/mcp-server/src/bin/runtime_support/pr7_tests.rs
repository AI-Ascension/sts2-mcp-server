// SPDX-License-Identifier: MIT

use super::*;
use std::collections::BTreeMap;
use sts2_mcp_server::Correlation;

fn config() -> RuntimeConfig {
    RuntimeConfig {
        gateway_address: SocketAddr::from(([127, 0, 0, 1], 15525)),
        gateway_token: String::from("token"),
        instance_id: String::from("configured-instance"),
        caller_id: String::from("caller"),
        session_id: String::from("configured-session"),
        mcp_session_id: String::from("configured-mcp-session"),
        lease_id: String::from("configured-lease"),
        lease_epoch: 7,
    }
}

fn request(body: JsonValue) -> GatewayRequest {
    GatewayRequest {
        method: GatewayMethod::Post,
        path: String::from("/v2/instances/configured-instance/action"),
        headers: BTreeMap::new(),
        body: Some(body),
        correlation: Correlation {
            mcp_session_id: String::from("configured-mcp-session"),
            mcp_request_id: sts2_mcp_server::RequestId::String(String::from("request-1")),
        },
    }
}

#[test]
fn runtime_profile_defaults_to_v1_and_selects_v2_explicitly() {
    assert_eq!(
        catalog_for_profile(None).map(|catalog| catalog.revision),
        Ok(String::from("runtime-v1-mcp"))
    );
    assert_eq!(
        catalog_for_profile(Some("runtime-v2")).map(|catalog| catalog.revision),
        Ok(String::from("runtime-v2-mcp"))
    );
    assert_eq!(
        catalog_for_profile(Some("runtime-v3-gameplay")).map(|catalog| catalog.revision),
        Ok(String::from("runtime-v3-gameplay-mcp"))
    );
    assert!(catalog_for_profile(Some("runtime-v4")).is_err());
}

#[test]
fn runtime_result_recognition_includes_reconcile_response() {
    for kind in ["state_response", "action_response", "reconcile_response"] {
        assert!(is_runtime_result(&JsonValue::object([(
            String::from("kind"),
            JsonValue::string(kind),
        )])));
    }
    assert!(!is_runtime_result(&JsonValue::object([(
        String::from("kind"),
        JsonValue::string("reconcile_request"),
    )])));
}

#[test]
fn forbidden_gateway_response_maps_to_typed_scope_error() {
    let response = classify_gateway_response(
        403,
        JsonValue::object([
            (
                String::from("error_code"),
                JsonValue::string("insufficient_scope"),
            ),
            (
                String::from("private_detail"),
                JsonValue::string("do-not-forward"),
            ),
        ]),
    );
    assert_eq!(response, Err(GatewayError::Forbidden));
}

#[test]
fn runtime_v2_body_rejects_wrong_supplied_identity_before_forwarding() {
    let adapter = RuntimeGatewayAdapter::new(config());
    let body = JsonValue::object([
        (
            String::from("protocol_version"),
            JsonValue::string(RUNTIME_V2_PROTOCOL_VERSION),
        ),
        (
            String::from("instance_id"),
            JsonValue::string("wrong-instance"),
        ),
        (
            String::from("session_id"),
            JsonValue::string("configured-session"),
        ),
        (
            String::from("lease_id"),
            JsonValue::string("configured-lease"),
        ),
        (String::from("lease_epoch"), JsonValue::Number(7)),
    ]);
    assert_eq!(adapter.body(&request(body)), Err(GatewayError::Rejected));
}

#[test]
fn runtime_v1_body_keeps_configured_identity_injection() {
    let adapter = RuntimeGatewayAdapter::new(config());
    let body = JsonValue::object([
        (
            String::from("protocol_version"),
            JsonValue::string("runtime-v1"),
        ),
        (
            String::from("instance_id"),
            JsonValue::string("wrong-instance"),
        ),
    ]);
    let encoded = adapter.body(&request(body));
    assert!(encoded.is_ok());
    let encoded = encoded.unwrap_or_default();
    assert!(String::from_utf8_lossy(&encoded).contains("configured-instance"));
    assert!(!String::from_utf8_lossy(&encoded).contains("wrong-instance"));
}
