// SPDX-License-Identifier: MIT

use std::collections::VecDeque;

use sts2_mcp_server::{
    GatewayAdapter, GatewayMethod, GatewayRequest, GatewayResponse, JsonValue, McpServer,
    ToolCatalog, parse_json,
};

struct RecordingGateway {
    requests: Vec<GatewayRequest>,
    responses: VecDeque<GatewayResponse>,
}

impl GatewayAdapter for RecordingGateway {
    fn forward(
        &mut self,
        request: GatewayRequest,
    ) -> Result<GatewayResponse, sts2_mcp_server::GatewayError> {
        self.requests.push(request);
        self.responses
            .pop_front()
            .ok_or(sts2_mcp_server::GatewayError::Unavailable)
    }
}

fn response_fixture(path: &str) -> JsonValue {
    let text = match path {
        "accepted" => include_str!(
            "../../../protocol-artifact/runtime-v4-expert-rest-action/golden/action-accepted.json"
        ),
        "settled" => include_str!(
            "../../../protocol-artifact/runtime-v4-expert-rest-action/golden/action-completed.json"
        ),
        _ => unreachable!(),
    };
    let parsed = parse_json(text);
    assert!(parsed.is_ok(), "fixture JSON");
    let Ok(mut value) = parsed else {
        return JsonValue::Null;
    };
    let JsonValue::Object(object) = &mut value else {
        return JsonValue::Null;
    };
    for (field, replacement) in [
        ("correlation_id", "request-1"),
        ("instance_id", "instance-1"),
        ("session_id", "session-1"),
        ("lease_id", "lease-1"),
        ("operation_id", "operation-1"),
    ] {
        object.insert(field.to_owned(), JsonValue::string(replacement));
    }
    fn replace_operation_ids(value: &mut JsonValue) {
        match value {
            JsonValue::Object(object) => {
                if object.contains_key("operation_id") {
                    object.insert(
                        String::from("operation_id"),
                        JsonValue::string("operation-1"),
                    );
                }
                for value in object.values_mut() {
                    replace_operation_ids(value);
                }
            }
            JsonValue::Array(values) => {
                for value in values {
                    replace_operation_ids(value);
                }
            }
            _ => {}
        }
    }
    replace_operation_ids(&mut value);
    value
}

fn call(tool: &str, arguments: &str) -> String {
    format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":\"request-1\",\"method\":\"tools/call\",\"params\":{{\"name\":\"{tool}\",\"arguments\":{{{arguments}}}}}}}"
    )
}

#[test]
fn dispatch_and_reconcile_preserve_typed_rest_identity_and_fences() {
    let gateway = RecordingGateway {
        requests: Vec::new(),
        responses: VecDeque::from([
            GatewayResponse {
                status: 202,
                body: response_fixture("accepted"),
            },
            GatewayResponse {
                status: 200,
                body: response_fixture("settled"),
            },
        ]),
    };
    let mut server = McpServer::with_catalog_and_sessions(
        gateway,
        ToolCatalog::runtime_v4_expert_rest_action(),
        "session-1",
        "mcp-session-1",
    );
    let action = r#"{"action_id":"rest-option:7:heal","action":{"kind":"rest_option","rest_option_id":"heal"}}"#;
    let dispatch = call(
        "sts2.expert_rest_action",
        &format!(
            "\"instance_id\":\"instance-1\",\"mcp_session_id\":\"mcp-session-1\",\"lease_id\":\"lease-1\",\"lease_epoch\":4,\"generation\":7,\"state_id\":\"live-7\",\"operation_id\":\"operation-1\",\"action\":{action}"
        ),
    );
    assert!(server.handle_frame(&dispatch).contains("\"isError\":false"));
    let reconcile = call(
        "sts2.expert_rest_reconcile",
        "\"instance_id\":\"instance-1\",\"mcp_session_id\":\"mcp-session-1\",\"lease_id\":\"lease-1\",\"lease_epoch\":4,\"operation_id\":\"operation-1\"",
    );
    assert!(
        server
            .handle_frame(&reconcile)
            .contains("\"isError\":false")
    );

    let requests = &server.gateway().requests;
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].method, GatewayMethod::Post);
    assert_eq!(
        requests[0].path,
        "/v4/instances/instance-1/expert-rest-action"
    );
    assert_eq!(requests[1].method, GatewayMethod::Get);
    assert_eq!(
        requests[1].path,
        "/v4/instances/instance-1/expert-rest-actions/operation-1"
    );
    for request in requests {
        assert_eq!(
            request.headers.get("x-mcp-session-id").map(String::as_str),
            Some("mcp-session-1")
        );
        assert_eq!(
            request.headers.get("x-sts2-session-id").map(String::as_str),
            Some("session-1")
        );
        assert_eq!(
            request
                .headers
                .get("x-sts2-lease-epoch")
                .map(String::as_str),
            Some("4")
        );
    }
    let body = requests[0].body.as_ref();
    assert!(matches!(body, Some(JsonValue::Object(_))), "dispatch body");
    if let Some(JsonValue::Object(body)) = body {
        assert_eq!(
            body.get("protocol_version"),
            Some(&JsonValue::string("runtime-v4-expert-rest-action-v1"))
        );
        assert_eq!(body.get("kind"), Some(&JsonValue::string("action_request")));
        assert_eq!(body.get("generation"), Some(&JsonValue::Number(7)));
        assert_eq!(body.get("state_id"), Some(&JsonValue::string("live-7")));
    }
}

#[test]
fn malformed_or_foreign_typed_actions_are_rejected_before_gateway() {
    let gateway = RecordingGateway {
        requests: Vec::new(),
        responses: VecDeque::new(),
    };
    let mut server = McpServer::with_catalog(gateway, ToolCatalog::runtime_v4_expert_rest_action());
    let output = server.handle_frame(&call(
        "sts2.expert_rest_action",
        "\"instance_id\":\"instance-1\",\"mcp_session_id\":\"mcp-session-1\",\"lease_id\":\"lease-1\",\"lease_epoch\":1,\"generation\":7,\"state_id\":\"live-7\",\"operation_id\":\"operation-1\",\"action\":{\"action_id\":\"action-1\",\"action\":{\"kind\":\"select_card\",\"selection_id\":\"selection-1\",\"rest_option_id\":\"mend\",\"card_id\":\"card-1\"}}",
    ));
    assert!(output.contains("-32602"), "{output}");
    assert!(server.gateway().requests.is_empty());
}
