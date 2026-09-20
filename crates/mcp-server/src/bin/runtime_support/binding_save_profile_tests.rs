// SPDX-License-Identifier: MIT
#![allow(clippy::unwrap_used)] // Fail immediately if disposable socket test setup fails.

use super::*;
use std::collections::BTreeMap;
use std::net::SocketAddr;
use sts2_mcp_server::{Correlation, JsonValue, RequestId, SAVE_PROFILE_CONTRACT};

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
        recovery_token: None,
        exact_restore_profile: false,
        recovery_profile: false,
        coop_native_peer_binding: None,
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
            (String::from("x-mcp-request-id"), String::from("request")),
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

fn select_body() -> JsonValue {
    JsonValue::object([
        (String::from("profile_id"), JsonValue::string("slot-1")),
        (
            String::from("baseline"),
            JsonValue::object([
                (String::from("identity"), JsonValue::string("base-1")),
                (
                    String::from("digest"),
                    JsonValue::string(
                        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                    ),
                ),
            ]),
        ),
    ])
}

#[test]
fn save_profile_routes_are_exact_bounded_and_authority_explicit() {
    let routes = [
        (
            GatewayMethod::Get,
            "/v1/instances/instance/save-profiles",
            None,
            Some("save_profile_response"),
        ),
        (
            GatewayMethod::Get,
            "/v1/instances/instance/save-profile/current",
            None,
            Some("save_profile_response"),
        ),
        (
            GatewayMethod::Post,
            "/v1/instances/instance/save-profile/select",
            Some(select_body()),
            Some("save_profile_response"),
        ),
        (
            GatewayMethod::Post,
            "/v1/instances/instance/save-profile/create-disposable",
            Some(JsonValue::object([])),
            Some("save_profile_response"),
        ),
        (
            GatewayMethod::Get,
            "/v1/instances/instance/save-profile/operations/op-1",
            None,
            Some("save_profile_response"),
        ),
    ];
    for (method, path, body, expected) in routes {
        let mut request = request();
        request.method = method;
        request.path = String::from(path);
        request.body = body;
        assert_eq!(response_kind(&config(), &request), expected);
        assert_eq!(admit(&config(), &request), Ok(()));
        for name in [
            "x-sts2-instance-id",
            "x-sts2-session-id",
            "x-sts2-lease-id",
            "x-sts2-lease-epoch",
        ] {
            let mut missing = request.clone();
            missing.headers.remove(name);
            assert_eq!(
                admit(&config(), &missing),
                Err(GatewayError::Rejected),
                "missing {name} was admitted for {path}"
            );
        }
    }

    let mut wrong_method = request();
    wrong_method.path = String::from("/v1/instances/instance/save-profiles");
    wrong_method.method = GatewayMethod::Post;
    wrong_method.body = Some(JsonValue::object([]));
    assert_eq!(response_kind(&config(), &wrong_method), None);
    assert_eq!(admit(&config(), &wrong_method), Err(GatewayError::Rejected));

    let mut body_on_read = request();
    body_on_read.path = String::from("/v1/instances/instance/save-profiles");
    body_on_read.body = Some(JsonValue::object([]));
    assert_eq!(response_kind(&config(), &body_on_read), None);
    assert_eq!(admit(&config(), &body_on_read), Err(GatewayError::Rejected));

    let mut foreign = request();
    foreign.path = String::from("/v1/instances/foreign/save-profiles");
    assert_eq!(response_kind(&config(), &foreign), None);
    assert_eq!(admit(&config(), &foreign), Err(GatewayError::Rejected));

    let mut unsafe_operation = request();
    unsafe_operation.path =
        String::from("/v1/instances/instance/save-profile/operations/../secret");
    assert_eq!(response_kind(&config(), &unsafe_operation), None);
    assert_eq!(
        admit(&config(), &unsafe_operation),
        Err(GatewayError::Rejected)
    );
}

#[test]
fn save_profile_body_does_not_receive_legacy_authority_injection() {
    let mut save = request();
    save.method = GatewayMethod::Post;
    save.path = String::from("/v1/instances/instance/save-profile/select");
    save.body = Some(select_body());
    let adapter = super::super::RuntimeGatewayAdapter::new(
        config(),
        super::super::http::LEGACY_MAX_RESPONSE_BYTES,
    );
    let encoded = adapter.body(&save).unwrap();
    assert_eq!(
        sts2_mcp_server::parse_json(std::str::from_utf8(&encoded).unwrap()).unwrap(),
        save.body.unwrap()
    );
}

#[test]
fn save_profile_response_binding_accepts_contract_errors_and_rejects_foreign_authority() {
    let body = JsonValue::object([
        (
            String::from("contract"),
            JsonValue::string(SAVE_PROFILE_CONTRACT),
        ),
        (String::from("operation_id"), JsonValue::string("operation")),
        (String::from("route"), JsonValue::string("select")),
        (String::from("status"), JsonValue::string("accepted")),
    ]);
    assert_eq!(
        response(&config(), &body, "request", "save_profile_response"),
        Ok(())
    );
    let bare_error = JsonValue::object([(
        String::from("error_code"),
        JsonValue::string("save_profile_fence_rejected"),
    )]);
    assert_eq!(
        response(&config(), &bare_error, "request", "save_profile_response"),
        Ok(())
    );
    let mut wrong = body.clone();
    if let JsonValue::Object(object) = &mut wrong {
        object.insert(
            String::from("contract"),
            JsonValue::string("gateway-save-profile-v99"),
        );
        object.insert(
            String::from("instance_id"),
            JsonValue::string("foreign-instance"),
        );
    }
    assert_eq!(
        response(&config(), &wrong, "request", "save_profile_response"),
        Err(GatewayError::MalformedResponse)
    );
    let mut wrong_identity = body;
    if let JsonValue::Object(object) = &mut wrong_identity {
        object.insert(
            String::from("instance_id"),
            JsonValue::string("foreign-instance"),
        );
    }
    assert_eq!(
        response(
            &config(),
            &wrong_identity,
            "request",
            "save_profile_response"
        ),
        Err(GatewayError::MalformedResponse)
    );
}
