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
        "accepted" => include_str!(
            "../../../protocol-artifact/runtime-v4-expert-rest-action/golden/action-accepted.json"
        ),
        "unknown" => include_str!(
            "../../../protocol-artifact/runtime-v4-expert-rest-action/golden/action-unknown.json"
        ),
        "smith-requested" => include_str!(
            "../../../protocol-artifact/runtime-v4-expert-rest-action/golden/action-selection-requested.json"
        ),
        "smith-first" => include_str!(
            "../../../protocol-artifact/runtime-v4-expert-rest-action/golden/action-selection-progressed.json"
        ),
        "smith-second" => include_str!(
            "../../../protocol-artifact/runtime-v4-expert-rest-action/golden/action-selection-second-progressed.json"
        ),
        "smith-completed" => include_str!(
            "../../../protocol-artifact/runtime-v4-expert-rest-action/golden/action-selection-completed.json"
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
    replace_operation_ids(&mut value, operation_id);
    value
}

fn replace_operation_ids(value: &mut JsonValue, operation_id: &str) {
    match value {
        JsonValue::Object(object) => {
            if object.contains_key("operation_id") {
                object.insert(
                    String::from("operation_id"),
                    JsonValue::string(operation_id),
                );
            }
            for value in object.values_mut() {
                replace_operation_ids(value, operation_id);
            }
        }
        JsonValue::Array(values) => {
            for value in values {
                replace_operation_ids(value, operation_id);
            }
        }
        _ => {}
    }
}

fn replace_selection_id(value: &mut JsonValue, selection_id: &str) {
    match value {
        JsonValue::String(current) if current == "selection:10:smith" => {
            *current = selection_id.to_owned();
        }
        JsonValue::Object(object) => {
            for value in object.values_mut() {
                replace_selection_id(value, selection_id);
            }
        }
        JsonValue::Array(values) => {
            for value in values {
                replace_selection_id(value, selection_id);
            }
        }
        _ => {}
    }
}

fn fresh_smith_requested(index: usize, operation_id: &str) -> JsonValue {
    let mut value = response_fixture("smith-requested", operation_id);
    replace_selection_id(&mut value, &format!("selection:fresh:{index}:smith"));
    if let JsonValue::Object(object) = &mut value {
        object.insert("action".to_owned(), selector_action(index));
    }
    value
}

