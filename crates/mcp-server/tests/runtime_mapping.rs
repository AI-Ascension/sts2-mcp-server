// SPDX-License-Identifier: MIT

use std::collections::VecDeque;

use sts2_mcp_server::{
    GatewayAdapter, GatewayError, GatewayMethod, GatewayRequest, GatewayResponse, JsonValue,
    McpServer, RUNTIME_ACTION_ID, RUNTIME_ARTIFACT, RUNTIME_GENERATOR, RUNTIME_PROTOCOL_VERSION,
    RUNTIME_SCHEMA_DIGEST, RUNTIME_SCHEMA_SOURCE, ToolCatalog,
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

fn state_call(id: &str, session: &str, instance: &str) -> String {
    format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":\"{id}\",\"method\":\"tools/call\",\
         \"params\":{{\"name\":\"get_state\",\"arguments\":{{\
         \"instance_id\":\"{instance}\",\"mcp_session_id\":\"{session}\"}}}}}}"
    )
}

fn action_call(id: &str, session: &str, instance: &str, generation: i64) -> String {
    format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":\"{id}\",\"method\":\"tools/call\",\
         \"params\":{{\"name\":\"submit_action\",\"arguments\":{{\
         \"instance_id\":\"{instance}\",\"mcp_session_id\":\"{session}\",\
         \"generation\":{generation},\"action_id\":\"{RUNTIME_ACTION_ID}\"}}}}}}"
    )
}

fn response(
    kind: &str,
    generation: i64,
    status: Option<&str>,
    error_code: Option<&str>,
) -> JsonValue {
    JsonValue::object([
        (
            "protocol_version".to_owned(),
            JsonValue::string(RUNTIME_PROTOCOL_VERSION),
        ),
        (
            "schema_digest".to_owned(),
            JsonValue::string(RUNTIME_SCHEMA_DIGEST),
        ),
        (
            "provenance".to_owned(),
            JsonValue::object([
                ("artifact".to_owned(), JsonValue::string(RUNTIME_ARTIFACT)),
                (
                    "source".to_owned(),
                    JsonValue::string(RUNTIME_SCHEMA_SOURCE),
                ),
                ("generator".to_owned(), JsonValue::string(RUNTIME_GENERATOR)),
            ]),
        ),
        ("correlation_id".to_owned(), JsonValue::string("request-1")),
        ("instance_id".to_owned(), JsonValue::string("instance-1")),
        ("session_id".to_owned(), JsonValue::string("session-1")),
        ("lease_id".to_owned(), JsonValue::string("lease-1")),
        ("lease_epoch".to_owned(), JsonValue::Number(1)),
        ("generation".to_owned(), JsonValue::Number(generation)),
        ("kind".to_owned(), JsonValue::string(kind)),
        (
            "observation".to_owned(),
            JsonValue::object([
                ("host_ready".to_owned(), JsonValue::Bool(true)),
                (
                    "overlay_visible".to_owned(),
                    JsonValue::Bool(generation > 0),
                ),
                ("screen".to_owned(), JsonValue::string("host")),
                ("action_count".to_owned(), JsonValue::Number(generation)),
            ]),
        ),
        (
            "action".to_owned(),
            if kind == "action_response" {
                JsonValue::object([("action_id".to_owned(), JsonValue::string(RUNTIME_ACTION_ID))])
            } else {
                JsonValue::Null
            },
        ),
        (
            "status".to_owned(),
            status.map_or(JsonValue::Null, JsonValue::string),
        ),
        (
            "error_code".to_owned(),
            error_code.map_or(JsonValue::Null, JsonValue::string),
        ),
        (
            "effect_witness".to_owned(),
            if status == Some("accepted") {
                JsonValue::object([
                    (
                        "kind".to_owned(),
                        JsonValue::string("status_overlay_visible"),
                    ),
                    ("generation".to_owned(), JsonValue::Number(generation)),
                ])
            } else {
                JsonValue::Null
            },
        ),
    ])
}

