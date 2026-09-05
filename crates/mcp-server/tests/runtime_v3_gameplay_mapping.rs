// SPDX-License-Identifier: MIT

use std::collections::VecDeque;

use sts2_mcp_server::{
    DISPATCH_ACTION_TOOL, GatewayAdapter, GatewayError, GatewayMethod, GatewayRequest,
    GatewayResponse, JsonValue, LEGAL_ACTIONS_TOOL, McpServer, OBSERVE_TOOL, RECOVER_TOOL,
    REOBSERVE_TOOL, ToolCatalog, WAIT_FOR_TRANSITION_TOOL,
};

struct RecordingGateway {
    requests: Vec<GatewayRequest>,
    responses: VecDeque<Result<GatewayResponse, GatewayError>>,
}

impl RecordingGateway {
    fn new(responses: impl IntoIterator<Item = Result<GatewayResponse, GatewayError>>) -> Self {
        Self {
            requests: Vec::new(),
            responses: responses.into_iter().collect(),
        }
    }
}

impl GatewayAdapter for RecordingGateway {
    fn forward(&mut self, request: GatewayRequest) -> Result<GatewayResponse, GatewayError> {
        self.requests.push(request);
        self.responses
            .pop_front()
            .unwrap_or(Err(GatewayError::Unavailable))
    }
}

fn context_arguments(extra: &str) -> String {
    format!(
        "\"instance_id\":\"instance-1\",\"mcp_session_id\":\"session-1\",\
         \"lease_id\":\"lease-1\",\"lease_epoch\":1,\"generation\":4{extra}"
    )
}

fn call(tool: &str, arguments: &str) -> String {
    format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":\"request-1\",\"method\":\"tools/call\",\
         \"params\":{{\"name\":\"{tool}\",\"arguments\":{{{arguments}}}}}}}"
    )
}

#[allow(clippy::too_many_arguments)]
fn root(
    kind: &str,
    generation: i64,
    state_id: JsonValue,
    operation_id: JsonValue,
    observation: JsonValue,
    legal_actions: JsonValue,
    action: JsonValue,
    status: JsonValue,
    transition: JsonValue,
    error_code: JsonValue,
    wait_outcome: JsonValue,
) -> JsonValue {
    let mut fields = fixture_identity(generation);
    fields.extend([
        (String::from("kind"), JsonValue::string(kind)),
        (String::from("state_id"), state_id),
        (String::from("operation_id"), operation_id),
        (String::from("observation"), observation),
        (String::from("legal_actions"), legal_actions),
        (String::from("action"), action),
        (String::from("status"), status),
        (String::from("transition"), transition),
        (String::from("error_code"), error_code),
        (String::from("wait_for_millis"), JsonValue::Null),
        (String::from("wait_outcome"), wait_outcome),
        (String::from("recovery"), JsonValue::Null),
    ]);
    JsonValue::object(fields)
}

fn observation(generation: i64) -> JsonValue {
    JsonValue::object([
        (String::from("state_id"), JsonValue::string("combat-1")),
        (String::from("generation"), JsonValue::Number(generation)),
        (
            String::from("visible_seed"),
            JsonValue::string("visible-seed"),
        ),
        (
            String::from("player"),
            JsonValue::object([
                (String::from("hp"), JsonValue::Number(70)),
                (String::from("max_hp"), JsonValue::Number(70)),
                (String::from("energy"), JsonValue::Number(3)),
                (String::from("gold"), JsonValue::Number(99)),
                (String::from("hand"), JsonValue::Array(Vec::new())),
                (String::from("deck"), JsonValue::Array(Vec::new())),
                (String::from("discard"), JsonValue::Array(Vec::new())),
                (String::from("exhaust"), JsonValue::Array(Vec::new())),
            ]),
        ),
        (
            String::from("state"),
            JsonValue::object([
                (String::from("state"), JsonValue::string("combat")),
                (String::from("turn_index"), JsonValue::Number(2)),
                (String::from("enemies"), JsonValue::Array(Vec::new())),
            ]),
        ),
    ])
}

fn legal_actions() -> JsonValue {
    JsonValue::Array(vec![JsonValue::object([
        (String::from("action_id"), JsonValue::string("end-turn")),
        (
            String::from("action"),
            JsonValue::object([(String::from("kind"), JsonValue::string("end_turn"))]),
        ),
    ])])
}