fn smith_card_action(card_id: &str, action_id: &str) -> JsonValue {
    let parsed = parse_json(&format!(
        r#"{{"action_id":"{action_id}","action":{{"kind":"select_card","selection_id":"selection:10:smith","rest_option_id":"smith","card_id":"{card_id}"}}}}"#
    ));
    assert!(parsed.is_ok(), "Smith card action JSON");
    parsed.unwrap_or(JsonValue::Null)
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

#[test]
fn late_progress_after_terminal_completion_does_not_reactivate_evicted_selector() {
    let mut responses = VecDeque::from([
        GatewayResponse {
            status: 200,
            body: response_fixture("smith-requested", "smith-open"),
        },
        GatewayResponse {
            status: 202,
            body: {
                let mut value = response_fixture("accepted", "smith-card-one");
                if let JsonValue::Object(object) = &mut value {
                    object.insert("generation".to_owned(), JsonValue::Number(10));
                    object.insert("state_id".to_owned(), JsonValue::string("live:10"));
                    object.insert(
                        "action".to_owned(),
                        smith_card_action("card:1", "select_card:10:smith:card:1"),
                    );
                }
                value
            },
        },
        GatewayResponse {
            status: 200,
            body: response_fixture("smith-first", "smith-card-one"),
        },
        GatewayResponse {
            status: 200,
            body: response_fixture("smith-second", "smith-card-two"),
        },
        GatewayResponse {
            status: 200,
            body: response_fixture("smith-completed", "smith-confirm"),
        },
    ]);
    for index in 0..128 {
        responses.push_back(GatewayResponse {
            status: 200,
            body: fresh_smith_requested(index, &format!("evict-selector-{index}")),
        });
    }
    responses.push_back(GatewayResponse {
        status: 200,
        body: response_fixture("smith-first", "smith-card-one"),
    });
    for index in 0..129 {
        responses.push_back(GatewayResponse {
            status: 200,
            body: fresh_smith_requested(index + 128, &format!("post-late-selector-{index}")),
        });
    }
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

    let open = call(
        "sts2.expert_rest_action",
        r#""instance_id":"instance-1","mcp_session_id":"mcp-session-1","lease_id":"lease-1","lease_epoch":4,"generation":9,"state_id":"live:9","operation_id":"smith-open","action":{"action_id":"rest-option:9:smith","action":{"kind":"rest_option","rest_option_id":"smith"}}"#,
    );
    assert!(server.handle_frame(&open).contains("\"isError\":false"));
    let card_one = call(
        "sts2.expert_rest_action",
        r#""instance_id":"instance-1","mcp_session_id":"mcp-session-1","lease_id":"lease-1","lease_epoch":4,"generation":10,"state_id":"live:10","operation_id":"smith-card-one","action":{"action_id":"select_card:10:smith:card:1","action":{"kind":"select_card","selection_id":"selection:10:smith","rest_option_id":"smith","card_id":"card:1"}}"#,
    );
    assert!(server.handle_frame(&card_one).contains("\"isError\":false"));
    let reconcile = call(
        "sts2.expert_rest_reconcile",
        r#""instance_id":"instance-1","mcp_session_id":"mcp-session-1","lease_id":"lease-1","lease_epoch":4,"operation_id":"smith-card-one""#,
    );
    assert!(
        server
            .handle_frame(&reconcile)
            .contains("\"isError\":false")
    );
    let card_two = call(
        "sts2.expert_rest_action",
        r#""instance_id":"instance-1","mcp_session_id":"mcp-session-1","lease_id":"lease-1","lease_epoch":4,"generation":11,"state_id":"live:11","operation_id":"smith-card-two","action":{"action_id":"select_card:11:smith:card:2","action":{"kind":"select_card","selection_id":"selection:10:smith","rest_option_id":"smith","card_id":"card:2"}}"#,
    );
    assert!(server.handle_frame(&card_two).contains("\"isError\":false"));
    let confirm = call(
        "sts2.expert_rest_action",
        r#""instance_id":"instance-1","mcp_session_id":"mcp-session-1","lease_id":"lease-1","lease_epoch":4,"generation":12,"state_id":"live:12","operation_id":"smith-confirm","action":{"action_id":"confirm_selection:12:smith","action":{"kind":"confirm_selection","selection_id":"selection:10:smith","rest_option_id":"smith"}}"#,
    );
    assert!(server.handle_frame(&confirm).contains("\"isError\":false"));

    for index in 0..128 {
        let action = selector_action(index);
        let request = call(
            "sts2.expert_rest_action",
            &format!(
                "\"instance_id\":\"instance-1\",\"mcp_session_id\":\"mcp-session-1\",\"lease_id\":\"lease-1\",\"lease_epoch\":4,\"generation\":9,\"state_id\":\"live:9\",\"operation_id\":\"evict-selector-{index}\",\"action\":{}",
                action.to_json()
            ),
        );
        assert!(server.handle_frame(&request).contains("\"isError\":false"));
    }

    let late_get = call(
        "sts2.expert_rest_reconcile",
        r#""instance_id":"instance-1","mcp_session_id":"mcp-session-1","lease_id":"lease-1","lease_epoch":4,"operation_id":"smith-card-one""#,
    );
    let late_output = server.handle_frame(&late_get);
    assert!(late_output.contains("\"isError\":false"), "{late_output}");
    assert!(late_output.contains("smith-card-one"), "{late_output}");
    assert!(late_output.contains("card:1"), "{late_output}");

    for index in 0..129 {
        let action = selector_action(index + 128);
        let request = call(
            "sts2.expert_rest_action",
            &format!(
                "\"instance_id\":\"instance-1\",\"mcp_session_id\":\"mcp-session-1\",\"lease_id\":\"lease-1\",\"lease_epoch\":4,\"generation\":9,\"state_id\":\"live:9\",\"operation_id\":\"post-late-selector-{index}\",\"action\":{}",
                action.to_json()
            ),
        );
        let output = server.handle_frame(&request);
        assert!(
            output.contains("selector admission capacity is exhausted"),
            "{output}"
        );
    }
    assert_eq!(server.gateway().requests.len(), 134);
}
