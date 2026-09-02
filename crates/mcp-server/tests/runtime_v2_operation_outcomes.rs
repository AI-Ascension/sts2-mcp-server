// SPDX-License-Identifier: MIT

#[path = "runtime_v2_support/mod.rs"]
#[allow(dead_code)]
mod support;

use sts2_mcp_server::{GatewayError, GatewayResponse, JsonValue, McpServer, ToolCatalog};
use support::{
    RecordingGateway, contains_result_field, observation, reconcile_call, result, settled,
    submit_call,
};

#[test]
fn timeout_becomes_unknown_without_an_automatic_mutation_retry() {
    let mut server = McpServer::with_catalog(
        RecordingGateway::new([Err(GatewayError::Timeout)]),
        ToolCatalog::runtime_v2(),
    );
    let response = server.handle_frame(&submit_call(
        "request-timeout",
        "instance-1",
        "session-1",
        "lease-1",
        1,
        4,
        "op-timeout",
    ));
    assert!(response.contains("\"isError\":true"));
    assert!(contains_result_field(&response, "status", "unknown"));
    assert!(response.contains("sts2.runtime/unknown_after_disconnect"));
    assert_eq!(response.matches("status").count(), 1);
    assert_eq!(server.gateway().requests.len(), 1);
}

#[test]
fn duplicate_replays_and_reconcile_keep_the_same_operation_identity() {
    let duplicate = settled("request-duplicate", 5, "op-duplicate", "action_response");
    let mut server = McpServer::with_catalog(
        RecordingGateway::new([
            Ok(GatewayResponse {
                status: 200,
                body: duplicate.clone(),
            }),
            Ok(GatewayResponse {
                status: 200,
                body: duplicate,
            }),
            Ok(GatewayResponse {
                status: 200,
                body: settled("request-reconcile", 5, "op-timeout", "reconcile_response"),
            }),
        ]),
        ToolCatalog::runtime_v2(),
    );
    let duplicate_call = submit_call(
        "request-duplicate",
        "instance-1",
        "session-1",
        "lease-1",
        1,
        4,
        "op-duplicate",
    );
    let first = server.handle_frame(&duplicate_call);
    let replay = server.handle_frame(&duplicate_call);
    let reconcile = server.handle_frame(&reconcile_call(
        "request-reconcile",
        "instance-1",
        "session-1",
        "lease-1",
        1,
        4,
        "op-timeout",
    ));
    assert!(contains_result_field(&first, "status", "settled"));
    assert!(contains_result_field(&replay, "status", "settled"));
    assert!(contains_result_field(
        &first,
        "operation_id",
        "op-duplicate"
    ));
    assert!(contains_result_field(&reconcile, "status", "settled"));
    assert!(contains_result_field(
        &reconcile,
        "operation_id",
        "op-timeout"
    ));
    assert_eq!(server.gateway().requests.len(), 3);
    assert_eq!(
        server.gateway().requests[2].path,
        "/v1/instances/instance-1/reconcile"
    );
    let Some(JsonValue::Object(body)) = server.gateway().requests[2].body.as_ref() else {
        return;
    };
    assert_eq!(
        body.get("kind"),
        Some(&JsonValue::string("reconcile_request"))
    );
    assert_eq!(
        body.get("operation_id"),
        Some(&JsonValue::string("op-timeout"))
    );
    assert_eq!(body.get("action"), Some(&JsonValue::Null));
}

#[test]
fn unknown_arguments_and_stale_fences_fail_closed_before_gateway_access() {
    let mut server = McpServer::with_catalog(RecordingGateway::new([]), ToolCatalog::runtime_v2());
    let unknown = server.handle_frame(
        "{\"jsonrpc\":\"2.0\",\"id\":\"unknown\",\"method\":\"tools/call\",\
         \"params\":{\"name\":\"submit_action\",\"arguments\":{\
         \"instance_id\":\"instance-1\",\"mcp_session_id\":\"session-1\",\
         \"lease_id\":\"lease-1\",\"lease_epoch\":1,\"generation\":4,\
         \"operation_id\":\"op-1\",\"action_id\":\"end_turn\",\"units\":1}}}",
    );
    let stale_epoch = server.handle_frame(&submit_call(
        "stale-epoch",
        "instance-1",
        "session-1",
        "lease-1",
        9_007_199_254_740_992,
        4,
        "op-stale-epoch",
    ));
    let unsafe_operation = server.handle_frame(&submit_call(
        "unsafe-operation",
        "instance-1",
        "session-1",
        "lease-1",
        1,
        4,
        "operation id",
    ));
    assert!(unknown.contains("\"code\":-32602"));
    assert!(stale_epoch.contains("\"code\":-32602"));
    assert!(unsafe_operation.contains("\"code\":-32602"));
    assert_eq!(server.gateway().requests.len(), 0);
}

#[test]
fn settled_without_a_fresh_witness_is_not_reported_as_settled() {
    let invalid = result(
        "request-invalid-settled",
        "instance-1",
        "session-1",
        "lease-1",
        1,
        4,
        "op-1",
        "action_response",
        "settled",
        Some(observation(4, "combat/player_turn", 2)),
        None,
        None,
    );
    let mut server = McpServer::with_catalog(
        RecordingGateway::new([Ok(GatewayResponse {
            status: 200,
            body: invalid,
        })]),
        ToolCatalog::runtime_v2(),
    );
    let response = server.handle_frame(&submit_call(
        "request-invalid-settled",
        "instance-1",
        "session-1",
        "lease-1",
        1,
        4,
        "op-1",
    ));
    assert!(response.contains("\"isError\":true"));
    assert!(response.contains("Runtime-v2 envelope"));
    assert_eq!(server.gateway().requests.len(), 1);
}
