// SPDX-License-Identifier: MIT
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::VecDeque;

use sts2_mcp_server::{
    GatewayAdapter, GatewayError, GatewayMethod, GatewayRequest, GatewayResponse, JsonValue,
    McpServer, ToolCatalog, parse_json,
};

const FRAMES: &str = include_str!("../../../protocol-artifact/exact-restore-v1/golden/frames.json");
const NEUTRAL_SCHEMA: &str =
    include_str!("../../../protocol-artifact/exact-restore-v1/schema.json");

struct RecordingGateway {
    requests: Vec<GatewayRequest>,
    responses: VecDeque<GatewayResponse>,
}

impl GatewayAdapter for RecordingGateway {
    fn forward(&mut self, request: GatewayRequest) -> Result<GatewayResponse, GatewayError> {
        self.requests.push(request);
        self.responses.pop_front().ok_or(GatewayError::Unavailable)
    }
}

fn server(responses: Vec<(u16, JsonValue)>) -> McpServer<RecordingGateway> {
    let gateway = RecordingGateway {
        requests: Vec::new(),
        responses: responses
            .into_iter()
            .map(|(status, body)| GatewayResponse { status, body })
            .collect(),
    };
    McpServer::with_catalog_and_sessions(
        gateway,
        ToolCatalog::exact_restore_v1(),
        "session-example",
        "mcp-session-1",
    )
}

fn frames() -> Vec<serde_json::Value> {
    serde_json::from_str::<serde_json::Value>(FRAMES).unwrap()["frames"]
        .as_array()
        .unwrap()
        .clone()
}

fn wrapper(frame: serde_json::Value, request: bool) -> JsonValue {
    let message_id = JsonValue::string(frame["message_id"].as_str().unwrap());
    let correlation_id = JsonValue::string(frame["correlation_id"].as_str().unwrap());
    let role = if request { "harness" } else { "gateway" };
    JsonValue::object([
        (
            String::from("contract"),
            JsonValue::string("sts2-exact-restore-gateway-v1"),
        ),
        (
            String::from("schema_digest"),
            JsonValue::string(sts2_mcp_server::EXACT_RESTORE_GATEWAY_SCHEMA_DIGEST),
        ),
        (String::from("message_id"), message_id),
        (String::from("correlation_id"), correlation_id),
        (
            String::from("actor"),
            JsonValue::object([
                (String::from("principal_id"), JsonValue::string("harness")),
                (String::from("role"), JsonValue::string(role)),
            ]),
        ),
        (
            String::from("auth"),
            JsonValue::object([
                (String::from("principal_id"), JsonValue::string("harness")),
                (
                    String::from("capability"),
                    JsonValue::string("exact_restore"),
                ),
                (String::from("proof"), JsonValue::Null),
            ]),
        ),
        (
            String::from("kind"),
            JsonValue::string(if request {
                "exact_restore_request"
            } else {
                "exact_restore_response"
            }),
        ),
        (
            String::from("payload"),
            JsonValue::object([(
                String::from("frame"),
                parse_json(&frame.to_string()).unwrap(),
            )]),
        ),
    ])
}

fn call(tool: &str, envelope: JsonValue, id: usize) -> String {
    let arguments: serde_json::Value =
        serde_json::from_str(&envelope.to_json()).expect("wrapper is JSON");
    serde_json::json!({
        "jsonrpc":"2.0",
        "id":id,
        "method":"tools/call",
        "params":{"name":tool,"arguments":arguments}
    })
    .to_string()
}

#[test]
fn serialized_mcp_calls_reach_all_five_fixed_gateway_routes() {
    let neutral: serde_json::Value = serde_json::from_str(NEUTRAL_SCHEMA).unwrap();
    let validator = jsonschema::draft202012::options().build(&neutral).unwrap();
    let frames = frames();
    let phases = [
        (0, 12, "sts2.exact_restore.begin", "begin"),
        (2, 14, "sts2.exact_restore.put_chunk", "chunk"),
        (3, 15, "sts2.exact_restore.finish_blob", "finish"),
        (4, 16, "sts2.exact_restore.commit", "commit"),
        (6, 18, "sts2.exact_restore.lookup", "lookup"),
    ];
    let mut server = server(
        phases
            .iter()
            .map(|(_, response, _, _)| (200, wrapper(frames[*response].clone(), false)))
            .collect(),
    );
    for (index, (request_index, response_index, tool, _)) in phases.into_iter().enumerate() {
        let request = wrapper(frames[request_index].clone(), true);
        let result: serde_json::Value =
            serde_json::from_str(&server.handle_frame(&call(tool, request.clone(), index + 1)))
                .unwrap();
        assert_eq!(result["result"]["isError"], false);
        let output: serde_json::Value = serde_json::from_str(
            result["result"]["content"][0]["text"]
                .as_str()
                .expect("text result"),
        )
        .unwrap();
        let expected = frames[response_index].clone();
        assert_eq!(output, expected);
        assert!(validator.is_valid(&output));
    }
    let requests = &server.gateway().requests;
    assert_eq!(requests.len(), 5);
    for (request, (request_index, _, _, route)) in requests.iter().zip(phases) {
        assert_eq!(request.method, GatewayMethod::Post);
        assert_eq!(request.path, format!("/v1/exact-restore/{route}"));
        assert_eq!(
            request.body,
            Some(wrapper(frames[request_index].clone(), true))
        );
        assert_eq!(
            request.headers.get("x-mcp-session-id").map(String::as_str),
            Some("mcp-session-1")
        );
        assert_eq!(
            request
                .headers
                .get("x-sts2-correlation-id")
                .map(String::as_str),
            frames[request_index]["correlation_id"].as_str()
        );
        assert_eq!(
            request
                .headers
                .get("x-sts2-lease-epoch")
                .map(String::as_str),
            Some("8")
        );
    }
}

#[test]
fn unsupported_adapter_refusal_stays_a_typed_begin_error_before_upload() {
    let frames = frames();
    let begin = wrapper(frames[9].clone(), true);
    let refusal = wrapper(frames[22].clone(), false);
    let mut server = server(vec![(422, refusal)]);
    let result: serde_json::Value =
        serde_json::from_str(&server.handle_frame(&call("sts2.exact_restore.begin", begin, 1)))
            .unwrap();
    assert_eq!(result["result"]["isError"], true);
    assert_eq!(server.gateway().requests.len(), 1);
    assert_eq!(server.gateway().requests[0].path, "/v1/exact-restore/begin");
    let output: serde_json::Value = serde_json::from_str(
        result["result"]["content"][0]["text"]
            .as_str()
            .expect("typed refusal"),
    )
    .unwrap();
    assert_eq!(output["payload"]["error_code"], "no_restore_adapter");
}

#[test]
fn invalid_mcp_envelopes_never_reach_the_gateway() {
    let frame = frames()[0].clone();
    let mut envelope = wrapper(frame, true);
    if let JsonValue::Object(object) = &mut envelope {
        object.insert(String::from("unrecognized"), JsonValue::Bool(true));
    }
    let mut server = server(Vec::new());
    let result: serde_json::Value =
        serde_json::from_str(&server.handle_frame(&call("sts2.exact_restore.begin", envelope, 1)))
            .unwrap();
    assert_eq!(result["error"]["code"], -32602);
    assert!(server.gateway().requests.is_empty());
}