#[test]
fn runtime_state_mapping_uses_the_bounded_route() {
    let state = response("state_response", 0, None, None);
    let mut server = McpServer::with_catalog(
        RecordingGateway::new([Ok(GatewayResponse {
            status: 200,
            body: state,
        })]),
        ToolCatalog::runtime_v1(),
    );

    let result = server.handle_frame(&state_call("request-1", "session-1", "instance-1"));

    assert!(result.contains("\"isError\":false"));
    assert!(result.contains("host_ready"));
    assert!(result.contains("overlay_visible"));
    assert_eq!(server.gateway().requests.len(), 1);
    let request = &server.gateway().requests[0];
    assert_eq!(request.method, GatewayMethod::Get);
    assert_eq!(request.path, "/v1/instances/instance-1/state");
    assert!(request.body.is_none());
}

#[test]
fn runtime_action_mapping_preserves_witness_and_stale_rejection() {
    let accepted = response("action_response", 1, Some("accepted"), None);
    let stale = response(
        "action_response",
        1,
        Some("rejected"),
        Some("sts2.game-mod/stale_generation"),
    );
    let mut server = McpServer::with_catalog(
        RecordingGateway::new([
            Ok(GatewayResponse {
                status: 200,
                body: accepted,
            }),
            Ok(GatewayResponse {
                status: 409,
                body: stale,
            }),
        ]),
        ToolCatalog::runtime_v1(),
    );

    let accepted_result =
        server.handle_frame(&action_call("request-1", "session-1", "instance-1", 0));
    let stale_result = server.handle_frame(&action_call("request-2", "session-1", "instance-1", 0));

    assert!(accepted_result.contains("\"isError\":false"));
    assert!(accepted_result.contains("status_overlay_visible"));
    assert!(stale_result.contains("\"isError\":true"));
    assert!(stale_result.contains("sts2.game-mod/stale_generation"));
    assert_eq!(server.gateway().requests.len(), 2);
    let action_request = &server.gateway().requests[0];
    assert_eq!(action_request.method, GatewayMethod::Post);
    assert_eq!(action_request.path, "/v1/instances/instance-1/action");
    assert_eq!(
        action_request.body,
        Some(JsonValue::object([
            (
                "action".to_owned(),
                JsonValue::object([("action_id".to_owned(), JsonValue::string(RUNTIME_ACTION_ID))]),
            ),
            ("correlation_id".to_owned(), JsonValue::string("request-1")),
            ("effect_witness".to_owned(), JsonValue::Null),
            ("error_code".to_owned(), JsonValue::Null),
            ("generation".to_owned(), JsonValue::Number(0)),
            ("instance_id".to_owned(), JsonValue::string("instance-1")),
            ("kind".to_owned(), JsonValue::string("action_request")),
            ("lease_epoch".to_owned(), JsonValue::Number(0)),
            ("lease_id".to_owned(), JsonValue::string("lease-pending")),
            ("observation".to_owned(), JsonValue::Null),
            (
                "protocol_version".to_owned(),
                JsonValue::string(RUNTIME_PROTOCOL_VERSION)
            ),
            (
                "provenance".to_owned(),
                JsonValue::object([
                    ("artifact".to_owned(), JsonValue::string(RUNTIME_ARTIFACT)),
                    ("generator".to_owned(), JsonValue::string(RUNTIME_GENERATOR)),
                    (
                        "source".to_owned(),
                        JsonValue::string(RUNTIME_SCHEMA_SOURCE)
                    ),
                ]),
            ),
            (
                "schema_digest".to_owned(),
                JsonValue::string(RUNTIME_SCHEMA_DIGEST)
            ),
            ("session_id".to_owned(), JsonValue::string("session-1")),
            ("status".to_owned(), JsonValue::Null),
        ]))
    );
}

#[test]
fn runtime_projection_rejects_schema_digest_drift() {
    let mut state = response("state_response", 0, None, None);
    if let JsonValue::Object(object) = &mut state {
        object.insert(
            String::from("schema_digest"),
            JsonValue::string("0000000000000000000000000000000000000000000000000000000000000000"),
        );
    }
    let mut server = McpServer::with_catalog(
        RecordingGateway::new([Ok(GatewayResponse {
            status: 200,
            body: state,
        })]),
        ToolCatalog::runtime_v1(),
    );

    let result = server.handle_frame(&state_call("request-1", "session-1", "instance-1"));

    assert!(result.contains("\"isError\":true"));
    assert!(result.contains("allowlisted state or error projection"));
}
