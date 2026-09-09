// SPDX-License-Identifier: MIT

use std::collections::VecDeque;

use sts2_mcp_server::{
    GatewayAdapter, GatewayRequest, GatewayResponse, JsonValue, McpServer, ToolCatalog, parse_json,
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

fn response_fixture(path: &str, operation_id: &str) -> JsonValue {
    let text = match path {
        "unknown" => include_str!(
            "../../../protocol-artifact/runtime-v4-expert-rest-action/golden/action-unknown.json"
        ),
        "smith-requested" => include_str!(
            "../../../protocol-artifact/runtime-v4-expert-rest-action/golden/action-selection-requested.json"
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
    ] {
        object.insert(field.to_owned(), JsonValue::string(replacement));
    }
    object.insert("operation_id".to_owned(), JsonValue::string(operation_id));
    value
}

fn selector_action(index: usize) -> JsonValue {
    let parsed = parse_json(&format!(
        r#"{{"action_id":"rest-option:9:smith:{index}","action":{{"kind":"rest_option","rest_option_id":"smith"}}}}"#
    ));
    assert!(parsed.is_ok(), "selector action JSON");
    parsed.unwrap_or(JsonValue::Null)
}

fn unknown_selector_response(operation_id: &str, action: &JsonValue) -> JsonValue {
    let mut value = response_fixture("unknown", operation_id);
    let JsonValue::Object(object) = &mut value else {
        return JsonValue::Null;
    };
    object.insert("generation".to_owned(), JsonValue::Number(9));
    object.insert("state_id".to_owned(), JsonValue::string("live:9"));
    object.insert("action".to_owned(), action.clone());
    value
}

fn call(tool: &str, arguments: &str) -> String {
    format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":\"request-1\",\"method\":\"tools/call\",\"params\":{{\"name\":\"{tool}\",\"arguments\":{{{arguments}}}}}}}"
    )
}

#[test]
fn unresolved_selector_operations_reserve_capacity_before_forwarding() {
    let mut responses = VecDeque::new();
    for index in 0..128 {
        let action = selector_action(index);
        responses.push_back(GatewayResponse {
            status: 502,
            body: unknown_selector_response(&format!("pending-selector-{index}"), &action),
        });
    }
    let mut recovered_response = response_fixture("smith-requested", "pending-selector-0");
    if let JsonValue::Object(object) = &mut recovered_response {
        object.insert("action".to_owned(), selector_action(0));
    }
    responses.push_back(GatewayResponse {
        status: 200,
        body: recovered_response,
    });
    let gateway = RecordingGateway {
        requests: Vec::new(),
        responses,
    };
    let mut server = McpServer::with_catalog_and_sessions(
        gateway,
        ToolCatalog::runtime_v4_expert_rest_action(),
        "session-1",
        "mcp-session-1",
    );

    for index in 0..128 {
        let action_json = selector_action(index).to_json();
        let request = call(
            "sts2.expert_rest_action",
            &format!(
                "\"instance_id\":\"instance-1\",\"mcp_session_id\":\"mcp-session-1\",\"lease_id\":\"lease-1\",\"lease_epoch\":4,\"generation\":9,\"state_id\":\"live:9\",\"operation_id\":\"pending-selector-{index}\",\"action\":{action_json}"
            ),
        );
        let output = server.handle_frame(&request);
        assert!(
            output.contains("\\\"status\\\":\\\"unknown\\\""),
            "{output}"
        );
    }

    let action_json = selector_action(128).to_json();
    let blocked = call(
        "sts2.expert_rest_action",
        &format!(
            "\"instance_id\":\"instance-1\",\"mcp_session_id\":\"mcp-session-1\",\"lease_id\":\"lease-1\",\"lease_epoch\":4,\"generation\":9,\"state_id\":\"live:9\",\"operation_id\":\"pending-selector-128\",\"action\":{action_json}"
        ),
    );
    let blocked_output = server.handle_frame(&blocked);
    assert!(
        blocked_output.contains("selector admission capacity is exhausted"),
        "{blocked_output}"
    );
    assert_eq!(server.gateway().requests.len(), 128);

    let reconcile = call(
        "sts2.expert_rest_reconcile",
        "\"instance_id\":\"instance-1\",\"mcp_session_id\":\"mcp-session-1\",\"lease_id\":\"lease-1\",\"lease_epoch\":4,\"operation_id\":\"pending-selector-0\"",
    );
    let recovered = server.handle_frame(&reconcile);
    assert!(recovered.contains("\"isError\":false"), "{recovered}");
    assert_eq!(server.gateway().requests.len(), 129);

    let still_blocked = server.handle_frame(&blocked);
    assert!(
        still_blocked.contains("selector admission capacity is exhausted"),
        "{still_blocked}"
    );
    assert_eq!(server.gateway().requests.len(), 129);
}
