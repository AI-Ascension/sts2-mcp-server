// SPDX-License-Identifier: MIT

#[path = "runtime_v2_support/mod.rs"]
mod support;

use sts2_mcp_server::{
    GatewayMethod, GatewayResponse, JsonValue, McpServer, RUNTIME_V2_ACTION_ID, ToolCatalog,
};
use support::{
    RecordingGateway, accepted, contains_result_field, observation, rejected, result, settled,
    submit_call,
};

#[test]
fn runtime_v2_catalog_exposes_only_submit_and_reconcile() {
    let mut server = McpServer::with_catalog(RecordingGateway::new([]), ToolCatalog::runtime_v2());
    let catalog = server
        .handle_frame("{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\",\"params\":{}}");
    assert!(catalog.contains("submit_action"));
    assert!(catalog.contains("reconcile_action"));
    assert!(!catalog.contains("get_state"));
    assert_eq!(catalog.matches("\"name\"").count(), 2);
    assert!(catalog.contains("runtime-v2-mcp"));
}

#[test]
fn accepted_preserves_the_complete_envelope_and_is_admission_only() {
    let mut server = McpServer::with_catalog(
        RecordingGateway::new([Ok(GatewayResponse {
            status: 200,
            body: accepted("request-accepted", 4),
        })]),
        ToolCatalog::runtime_v2(),
    );
    let response = server.handle_frame(&submit_call(
        "request-accepted",
        "instance-1",
        "session-1",
        "lease-1",
        1,
        4,
        "op-1",
    ));
    assert!(
        response.contains("\"isError\":false"),
        "response: {response}"
    );
    assert!(contains_result_field(
        &response,
        "protocol_version",
        "runtime-v2"
    ));
    assert!(response.contains("f7963b19c8ed5bbdc02c08e83c7a2e16c4771ed5eb798b29a8208d7a917a86c2"));
    assert!(contains_result_field(&response, "operation_id", "op-1"));
    assert!(contains_result_field(&response, "status", "accepted"));
    assert!(!response.contains("turn_end_settled"));
    let requests = &server.gateway().requests;
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method, GatewayMethod::Post);
    assert_eq!(requests[0].path, "/v1/instances/instance-1/action");
    let Some(JsonValue::Object(body)) = requests[0].body.as_ref() else {
        return;
    };
    assert_eq!(body.get("kind"), Some(&JsonValue::string("action_request")));
    assert_eq!(body.get("operation_id"), Some(&JsonValue::string("op-1")));
    assert_eq!(body.get("lease_epoch"), Some(&JsonValue::Number(1)));
    assert_eq!(
        body.get("action"),
        Some(&JsonValue::object([(
            String::from("action_id"),
            JsonValue::string(RUNTIME_V2_ACTION_ID),
        )]))
    );
}

#[test]
fn settled_requires_and_preserves_a_fresh_observation_witness() {
    let mut server = McpServer::with_catalog(
        RecordingGateway::new([Ok(GatewayResponse {
            status: 200,
            body: settled("request-settled", 5, "op-1", "action_response"),
        })]),
        ToolCatalog::runtime_v2(),
    );
    let response = server.handle_frame(&submit_call(
        "request-settled",
        "instance-1",
        "session-1",
        "lease-1",
        1,
        4,
        "op-1",
    ));
    assert!(response.contains("\"isError\":false"));
    assert!(contains_result_field(&response, "status", "settled"));
    assert!(contains_result_field(
        &response,
        "combat_phase",
        "combat/player_turn"
    ));
    assert!(contains_result_field(&response, "kind", "turn_end_settled"));
    assert_eq!(server.gateway().requests.len(), 1);
}

#[test]
fn rejected_cancelled_and_idempotency_conflict_preserve_exact_error_origin() {
    let cancelled = result(
        "request-cancelled",
        "instance-1",
        "session-1",
        "lease-1",
        1,
        4,
        "op-cancel",
        "action_response",
        "cancelled",
        Some(observation(4, "combat/player_turn", 2)),
        Some("sts2.runtime/cancelled_before_dispatch"),
        None,
    );
    let conflict = rejected("request-conflict", "idempotency_conflict", "op-1");
    let stale = rejected(
        "request-stale",
        "sts2.game-core/stale_generation",
        "op-stale",
    );
    let mut server = McpServer::with_catalog(
        RecordingGateway::new([
            Ok(GatewayResponse {
                status: 409,
                body: stale,
            }),
            Ok(GatewayResponse {
                status: 409,
                body: cancelled,
            }),
            Ok(GatewayResponse {
                status: 409,
                body: conflict,
            }),
        ]),
        ToolCatalog::runtime_v2(),
    );
    let stale_response = server.handle_frame(&submit_call(
        "request-stale",
        "instance-1",
        "session-1",
        "lease-1",
        1,
        3,
        "op-stale",
    ));
    let cancelled_response = server.handle_frame(&submit_call(
        "request-cancelled",
        "instance-1",
        "session-1",
        "lease-1",
        1,
        4,
        "op-cancel",
    ));
    let conflict_response = server.handle_frame(&submit_call(
        "request-conflict",
        "instance-1",
        "session-1",
        "lease-1",
        1,
        4,
        "op-1",
    ));
    assert!(stale_response.contains("sts2.game-core/stale_generation"));
    assert!(cancelled_response.contains("sts2.runtime/cancelled_before_dispatch"));
    assert!(conflict_response.contains("idempotency_conflict"));
    assert!(stale_response.contains("\"isError\":true"));
    assert!(contains_result_field(
        &cancelled_response,
        "status",
        "cancelled"
    ));
    assert!(contains_result_field(
        &conflict_response,
        "status",
        "rejected"
    ));
    assert_eq!(server.gateway().requests.len(), 3);
}
