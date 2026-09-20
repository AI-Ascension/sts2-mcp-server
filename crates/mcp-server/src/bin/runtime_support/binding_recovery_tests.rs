// SPDX-License-Identifier: MIT
#![allow(clippy::unwrap_used)] // Fail immediately if the fixture frame cannot be built.

use super::super::RuntimeConfig;
use super::*;
use std::collections::BTreeMap;
use sts2_mcp_server::{
    Correlation, GatewayRequest, JsonValue, RequestId, build_recovery_request, uuid_v4,
};

const CALLER: &str = "00000000-0000-4000-8000-00000000000a";
const MCP_SESSION: &str = "mcp-session-1";

fn config() -> RuntimeConfig {
    RuntimeConfig {
        gateway_address: std::net::SocketAddr::from(([127, 0, 0, 1], 15561)),
        gateway_token: String::new(),
        instance_id: String::from("instance"),
        caller_id: String::from(CALLER),
        session_id: String::from("session"),
        mcp_session_id: String::from(MCP_SESSION),
        lease_id: String::from("lease"),
        lease_epoch: 1,
        recovery_token: Some(String::from("recovery-token")),
        exact_restore_profile: false,
        recovery_profile: true,
        coop_native_peer_binding: None,
    }
}

fn context() -> JsonValue {
    JsonValue::object([
        (
            "deployment_id".to_owned(),
            JsonValue::string(uuid_v4().unwrap()),
        ),
        (
            "instance_id".to_owned(),
            JsonValue::string(uuid_v4().unwrap()),
        ),
        (
            "instance_incarnation".to_owned(),
            JsonValue::string(uuid_v4().unwrap()),
        ),
        ("boot_id".to_owned(), JsonValue::string(uuid_v4().unwrap())),
        ("authority_generation".to_owned(), JsonValue::Number(1)),
        ("lease_id".to_owned(), JsonValue::string(uuid_v4().unwrap())),
        ("lease_epoch".to_owned(), JsonValue::Number(1)),
    ])
}

fn operation_ref() -> JsonValue {
    JsonValue::object([
        (
            "operation_id".to_owned(),
            JsonValue::string(uuid_v4().unwrap()),
        ),
        (
            "payload_digest".to_owned(),
            JsonValue::string("0".repeat(64)),
        ),
        ("original_context".to_owned(), context()),
    ])
}

fn lookup_payload() -> JsonValue {
    JsonValue::object([
        ("operation".to_owned(), operation_ref()),
        (
            "lookup_scope".to_owned(),
            JsonValue::string("historical_read"),
        ),
    ])
}

fn reconcile_payload() -> JsonValue {
    JsonValue::object([
        ("operation".to_owned(), operation_ref()),
        ("strategy".to_owned(), JsonValue::string("receipt_lookup")),
        (
            "current_fence".to_owned(),
            JsonValue::object([
                ("state_id".to_owned(), JsonValue::string("live:7")),
                ("generation".to_owned(), JsonValue::Number(3)),
                (
                    "catalog_digest".to_owned(),
                    JsonValue::string("1".repeat(64)),
                ),
            ]),
        ),
    ])
}

fn payload(operation: RecoveryOperation) -> JsonValue {
    match operation {
        RecoveryOperation::Lookup => lookup_payload(),
        RecoveryOperation::Reconcile => reconcile_payload(),
    }
}

fn request(operation: RecoveryOperation) -> GatewayRequest {
    let built = build_recovery_request(operation, CALLER, &payload(operation)).unwrap();
    GatewayRequest {
        method: sts2_mcp_server::GatewayMethod::Post,
        path: String::from(operation.path()),
        headers: BTreeMap::from([
            (String::from("x-mcp-session-id"), String::from(MCP_SESSION)),
            (
                String::from("x-sts2-correlation-id"),
                built.correlation_id.clone(),
            ),
        ]),
        body: Some(built.frame),
        correlation: Correlation {
            mcp_session_id: String::from(MCP_SESSION),
            mcp_request_id: RequestId::Number(1),
        },
    }
}

#[test]
fn both_recovery_routes_bind_to_the_configured_caller() {
    for operation in RecoveryOperation::ALL {
        let bound = validate(&config(), &request(operation)).unwrap().unwrap();
        assert_eq!(bound.operation, operation);
        assert!(is_route(&request(operation)));
    }
}

#[test]
fn a_non_recovery_profile_refuses_the_recovery_route() {
    let mut config = config();
    config.recovery_profile = false;
    assert_eq!(
        validate(&config, &request(RecoveryOperation::Lookup)),
        Err(GatewayError::Rejected)
    );
}

#[test]
fn a_frame_naming_another_principal_is_refused_not_rewritten() {
    let mut request = request(RecoveryOperation::Lookup);
    let foreign = uuid_v4().unwrap();
    let built =
        build_recovery_request(RecoveryOperation::Lookup, &foreign, &lookup_payload()).unwrap();
    request.body = Some(built.frame);
    assert_eq!(validate(&config(), &request), Err(GatewayError::Rejected));
}

#[test]
fn a_foreign_or_extra_header_is_refused() {
    let mut request = request(RecoveryOperation::Lookup);
    request
        .headers
        .insert(String::from("x-sts2-caller-id"), String::from(CALLER));
    assert_eq!(validate(&config(), &request), Err(GatewayError::Rejected));
}

#[test]
fn a_correlation_that_does_not_match_the_frame_is_refused() {
    let mut request = request(RecoveryOperation::Lookup);
    request
        .headers
        .insert(String::from("x-sts2-correlation-id"), uuid_v4().unwrap());
    assert_eq!(validate(&config(), &request), Err(GatewayError::Rejected));
}

#[test]
fn an_unrelated_route_is_not_a_recovery_route() {
    let mut request = request(RecoveryOperation::Lookup);
    request.path = String::from("/v1/instances/instance/state");
    assert!(!is_route(&request));
    assert_eq!(validate(&config(), &request), Ok(None));
}

#[test]
fn a_lookup_payload_carried_on_the_reconcile_route_is_refused() {
    let mut request = request(RecoveryOperation::Reconcile);
    let built =
        build_recovery_request(RecoveryOperation::Lookup, CALLER, &lookup_payload()).unwrap();
    request.body = Some(built.frame);
    assert_eq!(validate(&config(), &request), Err(GatewayError::Rejected));
}
