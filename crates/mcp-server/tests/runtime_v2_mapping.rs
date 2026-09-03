// SPDX-License-Identifier: MIT

#[path = "runtime_v2_support/mod.rs"]
mod support;

use sts2_mcp_server::{
    GatewayMethod, GatewayResponse, JsonValue, McpServer, RUNTIME_V2_ACTION_ID, ToolCatalog,
};
use support::{
    RecordingGateway, accepted, contains_result_field, observation, rejected, result, settled,
    state_call, state_response, submit_call,
};

#[test]
fn runtime_v2_catalog_exposes_state_submit_and_reconcile() {
    let mut server = McpServer::with_catalog(RecordingGateway::new([]), ToolCatalog::runtime_v2());
    let catalog = server
        .handle_frame("{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\",\"params\":{}}");
    assert!(catalog.contains("get_state"));
    assert!(catalog.contains("submit_action"));
    assert!(catalog.contains("reconcile_action"));
    assert_eq!(catalog.matches("\"name\"").count(), 3);
    assert!(catalog.contains("runtime-v2-mcp"));
}

#[test]
fn gateway_overload_preserves_typed_retry_guidance_at_mcp_boundary() {
    let mut server = McpServer::with_catalog(
        RecordingGateway::new([Ok(GatewayResponse {
            status: 429,
            body: JsonValue::object([
                (
                    String::from("error_code"),
                    JsonValue::string("runtime_v2_queue_capacity"),
                ),
                (String::from("retryable"), JsonValue::Bool(true)),
                (String::from("retry_after_ms"), JsonValue::Number(1000)),
            ]),
        })]),
        ToolCatalog::runtime_v2(),
    );
    let response = server.handle_frame(&submit_call(
        "request-overloaded",
        "instance-1",
        "session-1",
        "lease-1",
        1,
        4,
        "op-overloaded",
    ));
    assert!(response.contains("\"isError\":true"));
    assert!(response.contains("runtime_v2_queue_capacity"));
    assert!(response.contains(r#"\"retryable\":true"#));
    assert!(response.contains(r#"\"retry_after_ms\":1000"#));
    assert!(!response.contains("invalid Runtime-v2 envelope"));
}

#[test]
fn configured_mcp_and_gateway_sessions_remain_distinct_at_the_mapping_boundary() {
    let mut server = McpServer::with_catalog_and_sessions(
        RecordingGateway::new([Ok(GatewayResponse {
            status: 200,
            body: state_response("request-bound", 4),
        })]),
        ToolCatalog::runtime_v2(),
        "session-1",
        "mcp-session-1",
    );
    let response = server.handle_frame(&state_call(
        "request-bound",
        "instance-1",
        "mcp-session-1",
        "lease-1",
        1,
        4,
    ));
    assert!(response.contains("\"isError\":false"));
    let request = &server.gateway().requests[0];
    assert_eq!(
        request.headers.get("x-mcp-session-id"),
        Some(&String::from("mcp-session-1"))
    );
    assert_eq!(request.correlation.mcp_session_id, "mcp-session-1");
    assert_eq!(
        request.body.as_ref().and_then(|body| match body {
            JsonValue::Object(object) => object.get("session_id"),
            _ => None,
        }),
        Some(&JsonValue::string("session-1"))
    );
}

#[test]
fn configured_mcp_session_mismatch_is_rejected_before_gateway_access() {
    let mut server = McpServer::with_catalog_and_sessions(
        RecordingGateway::new([]),
        ToolCatalog::runtime_v2(),
        "session-1",
        "mcp-session-1",
    );
    let response = server.handle_frame(&state_call(
        "request-wrong-mcp",
        "instance-1",
        "other-mcp-session",
        "lease-1",
        1,
        4,
    ));
    assert!(response.contains("MCP session identity does not match"));
    assert!(server.gateway().requests.is_empty());
}

#[test]
fn state_preserves_the_complete_response_and_uses_the_v2_read_route() {
    let mut server = McpServer::with_catalog(
        RecordingGateway::new([Ok(GatewayResponse {
            status: 200,
            body: state_response("request-state", 4),
        })]),
        ToolCatalog::runtime_v2(),
    );
    let response = server.handle_frame(&state_call(
        "request-state",
        "instance-1",
        "session-1",
        "lease-1",
        1,
        4,
    ));
    assert!(response.contains("\"isError\":false"));
    assert!(contains_result_field(&response, "kind", "state_response"));
    assert!(contains_result_field(
        &response,
        "combat_phase",
        "combat/player_turn"
    ));
    let request = &server.gateway().requests[0];
    assert_eq!(request.method, GatewayMethod::Get);
    assert_eq!(request.path, "/v2/instances/instance-1/state");
    assert_eq!(
        request.body,
        Some(JsonValue::object([
            (
                String::from("protocol_version"),
                JsonValue::string("runtime-v2"),
            ),
            (
                String::from("schema_digest"),
                JsonValue::string(
                    "f7963b19c8ed5bbdc02c08e83c7a2e16c4771ed5eb798b29a8208d7a917a86c2",
                ),
            ),
            (
                String::from("provenance"),
                JsonValue::object([
                    (
                        String::from("artifact"),
                        JsonValue::string("sts2-protocol/runtime-v2"),
                    ),
                    (
                        String::from("source"),
                        JsonValue::string("schemas/runtime-v2.schema.json"),
                    ),
                    (
                        String::from("generator"),
                        JsonValue::string("hand-authored")
                    ),
                ]),
            ),
            (
                String::from("correlation_id"),
                JsonValue::string("request-state"),
            ),
            (String::from("instance_id"), JsonValue::string("instance-1"),),
            (String::from("session_id"), JsonValue::string("session-1")),
            (String::from("lease_id"), JsonValue::string("lease-1")),
            (String::from("lease_epoch"), JsonValue::Number(1)),
            (String::from("generation"), JsonValue::Number(4)),
            (String::from("kind"), JsonValue::string("state_request")),
            (String::from("operation_id"), JsonValue::Null),
            (String::from("observation"), JsonValue::Null),
            (String::from("action"), JsonValue::Null),
            (String::from("status"), JsonValue::Null),
            (String::from("error_code"), JsonValue::Null),
            (String::from("effect_witness"), JsonValue::Null),
        ]))
    );
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
    assert_eq!(requests[0].path, "/v2/instances/instance-1/action");
    assert_eq!(
        requests[0].body,
        Some(JsonValue::object([
            (
                String::from("protocol_version"),
                JsonValue::string("runtime-v2"),
            ),
            (
                String::from("schema_digest"),
                JsonValue::string(
                    "f7963b19c8ed5bbdc02c08e83c7a2e16c4771ed5eb798b29a8208d7a917a86c2",
                ),
            ),
            (
                String::from("provenance"),
                JsonValue::object([
                    (
                        String::from("artifact"),
                        JsonValue::string("sts2-protocol/runtime-v2"),
                    ),
                    (
                        String::from("source"),
                        JsonValue::string("schemas/runtime-v2.schema.json"),
                    ),
                    (
                        String::from("generator"),
                        JsonValue::string("hand-authored")
                    ),
                ]),
            ),
            (
                String::from("correlation_id"),
                JsonValue::string("request-accepted"),
            ),
            (String::from("instance_id"), JsonValue::string("instance-1"),),
            (String::from("session_id"), JsonValue::string("session-1")),
            (String::from("lease_id"), JsonValue::string("lease-1")),
            (String::from("lease_epoch"), JsonValue::Number(1)),
            (String::from("generation"), JsonValue::Number(4)),
            (String::from("kind"), JsonValue::string("action_request")),
            (String::from("operation_id"), JsonValue::string("op-1")),
            (String::from("observation"), JsonValue::Null),
            (
                String::from("action"),
                JsonValue::object([(
                    String::from("action_id"),
                    JsonValue::string(RUNTIME_V2_ACTION_ID),
                )]),
            ),
            (String::from("status"), JsonValue::Null),
            (String::from("error_code"), JsonValue::Null),
            (String::from("effect_witness"), JsonValue::Null),
        ]))
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
