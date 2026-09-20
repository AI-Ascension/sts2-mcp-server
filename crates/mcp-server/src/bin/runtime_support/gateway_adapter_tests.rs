// SPDX-License-Identifier: MIT
#![allow(clippy::unwrap_used)] // Fail immediately if the fixture envelope cannot be encoded.

use super::super::RuntimeConfig;
use super::*;
use std::collections::BTreeMap;
use sts2_mcp_server::{
    Correlation, GatewayMethod, GatewayRequest, JsonValue, LIVE_BOOTSTRAP_PROTOCOL_VERSION,
    LIVE_BOOTSTRAP_SCHEMA_DIGEST, RequestId,
};

/// The four members `inject_profile_identity` adds to runtime-v1 bodies.
const LEGACY_IDENTITY: [&str; 4] = ["instance_id", "session_id", "lease_id", "lease_epoch"];

fn bootstrap_test_config() -> RuntimeConfig {
    RuntimeConfig {
        gateway_address: std::net::SocketAddr::from(([127, 0, 0, 1], 15525)),
        gateway_token: String::from("token"),
        instance_id: String::from("instance"),
        caller_id: String::from("caller"),
        session_id: String::from("session"),
        mcp_session_id: String::from("mcp-session"),
        lease_id: String::from("lease"),
        lease_epoch: 1,
        recovery_token: None,
        exact_restore_profile: false,
        recovery_profile: false,
        coop_native_peer_binding: None,
    }
}

/// A sealed live-observation bootstrap envelope, in the shape the tool adapter builds.
///
/// `overrides` replaces or adds members, so a test can express exactly one deviation from the
/// sealed shape and show that the guard refuses it.
fn bootstrap_envelope(overrides: impl IntoIterator<Item = (&'static str, JsonValue)>) -> JsonValue {
    let mut members: BTreeMap<String, JsonValue> = [
        (
            "protocol_version".to_owned(),
            JsonValue::string(LIVE_BOOTSTRAP_PROTOCOL_VERSION),
        ),
        (
            "schema_digest".to_owned(),
            JsonValue::string(LIVE_BOOTSTRAP_SCHEMA_DIGEST),
        ),
        ("kind".to_owned(), JsonValue::string("bootstrap_request")),
        ("correlation_id".to_owned(), JsonValue::string("4")),
        ("selector".to_owned(), JsonValue::Null),
        ("scope".to_owned(), JsonValue::Null),
        ("limits".to_owned(), JsonValue::Null),
        ("parent_observation".to_owned(), JsonValue::Null),
        ("visible_entities".to_owned(), JsonValue::Null),
        ("owner_provenance".to_owned(), JsonValue::Null),
        ("error".to_owned(), JsonValue::Null),
        (
            "provenance".to_owned(),
            JsonValue::object([("artifact".to_owned(), JsonValue::string("sts2-protocol/x"))]),
        ),
    ]
    .into_iter()
    .collect();
    for (field, value) in overrides {
        members.insert(field.to_owned(), value);
    }
    JsonValue::Object(members)
}

fn bootstrap_request() -> GatewayRequest {
    GatewayRequest {
        method: GatewayMethod::Post,
        path: String::from("/v1/instances/instance/game-information/live-observation-bootstrap"),
        headers: std::collections::BTreeMap::new(),
        body: Some(bootstrap_envelope([])),
        correlation: Correlation {
            mcp_session_id: String::from("mcp-session"),
            mcp_request_id: RequestId::Number(4),
        },
    }
}

/// The wire body of the sealed bootstrap envelope must reach the gateway byte-identical.
///
/// This is the regression test for the defect the pinned-peer harness observed as
/// `ProviderUnavailable` with no bootstrap request in the downstream trace: `body()` did not
/// recognize this route, so it fell through to `inject_profile_identity` and added the runtime-v1
/// transport identity. The envelope schema sets `additionalProperties: false`, so the gateway
/// rejected every bootstrap as schema-invalid before contacting the producer.
#[test]
fn sealed_bootstrap_body_is_not_rewritten_with_legacy_identity() {
    let adapter = RuntimeGatewayAdapter::new(bootstrap_test_config(), 64 * 1024);
    let request = bootstrap_request();
    let encoded = adapter.body(&request).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&encoded).unwrap();
    let object = value.as_object().unwrap();

    for field in LEGACY_IDENTITY {
        assert!(
            !object.contains_key(field),
            "adapter injected legacy identity member {field} into the sealed bootstrap envelope"
        );
    }
    let expected: serde_json::Value =
        serde_json::from_str(&bootstrap_envelope([]).to_json()).unwrap();
    assert_eq!(
        value, expected,
        "adapter must forward the sealed envelope unchanged"
    );
}

/// The guard still refuses a body that is not the sealed bootstrap envelope.
#[test]
fn bootstrap_body_guard_rejects_a_foreign_or_identity_bearing_body() {
    let foreign = bootstrap_envelope([("kind", JsonValue::string("query_request"))]);
    assert!(negotiated::validate_live_bootstrap_body(foreign.as_object().unwrap()).is_err());

    let bearing = bootstrap_envelope([("lease_id", JsonValue::string("lease-1"))]);
    assert!(negotiated::validate_live_bootstrap_body(bearing.as_object().unwrap()).is_err());

    let sealed = bootstrap_envelope([]);
    assert!(negotiated::validate_live_bootstrap_body(sealed.as_object().unwrap()).is_ok());
}
