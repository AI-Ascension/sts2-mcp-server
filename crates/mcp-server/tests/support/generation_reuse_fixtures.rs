// SPDX-License-Identifier: MIT

use std::collections::VecDeque;

use sts2_mcp_server::{GatewayAdapter, GatewayRequest, GatewayResponse, JsonValue, parse_json};

pub(crate) struct RecordingGateway {
    pub(crate) requests: Vec<GatewayRequest>,
    pub(crate) responses: VecDeque<GatewayResponse>,
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

pub(crate) fn response_fixture(path: &str, operation_id: &str) -> JsonValue {
    let text = match path {
        "accepted" => include_str!(
            "../../../../protocol-artifact/runtime-v4-expert-rest-action/golden/action-accepted.json"
        ),
        "smith-requested" => include_str!(
            "../../../../protocol-artifact/runtime-v4-expert-rest-action/golden/action-selection-requested.json"
        ),
        "smith-first" => include_str!(
            "../../../../protocol-artifact/runtime-v4-expert-rest-action/golden/action-selection-progressed.json"
        ),
        "smith-second" => include_str!(
            "../../../../protocol-artifact/runtime-v4-expert-rest-action/golden/action-selection-second-progressed.json"
        ),
        "smith-completed" => include_str!(
            "../../../../protocol-artifact/runtime-v4-expert-rest-action/golden/action-selection-completed.json"
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

pub(crate) fn replace_operation_ids(value: &mut JsonValue, operation_id: &str) {
    match value {
        JsonValue::Object(object) => {
            if object.contains_key("operation_id") {
                object.insert("operation_id".to_owned(), JsonValue::string(operation_id));
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

pub(crate) fn replace_string(value: &mut JsonValue, from: &str, to: &str) {
    match value {
        JsonValue::String(current) if current == from => *current = to.to_owned(),
        JsonValue::Object(object) => {
            for value in object.values_mut() {
                replace_string(value, from, to);
            }
        }
        JsonValue::Array(values) => {
            for value in values {
                replace_string(value, from, to);
            }
        }
        _ => {}
    }
}

pub(crate) fn replace_number(value: &mut JsonValue, from: i64, to: i64) {
    match value {
        JsonValue::Number(current) if *current == from => *current = to,
        JsonValue::Object(object) => {
            for value in object.values_mut() {
                replace_number(value, from, to);
            }
        }
        JsonValue::Array(values) => {
            for value in values {
                replace_number(value, from, to);
            }
        }
        _ => {}
    }
}

pub(crate) fn selector_action(index: usize) -> JsonValue {
    let parsed = parse_json(&format!(
        r#"{{"action_id":"rest-option:9:smith:{index}","action":{{"kind":"rest_option","rest_option_id":"smith"}}}}"#
    ));
    assert!(parsed.is_ok(), "selector action JSON");
    parsed.unwrap_or(JsonValue::Null)
}

pub(crate) fn fresh_smith_requested(index: usize, operation_id: &str) -> JsonValue {
    let mut value = response_fixture("smith-requested", operation_id);
    replace_string(
        &mut value,
        "selection:10:smith",
        &format!("selection:fresh:{index}:smith"),
    );
    if let JsonValue::Object(object) = &mut value {
        object.insert("action".to_owned(), selector_action(index));
    }
    value
}

pub(crate) fn accepted_for_action(operation_id: &str, action: &str, generation: i64) -> JsonValue {
    let mut value = response_fixture("accepted", operation_id);
    let parsed = parse_json(action);
    assert!(parsed.is_ok(), "accepted action JSON");
    if let Ok(action) = parsed
        && let JsonValue::Object(object) = &mut value
    {
        object.insert("action".to_owned(), action);
        object.insert("generation".to_owned(), JsonValue::Number(generation));
        object.insert(
            "state_id".to_owned(),
            JsonValue::string(format!("live:{generation}")),
        );
    }
    value
}

pub(crate) fn newer_smith_requested(operation_id: &str) -> JsonValue {
    let mut value = response_fixture("smith-requested", operation_id);
    for (from, to) in [
        ("card:1", "card:3"),
        ("card:2", "card:4"),
        ("select_card:10:smith:card:1", "select_card:20:smith:card:3"),
        ("select_card:10:smith:card:2", "select_card:20:smith:card:4"),
        ("select_card:10:card:1", "select_card:20:card:3"),
        ("select_card:10:card:2", "select_card:20:card:4"),
        ("live:10", "live:20"),
    ] {
        replace_string(&mut value, from, to);
    }
    replace_number(&mut value, 10, 20);
    replace_number(&mut value, 9, 19);
    if let JsonValue::Object(object) = &mut value {
        object.insert("action".to_owned(), selector_action(200));
        object.insert("generation".to_owned(), JsonValue::Number(20));
        object.insert("state_id".to_owned(), JsonValue::string("live:20"));
    }
    value
}

pub(crate) fn newer_smith_progressed(
    path: &str,
    operation_id: &str,
    generation: i64,
    before: i64,
    after: i64,
) -> JsonValue {
    let mut value = response_fixture(path, operation_id);
    for (from, to) in [
        ("card:1", "card:3"),
        ("card:2", "card:4"),
        ("select_card:10:smith:card:1", "select_card:20:smith:card:3"),
        ("select_card:11:smith:card:2", "select_card:21:smith:card:4"),
        ("select_card:10:card:1", "select_card:20:card:3"),
        ("select_card:11:card:2", "select_card:21:card:4"),
        ("confirm_selection:12:smith", "confirm_selection:22:smith"),
        ("confirm_selection:12", "confirm_selection:22"),
        ("live:11", "live:21"),
        ("live:12", "live:22"),
    ] {
        replace_string(&mut value, from, to);
    }
    replace_number(&mut value, 11, generation);
    replace_number(&mut value, 12, generation);
    replace_number(&mut value, 10, before);
    if let JsonValue::Object(object) = &mut value {
        object.insert("generation".to_owned(), JsonValue::Number(generation));
        object.insert(
            "state_id".to_owned(),
            JsonValue::string(format!("live:{generation}")),
        );
        if let Some(JsonValue::Object(transition)) = object.get_mut("transition") {
            transition.insert("before_generation".to_owned(), JsonValue::Number(before));
            transition.insert("after_generation".to_owned(), JsonValue::Number(after));
        }
        if let Some(JsonValue::Object(observation)) = object.get_mut("observation") {
            observation.insert("generation".to_owned(), JsonValue::Number(generation));
            observation.insert(
                "state_id".to_owned(),
                JsonValue::string(format!("live:{generation}")),
            );
        }
    }
    value
}

pub(crate) fn newer_smith_completed(operation_id: &str) -> JsonValue {
    let mut value = response_fixture("smith-completed", operation_id);
    for (from, to) in [
        ("card:1", "card:3"),
        ("card:2", "card:4"),
        ("confirm_selection:12:smith", "confirm_selection:22:smith"),
        ("live:13", "live:23"),
        ("live:12", "live:22"),
    ] {
        replace_string(&mut value, from, to);
    }
    replace_number(&mut value, 13, 23);
    replace_number(&mut value, 12, 22);
    if let JsonValue::Object(object) = &mut value {
        object.insert("generation".to_owned(), JsonValue::Number(23));
        object.insert("state_id".to_owned(), JsonValue::string("live:23"));
    }
    value
}
