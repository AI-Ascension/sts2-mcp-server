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

fn response_fixture(status: &str) -> JsonValue {
    let Ok(mut value) = parse_json(include_str!(
        "../../../protocol-artifact/runtime-v4-expert-rest-action/golden/action-unknown.json"
    )) else {
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
    object.insert("status".to_owned(), JsonValue::string(status));
    value
}

fn response_fixture_with_binding_forgery() -> JsonValue {
    let mut value = response_fixture("unknown");
    let JsonValue::Object(object) = &mut value else {
        return JsonValue::Null;
    };
    object.insert("generation".to_owned(), JsonValue::Number(8));
    let Ok(forged_action) = parse_json(
        r#"{"action_id":"rest-option:8:heal","action":{"kind":"rest_option","rest_option_id":"heal"}}"#,
    ) else {
        return JsonValue::Null;
    };
    object.insert("action".to_owned(), forged_action);
    value
}

fn call(tool: &str, arguments: &str) -> String {
    format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":\"request-1\",\"method\":\"tools/call\",\"params\":{{\"name\":\"{tool}\",\"arguments\":{{{arguments}}}}}}}"
    )
}

#[test]
fn unknown_and_cancelled_statuses_preserve_typed_receipts() {
    for (http_status, receipt_status) in [(502, "unknown"), (504, "unknown"), (499, "cancelled")] {
        let gateway = RecordingGateway {
            requests: Vec::new(),
            responses: VecDeque::from([GatewayResponse {
                status: http_status,
                body: response_fixture(receipt_status),
            }]),
        };
        let mut server = McpServer::with_catalog_and_sessions(
            gateway,
            ToolCatalog::runtime_v4_expert_rest_action(),
            "session-1",
            "mcp-session-1",
        );
        let request = call(
            "sts2.expert_rest_action",
            r#""instance_id":"instance-1","mcp_session_id":"mcp-session-1","lease_id":"lease-1","lease_epoch":4,"generation":7,"state_id":"live-7","operation_id":"operation-1","action":{"action_id":"rest-option:7:heal","action":{"kind":"rest_option","rest_option_id":"heal"}}"#,
        );
        let output = server.handle_frame(&request);
        assert!(
            output.contains("\"isError\":true"),
            "{http_status}: {output}"
        );
        assert!(
            output.contains(&format!("\\\"status\\\":\\\"{receipt_status}\\\"")),
            "{http_status}: {output}"
        );
        assert!(output.contains("sts2.game-mod/outcome_unknown"));
    }

    let gateway = RecordingGateway {
        requests: Vec::new(),
        responses: VecDeque::from([GatewayResponse {
            status: 502,
            body: response_fixture("unknown"),
        }]),
    };
    let mut server = McpServer::with_catalog_and_sessions(
        gateway,
        ToolCatalog::runtime_v4_expert_rest_action(),
        "session-1",
        "mcp-session-1",
    );
    let reconcile = call(
        "sts2.expert_rest_reconcile",
        "\"instance_id\":\"instance-1\",\"mcp_session_id\":\"mcp-session-1\",\"lease_id\":\"lease-1\",\"lease_epoch\":4,\"operation_id\":\"operation-1\"",
    );
    let output = server.handle_frame(&reconcile);
    assert!(output.contains("\"isError\":true"), "{output}");
    assert!(
        output.contains("\\\"status\\\":\\\"unknown\\\""),
        "{output}"
    );
    assert!(output.contains("sts2.game-mod/outcome_unknown"));
    assert_eq!(server.gateway().requests[0].method, GatewayMethod::Get);
    assert_eq!(
        server.gateway().requests[0].path,
        "/v4/instances/instance-1/expert-rest-actions/operation-1"
    );
}

#[test]
fn reconcile_rejects_a_same_operation_forged_action_and_generation() {
    let gateway = RecordingGateway {
        requests: Vec::new(),
        responses: VecDeque::from([
            GatewayResponse {
                status: 502,
                body: response_fixture("unknown"),
            },
            GatewayResponse {
                status: 502,
                body: response_fixture_with_binding_forgery(),
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
    let dispatch_output = server.handle_frame(&dispatch);
    assert!(
        dispatch_output.contains("\"isError\":true"),
        "{dispatch_output}"
    );
    assert!(
        dispatch_output.contains("\\\"status\\\":\\\"unknown\\\""),
        "{dispatch_output}"
    );
    let reconcile = call(
        "sts2.expert_rest_reconcile",
        "\"instance_id\":\"instance-1\",\"mcp_session_id\":\"mcp-session-1\",\"lease_id\":\"lease-1\",\"lease_epoch\":4,\"operation_id\":\"operation-1\"",
    );
    let reconcile_output = server.handle_frame(&reconcile);
    assert!(
        reconcile_output.contains("\"isError\":true"),
        "{reconcile_output}"
    );
    assert!(
        !reconcile_output.contains("\\\"status\\\":\\\"unknown\\\""),
        "{reconcile_output}"
    );
    assert_eq!(server.gateway().requests.len(), 2);
}

#[test]
fn dispatch_rejects_rebinding_an_operation_after_transport_failure() {
    let gateway = RecordingGateway {
        requests: Vec::new(),
        responses: VecDeque::new(),
    };
    let mut server = McpServer::with_catalog_and_sessions(
        gateway,
        ToolCatalog::runtime_v4_expert_rest_action(),
        "session-1",
        "mcp-session-1",
    );
    let first = call(
        "sts2.expert_rest_action",
        r#""instance_id":"instance-1","mcp_session_id":"mcp-session-1","lease_id":"lease-1","lease_epoch":4,"generation":7,"state_id":"live-7","operation_id":"operation-1","action":{"action_id":"rest-option:7:heal","action":{"kind":"rest_option","rest_option_id":"heal"}}"#,
    );
    assert!(server.handle_frame(&first).contains("-32003"));
    let second = call(
        "sts2.expert_rest_action",
        r#""instance_id":"instance-1","mcp_session_id":"mcp-session-1","lease_id":"lease-1","lease_epoch":4,"generation":8,"state_id":"live-8","operation_id":"operation-1","action":{"action_id":"rest-option:8:heal","action":{"kind":"rest_option","rest_option_id":"heal"}}"#,
    );
    assert!(server.handle_frame(&second).contains("-32602"));
    assert_eq!(server.gateway().requests.len(), 1);
}
