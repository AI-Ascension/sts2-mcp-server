// SPDX-License-Identifier: MIT

#[path = "runtime_v2_support/mod.rs"]
#[allow(dead_code)]
mod support;

use sts2_mcp_server::{
    GatewayMethod, GatewayResponse, JsonValue, McpServer, RUNTIME_V3_GAMEPLAY_ACTION_ID,
    ToolCatalog,
};
use support::RecordingGateway;

const DIGEST: &str = "c961bbde893f0422f80233d14ea9ae8b648ee9032136e5370aa5f6b949f6575e";

#[test]
fn runtime_v3_gameplay_catalog_exposes_only_the_bounded_action_profile() {
    let mut server = McpServer::with_catalog(
        RecordingGateway::new([]),
        ToolCatalog::runtime_v3_gameplay(),
    );
    let catalog = server
        .handle_frame("{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\",\"params\":{}}");
    assert!(catalog.contains("runtime-v3-gameplay-mcp"));
    assert!(catalog.contains("card_index"));
    assert!(catalog.contains("target_id"));
    assert_eq!(catalog.matches("\"name\"").count(), 3);
    assert!(!catalog.contains("end_turn"));
}

#[test]
fn play_card_maps_to_the_v3_route_and_preserves_target_and_witness() -> Result<(), String> {
    let mut server = McpServer::with_catalog(
        RecordingGateway::new([Ok(GatewayResponse {
            status: 200,
            body: result(
                "request-card",
                5,
                "op-card",
                "action_response",
                "settled",
                Some("enemy-1"),
            ),
        })]),
        ToolCatalog::runtime_v3_gameplay(),
    );
    let response = server.handle_frame(&submit_call(
        "request-card",
        4,
        "op-card",
        0,
        Some("enemy-1"),
    ));
    assert!(response.contains("\"isError\":false"));
    assert!(response.contains("play_card_settled"));
    assert!(response.contains("enemy-1"));
    let request = &server.gateway().requests[0];
    assert_eq!(request.method, GatewayMethod::Post);
    assert_eq!(request.path, "/v3/instances/instance-1/action");
    let Some(JsonValue::Object(body)) = request.body.as_ref() else {
        return Err(String::from(
            "play_card request did not contain a JSON object",
        ));
    };
    assert_eq!(
        body.get("protocol_version"),
        Some(&JsonValue::string("runtime-v3-gameplay"))
    );
    assert_eq!(body.get("schema_digest"), Some(&JsonValue::string(DIGEST)));
    assert_eq!(body.get("kind"), Some(&JsonValue::string("action_request")));
    let Some(JsonValue::Object(action)) = body.get("action") else {
        return Err(String::from(
            "play_card request did not contain an action object",
        ));
    };
    assert_eq!(
        action.get("action_id"),
        Some(&JsonValue::string(RUNTIME_V3_GAMEPLAY_ACTION_ID))
    );
    assert_eq!(action.get("card_index"), Some(&JsonValue::Number(0)));
    assert_eq!(action.get("target_id"), Some(&JsonValue::string("enemy-1")));
    Ok(())
}

#[test]
fn uncertain_play_card_is_reported_without_a_mutation_retry() {
    let mut server = McpServer::with_catalog(
        RecordingGateway::new([Err(sts2_mcp_server::GatewayError::Timeout)]),
        ToolCatalog::runtime_v3_gameplay(),
    );
    let response = server.handle_frame(&submit_call("request-timeout", 4, "op-timeout", 1, None));
    assert!(response.contains("\"isError\":true"));
    assert!(
        response.contains("status\\\":\\\"unknown"),
        "response: {response}"
    );
    assert!(response.contains("unknown_after_disconnect"));
    assert_eq!(server.gateway().requests.len(), 1);
}