fn state_response(generation: i64) -> JsonValue {
    root(
        "state_response",
        generation,
        JsonValue::string("combat-1"),
        JsonValue::Null,
        observation(generation),
        legal_actions(),
        JsonValue::Null,
        JsonValue::Null,
        JsonValue::Null,
        JsonValue::Null,
        JsonValue::Null,
    )
}

fn rejected_response() -> JsonValue {
    root(
        "dispatch_action_response",
        4,
        JsonValue::string("combat-1"),
        JsonValue::string("operation-1"),
        observation(4),
        legal_actions(),
        JsonValue::Null,
        JsonValue::string("rejected"),
        JsonValue::Null,
        JsonValue::string("sts2.game-mod/stale_generation"),
        JsonValue::Null,
    )
}

#[test]
fn catalog_exposes_only_the_six_semantic_operations() {
    let catalog = ToolCatalog::runtime_v3_gameplay();
    let mut server = McpServer::with_catalog(RecordingGateway::new([]), catalog);
    let response = server
        .handle_frame("{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\",\"params\":{}}");
    for name in [
        OBSERVE_TOOL,
        LEGAL_ACTIONS_TOOL,
        DISPATCH_ACTION_TOOL,
        WAIT_FOR_TRANSITION_TOOL,
        REOBSERVE_TOOL,
        RECOVER_TOOL,
    ] {
        assert!(response.contains(name), "missing tool {name}");
    }
    assert_eq!(response.matches("\"name\"").count(), 6);
    assert!(!response.contains("shell"));
    assert!(!response.contains("raw_memory"));
}

#[test]
fn observe_maps_the_complete_identity_bound_request_to_the_v3_route() -> Result<(), String> {
    let mut server = McpServer::with_catalog(
        RecordingGateway::new([Ok(GatewayResponse {
            status: 200,
            body: state_response(4),
        })]),
        ToolCatalog::runtime_v3_gameplay(),
    );
    let response = server.handle_frame(&call(OBSERVE_TOOL, &context_arguments("")));
    assert!(
        response.contains("\"isError\":false"),
        "response: {response}"
    );
    assert!(response.contains("combat-1"));
    assert_eq!(server.gateway().requests.len(), 1);
    let request = &server.gateway().requests[0];
    assert_eq!(request.method, GatewayMethod::Get);
    assert_eq!(request.path, "/v3/instances/instance-1/state");
    let Some(JsonValue::Object(body)) = request.body.as_ref() else {
        return Err(String::from("expected a Runtime-v3 envelope"));
    };
    assert_eq!(body.get("kind"), Some(&JsonValue::string("state_request")));
    assert_eq!(body.get("generation"), Some(&JsonValue::Number(4)));
    assert_eq!(body.get("lease_epoch"), Some(&JsonValue::Number(1)));
    Ok(())
}

#[test]
fn dispatch_maps_one_typed_action_and_preserves_stale_rejection() {
    let mut server = McpServer::with_catalog(
        RecordingGateway::new([Ok(GatewayResponse {
            status: 409,
            body: rejected_response(),
        })]),
        ToolCatalog::runtime_v3_gameplay(),
    );
    let arguments = context_arguments(
        ",\"state_id\":\"combat-1\",\"operation_id\":\"operation-1\",\
         \"action\":{\"action_id\":\"end-turn\",\"action\":{\"kind\":\"end_turn\"}}",
    );
    let response = server.handle_frame(&call(DISPATCH_ACTION_TOOL, &arguments));
    assert!(response.contains("\"isError\":true"));
    assert!(response.contains("stale_generation"));
    let request = &server.gateway().requests[0];
    assert_eq!(request.method, GatewayMethod::Post);
    assert_eq!(request.path, "/v3/instances/instance-1/action");
    assert!(request.body.as_ref().is_some_and(|body| {
        body.to_json().contains("dispatch_action_request")
            && body.to_json().contains("end-turn")
            && !body.to_json().contains("coordinates")
    }));
}

