// SPDX-License-Identifier: MIT

use super::profiles::profile_for_name;
use super::*;
use std::collections::BTreeMap;
use sts2_mcp_server::{Correlation, GatewayMethod};

fn config() -> RuntimeConfig {
    RuntimeConfig {
        gateway_address: SocketAddr::from(([127, 0, 0, 1], 15525)),
        gateway_token: String::from("token"),
        instance_id: String::from("configured-instance"),
        caller_id: String::from("caller"),
        session_id: String::from("configured-session"),
        mcp_session_id: String::from("configured-session"),
        lease_id: String::from("configured-lease"),
        lease_epoch: 7,
    }
}

#[test]
fn gateway_configuration_rejects_dns_remote_zero_port_and_unsafe_tokens() {
    for value in [
        "localhost:15525",
        "192.0.2.1:15525",
        "0.0.0.0:15525",
        "[::]:15525",
        "127.0.0.1:0",
        "127.0.0.1:80\r\nX: a",
    ] {
        assert!(gateway_address(value).is_err(), "{value:?}");
    }
    assert!(gateway_address("127.0.0.1:15525").is_ok());
    assert!(gateway_address("[::1]:15525").is_ok());
    for value in [
        "",
        "token\0",
        "token\u{7f}",
        "tokené",
        "token word",
        "token\r\n",
    ] {
        assert!(!safe_token(value));
    }
    assert!(safe_token("token-safe_123.=/+"));
}

fn request(body: JsonValue) -> GatewayRequest {
    GatewayRequest {
        method: GatewayMethod::Post,
        path: String::from("/v2/instances/configured-instance/action"),
        headers: BTreeMap::new(),
        body: Some(body),
        correlation: Correlation {
            mcp_session_id: String::from("configured-session"),
            mcp_request_id: sts2_mcp_server::RequestId::String(String::from("request-1")),
        },
    }
}

#[test]
fn runtime_profile_defaults_to_v1_and_selects_v2_explicitly() {
    assert_eq!(
        profile_for_name(None).map(|profile| profile.catalog.revision),
        Ok(String::from("runtime-v1-mcp"))
    );
    assert_eq!(
        profile_for_name(Some("runtime-v2")).map(|profile| profile.catalog.revision),
        Ok(String::from("runtime-v2-mcp"))
    );
    assert!(profile_for_name(Some("runtime-v3")).is_err());
    assert_eq!(
        profile_for_name(Some("runtime-v3-gameplay")).map(|profile| profile.catalog.revision),
        Ok(String::from("runtime-v3-gameplay-mcp"))
    );
}

#[test]
fn runtime_profile_bounds_are_scoped_to_the_semantic_profile() -> Result<(), String> {
    for (name, frame, body) in [
        (None, 16 * 1024, 64 * 1024),
        (Some("runtime-v1"), 16 * 1024, 64 * 1024),
        (Some("runtime-v2"), 16 * 1024, 64 * 1024),
        (Some("runtime-v3-gameplay"), 256 * 1024, 128 * 1024),
    ] {
        let profile = profile_for_name(name)?;
        assert_eq!(profile.catalog.max_frame_bytes(), frame, "{name:?}");
        assert_eq!(profile.max_response_bytes, body, "{name:?}");
    }
    Ok(())
}

#[test]
fn runtime_result_recognition_includes_reconcile_response() {
    for kind in [
        "state_response",
        "action_response",
        "reconcile_response",
        "dispatch_action_response",
        "wait_response",
        "recover_response",
        "legal_actions_response",
        "reobserve_response",
    ] {
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
fn runtime_v2_and_v3_body_reject_wrong_supplied_identity_before_forwarding() {
    let adapter = RuntimeGatewayAdapter::new(config(), http::LEGACY_MAX_RESPONSE_BYTES);
    for protocol in [
        RUNTIME_V2_PROTOCOL_VERSION,
        RUNTIME_V3_GAMEPLAY_PROTOCOL_VERSION,
    ] {
        let body = JsonValue::object([
            (
                String::from("protocol_version"),
                JsonValue::string(protocol),
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
}

#[test]
fn runtime_v1_body_keeps_configured_identity_injection() {
    let adapter = RuntimeGatewayAdapter::new(config(), http::LEGACY_MAX_RESPONSE_BYTES);
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

#[test]
fn mcp_session_default_matches_composition_and_preserves_explicit_override() {
    let select =
        |supplied| configured_value("STS2_MCP_SESSION_ID", supplied, DEFAULT_MCP_SESSION_ID);
    assert_eq!(
        select(Err(std::env::VarError::NotPresent)),
        Ok(String::from("mcp-session-1"))
    );
    for explicit in ["custom-mcp", "session-1"] {
        assert_eq!(
            select(Ok(String::from(explicit))),
            Ok(String::from(explicit))
        );
    }
    assert!(select(Ok(String::new())).is_err());
    assert!(
        select(Err(std::env::VarError::NotUnicode(
            std::ffi::OsString::new()
        )))
        .is_err()
    );
}