#[test]
fn malformed_play_card_response_keeps_operation_outcome_unknown() {
    let mut server = McpServer::with_catalog(
        RecordingGateway::new([Err(sts2_mcp_server::GatewayError::MalformedResponse)]),
        ToolCatalog::runtime_v3_gameplay(),
    );
    let response =
        server.handle_frame(&submit_call("request-bad", 4, "op-bad", 3, Some("enemy-1")));
    assert!(response.contains("\"isError\":true"));
    assert!(response.contains("status\\\":\\\"unknown"), "{response}");
    assert!(response.contains("enemy-1"));
    assert_eq!(server.gateway().requests.len(), 1);
}

#[test]
fn v3_state_and_reconcile_use_their_fixed_routes() {
    let mut server = McpServer::with_catalog(
        RecordingGateway::new([
            Ok(GatewayResponse {
                status: 200,
                body: state_response("request-state", 4),
            }),
            Ok(GatewayResponse {
                status: 200,
                body: result(
                    "request-reconcile",
                    5,
                    "op-card",
                    "reconcile_response",
                    "settled",
                    None,
                ),
            }),
        ]),
        ToolCatalog::runtime_v3_gameplay(),
    );
    let state = server.handle_frame(&state_call("request-state", 4));
    let reconcile = server.handle_frame(&reconcile_call("request-reconcile", 5, "op-card"));
    assert!(state.contains("\"isError\":false"));
    assert!(reconcile.contains("\"isError\":false"));
    assert_eq!(server.gateway().requests[0].method, GatewayMethod::Get);
    assert_eq!(
        server.gateway().requests[0].path,
        "/v3/instances/instance-1/state"
    );
    assert_eq!(server.gateway().requests[1].method, GatewayMethod::Get);
    assert_eq!(
        server.gateway().requests[1].path,
        "/v3/instances/instance-1/operations/op-card"
    );
    for request in &server.gateway().requests {
        for (key, value) in [
            ("x-sts2-instance-id", "instance-1"),
            ("x-sts2-session-id", "session-1"),
            ("x-sts2-lease-id", "lease-1"),
            ("x-sts2-lease-epoch", "1"),
        ] {
            assert_eq!(request.headers.get(key).map(String::as_str), Some(value));
        }
    }
}

#[test]
fn reconcile_accepts_the_recorded_card_and_target_without_resubmission() {
    let mut body = result(
        "reconcile",
        5,
        "op-card",
        "reconcile_response",
        "settled",
        Some("enemy-1"),
    );
    set_card(&mut body, "action", 3);
    set_card(&mut body, "effect_witness", 3);
    let mut server = McpServer::with_catalog(
        RecordingGateway::new([Ok(GatewayResponse { status: 200, body })]),
        ToolCatalog::runtime_v3_gameplay(),
    );
    let response = server.handle_frame(&reconcile_call("reconcile", 4, "op-card"));
    assert!(response.contains("\"isError\":false"), "{response}");
    assert!(response.contains("enemy-1"));
    assert_eq!(server.gateway().requests.len(), 1);
    assert_eq!(server.gateway().requests[0].method, GatewayMethod::Get);
    assert!(server.gateway().requests[0].body.is_none());
}

#[test]
fn reconcile_rejects_witness_action_conflicts_and_out_of_bound_cards() {
    for (card, witness_card) in [(3, 0), (65, 65)] {
        let mut body = result(
            "reconcile",
            5,
            "op-card",
            "reconcile_response",
            "settled",
            Some("enemy-1"),
        );
        set_card(&mut body, "action", card);
        set_card(&mut body, "effect_witness", witness_card);
        let mut server = McpServer::with_catalog(
            RecordingGateway::new([Ok(GatewayResponse { status: 200, body })]),
            ToolCatalog::runtime_v3_gameplay(),
        );
        let response = server.handle_frame(&reconcile_call("reconcile", 4, "op-card"));
        assert!(response.contains("\"isError\":true"), "{response}");
    }
}