#[test]
fn action_shape_and_unknown_response_fields_fail_closed_before_or_at_projection() {
    let mut server = McpServer::with_catalog(
        RecordingGateway::new([]),
        ToolCatalog::runtime_v3_gameplay(),
    );
    let invalid_action = context_arguments(
        ",\"state_id\":\"combat-1\",\"operation_id\":\"operation-1\",\
         \"action\":{\"action_id\":\"end-turn\",\"action\":{\"kind\":\"end_turn\",\"shell\":\"rm\"}}",
    );
    let invalid = server.handle_frame(&call(DISPATCH_ACTION_TOOL, &invalid_action));
    assert!(invalid.contains("\"code\":-32602"));
    assert!(server.gateway().requests.is_empty());

    let mut response = state_response(4);
    if let JsonValue::Object(object) = &mut response {
        object.insert(String::from("raw_memory"), JsonValue::string("blocked"));
    }
    let mut projection_server = McpServer::with_catalog(
        RecordingGateway::new([Ok(GatewayResponse {
            status: 200,
            body: response,
        })]),
        ToolCatalog::runtime_v3_gameplay(),
    );
    let observed = projection_server.handle_frame(&call(OBSERVE_TOOL, &context_arguments("")));
    assert!(observed.contains("\"isError\":true"));
    assert!(!observed.contains("blocked"));
}

#[test]
fn timeout_on_dispatch_is_unknown_and_is_not_retried() {
    let mut server = McpServer::with_catalog(
        RecordingGateway::new([Err(GatewayError::Timeout)]),
        ToolCatalog::runtime_v3_gameplay(),
    );
    let arguments = context_arguments(
        ",\"state_id\":\"combat-1\",\"operation_id\":\"operation-1\",\
         \"action\":{\"action_id\":\"end-turn\",\"action\":{\"kind\":\"end_turn\"}}",
    );
    let response = server.handle_frame(&call(DISPATCH_ACTION_TOOL, &arguments));
    assert!(response.contains("\\\"status\\\":\\\"unknown\\\""));
    assert!(response.contains("unknown_after_disconnect"));
    assert_eq!(server.gateway().requests.len(), 1);
}

#[test]
fn timeout_on_wait_is_an_unknown_timeout_result() {
    let mut server = McpServer::with_catalog(
        RecordingGateway::new([Err(GatewayError::Timeout)]),
        ToolCatalog::runtime_v3_gameplay(),
    );
    let arguments = context_arguments(",\"operation_id\":\"operation-1\",\"wait_for_millis\":1000");
    let response = server.handle_frame(&call(WAIT_FOR_TRANSITION_TOOL, &arguments));
    assert!(response.contains("\\\"status\\\":\\\"unknown\\\""));
    assert!(response.contains("\\\"wait_outcome\\\":\\\"timeout\\\""));
    assert_eq!(server.gateway().requests.len(), 1);
}

#[test]
fn result_operation_identity_must_match_the_requested_operation() {
    let mut response = rejected_response();
    if let JsonValue::Object(object) = &mut response {
        object.insert(
            String::from("operation_id"),
            JsonValue::string("other-operation"),
        );
    }
    let mut server = McpServer::with_catalog(
        RecordingGateway::new([Ok(GatewayResponse {
            status: 409,
            body: response,
        })]),
        ToolCatalog::runtime_v3_gameplay(),
    );
    let arguments = context_arguments(
        ",\"state_id\":\"combat-1\",\"operation_id\":\"operation-1\",\
         \"action\":{\"action_id\":\"end-turn\",\"action\":{\"kind\":\"end_turn\"}}",
    );
    let output = server.handle_frame(&call(DISPATCH_ACTION_TOOL, &arguments));
    assert!(output.contains("\"isError\":true"));
    assert!(output.contains("unknown_after_invalid_response"));
    assert!(!output.contains("other-operation"));
}

#[path = "runtime_v3_gameplay_regressions/mod.rs"]
mod regressions;

fn fixture_identity(generation: i64) -> Vec<(String, JsonValue)> {
    vec![
        (
            String::from("protocol_version"),
            JsonValue::string("runtime-v3-gameplay"),
        ),
        (
            String::from("schema_digest"),
            JsonValue::string("b37c80f583aeaf4f81ede2083bcfb4129196baf5eb092470e8738173c4b7226c"),
        ),
        (
            String::from("provenance"),
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
            ]),
        ),
        (
            String::from("correlation_id"),
            JsonValue::string("request-1"),
        ),
        (String::from("instance_id"), JsonValue::string("instance-1")),
        (String::from("session_id"), JsonValue::string("session-1")),
        (String::from("lease_id"), JsonValue::string("lease-1")),
        (String::from("lease_epoch"), JsonValue::Number(1)),
        (String::from("generation"), JsonValue::Number(generation)),
    ]
}