#[test]
fn submit_still_rejects_a_different_action_in_the_receipt() {
    let body = result(
        "submit",
        5,
        "op-card",
        "action_response",
        "settled",
        Some("enemy-1"),
    );
    let mut server = McpServer::with_catalog(
        RecordingGateway::new([Ok(GatewayResponse { status: 200, body })]),
        ToolCatalog::runtime_v3_gameplay(),
    );
    let response = server.handle_frame(&submit_call("submit", 4, "op-card", 3, Some("enemy-1")));
    assert!(response.contains("\"isError\":true"), "{response}");
}

#[test]
fn unavailable_reconciliation_never_invents_a_default_card_receipt() {
    let mut server = McpServer::with_catalog(
        RecordingGateway::new([Err(sts2_mcp_server::GatewayError::Timeout)]),
        ToolCatalog::runtime_v3_gameplay(),
    );
    let response = server.handle_frame(&reconcile_call("reconcile", 4, "op-card"));
    assert!(response.contains("\"isError\":true"));
    assert!(response.contains("outcome remains unknown"));
    assert!(!response.contains("card_index"));
    assert_eq!(server.gateway().requests.len(), 1);
}

fn set_card(body: &mut JsonValue, field: &str, card: i64) {
    if let JsonValue::Object(object) = body
        && let Some(JsonValue::Object(value)) = object.get_mut(field)
    {
        value.insert(String::from("card_index"), JsonValue::Number(card));
    }
}

#[test]
fn slash_operation_ids_are_rejected_before_mutation_or_read() {
    let mut server = McpServer::with_catalog(
        RecordingGateway::new([]),
        ToolCatalog::runtime_v3_gameplay(),
    );
    for call in [
        submit_call("submit", 4, "namespace/op-card", 0, None),
        reconcile_call("read", 4, "namespace/op-card"),
    ] {
        let response = server.handle_frame(&call);
        assert!(response.contains("unsafe or oversized"), "{response}");
    }
    assert!(server.gateway().requests.is_empty());
}

fn submit_call(
    id: &str,
    generation: i64,
    operation: &str,
    card_index: i64,
    target_id: Option<&str>,
) -> String {
    let target = target_id.map_or_else(|| String::from("null"), |value| format!("\"{value}\""));
    format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":\"{id}\",\"method\":\"tools/call\",\"params\":{{\"name\":\"submit_action\",\"arguments\":{{\"instance_id\":\"instance-1\",\"mcp_session_id\":\"session-1\",\"lease_id\":\"lease-1\",\"lease_epoch\":1,\"generation\":{generation},\"operation_id\":\"{operation}\",\"action_id\":\"play_card\",\"card_index\":{card_index},\"target_id\":{target}}}}}}}"
    )
}

fn state_call(id: &str, generation: i64) -> String {
    format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":\"{id}\",\"method\":\"tools/call\",\"params\":{{\"name\":\"get_state\",\"arguments\":{{\"instance_id\":\"instance-1\",\"mcp_session_id\":\"session-1\",\"lease_id\":\"lease-1\",\"lease_epoch\":1,\"generation\":{generation}}}}}}}"
    )
}

fn reconcile_call(id: &str, generation: i64, operation: &str) -> String {
    format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":\"{id}\",\"method\":\"tools/call\",\"params\":{{\"name\":\"reconcile_action\",\"arguments\":{{\"instance_id\":\"instance-1\",\"mcp_session_id\":\"session-1\",\"lease_id\":\"lease-1\",\"lease_epoch\":1,\"generation\":{generation},\"operation_id\":\"{operation}\"}}}}}}"
    )
}

fn provenance() -> JsonValue {
    JsonValue::object([
        (
            String::from("artifact"),
            JsonValue::string("sts2-protocol/runtime-v3-gameplay"),
        ),
        (
            String::from("source"),
            JsonValue::string("schemas/runtime-v3-gameplay.schema.json"),
        ),
        (
            String::from("generator"),
            JsonValue::string("hand-authored"),
        ),
    ])
}

fn observation(generation: i64, turn_index: i64) -> JsonValue {
    JsonValue::object([
        (
            String::from("combat_phase"),
            JsonValue::string("combat/player_turn"),
        ),
        (String::from("turn_index"), JsonValue::Number(turn_index)),
        (String::from("host_ready"), JsonValue::Bool(true)),
        (String::from("generation"), JsonValue::Number(generation)),
        (String::from("hand_count"), JsonValue::Number(4)),
        (String::from("energy"), JsonValue::Number(1)),
        (String::from("draw_pile_count"), JsonValue::Number(10)),
        (String::from("discard_pile_count"), JsonValue::Number(1)),
        (String::from("exhaust_pile_count"), JsonValue::Number(0)),
        (
            String::from("enemies"),
            JsonValue::Array(vec![JsonValue::object([
                (String::from("target_id"), JsonValue::string("enemy-1")),
                (String::from("alive"), JsonValue::Bool(true)),
                (String::from("hittable"), JsonValue::Bool(true)),
            ])]),
        ),
    ])
}

fn state_response(correlation: &str, generation: i64) -> JsonValue {
    JsonValue::object([
        (
            String::from("protocol_version"),
            JsonValue::string("runtime-v3-gameplay"),
        ),
        (String::from("schema_digest"), JsonValue::string(DIGEST)),
        (String::from("provenance"), provenance()),
        (
            String::from("correlation_id"),
            JsonValue::string(correlation),
        ),
        (String::from("instance_id"), JsonValue::string("instance-1")),
        (String::from("session_id"), JsonValue::string("session-1")),
        (String::from("lease_id"), JsonValue::string("lease-1")),
        (String::from("lease_epoch"), JsonValue::Number(1)),
        (String::from("generation"), JsonValue::Number(generation)),
        (String::from("kind"), JsonValue::string("state_response")),
        (String::from("operation_id"), JsonValue::Null),
        (String::from("observation"), observation(generation, 2)),
        (String::from("action"), JsonValue::Null),
        (String::from("status"), JsonValue::Null),
        (String::from("error_code"), JsonValue::Null),
        (String::from("effect_witness"), JsonValue::Null),
    ])
}

fn result(
    correlation: &str,
    generation: i64,
    operation: &str,
    kind: &str,
    status: &str,
    target_id: Option<&str>,
) -> JsonValue {
    let witness = if status == "settled" {
        Some(JsonValue::object([
            (String::from("kind"), JsonValue::string("play_card_settled")),
            (String::from("generation"), JsonValue::Number(generation)),
            (String::from("card_index"), JsonValue::Number(0)),
            (
                String::from("target_id"),
                target_id.map_or(JsonValue::Null, JsonValue::string),
            ),
        ]))
    } else {
        None
    };
    JsonValue::object([
        (
            String::from("protocol_version"),
            JsonValue::string("runtime-v3-gameplay"),
        ),
        (String::from("schema_digest"), JsonValue::string(DIGEST)),
        (String::from("provenance"), provenance()),
        (
            String::from("correlation_id"),
            JsonValue::string(correlation),
        ),
        (String::from("instance_id"), JsonValue::string("instance-1")),
        (String::from("session_id"), JsonValue::string("session-1")),
        (String::from("lease_id"), JsonValue::string("lease-1")),
        (String::from("lease_epoch"), JsonValue::Number(1)),
        (String::from("generation"), JsonValue::Number(generation)),
        (String::from("kind"), JsonValue::string(kind)),
        (String::from("operation_id"), JsonValue::string(operation)),
        (String::from("observation"), observation(generation, 2)),
        (
            String::from("action"),
            JsonValue::object([
                (String::from("action_id"), JsonValue::string("play_card")),
                (String::from("card_index"), JsonValue::Number(0)),
                (
                    String::from("target_id"),
                    target_id.map_or(JsonValue::Null, JsonValue::string),
                ),
            ]),
        ),
        (String::from("status"), JsonValue::string(status)),
        (String::from("error_code"), JsonValue::Null),
        (
            String::from("effect_witness"),
            witness.unwrap_or(JsonValue::Null),
        ),
    ])
}
