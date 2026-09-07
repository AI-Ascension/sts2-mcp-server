// SPDX-License-Identifier: MIT

use super::*;

pub(super) fn valid_potion_action(value: &JsonValue) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };
    if object.len() != 2 {
        return false;
    }
    let Some(action_id) = object.get("action_id").and_then(JsonValue::as_string) else {
        return false;
    };
    if !super::super::safe_header_value(action_id) {
        return false;
    }
    let Some(action) = object.get("action").and_then(JsonValue::as_object) else {
        return false;
    };
    action.len() == 3
        && action.get("kind") == Some(&JsonValue::string("use_potion"))
        && action
            .get("potion_id")
            .and_then(JsonValue::as_string)
            .is_some_and(super::super::safe_header_value)
        && matches!(
            action.get("target_id"),
            Some(JsonValue::Null) | Some(JsonValue::String(_))
        )
}

pub(super) fn expert_error_result(id: RequestId, status: u16, body: &JsonValue) -> RpcResponse {
    let Some(object) = body.as_object() else {
        return super::tool_result(
            id,
            format!("gateway returned Runtime-v4 status {status}"),
            true,
        );
    };
    if object.len() != 1
        || !matches!(object.get("error_code"), Some(JsonValue::String(value))
            if !value.is_empty()
                && value.len() <= 128
                && value.bytes().all(|byte| byte.is_ascii_alphanumeric()
                    || matches!(byte, b'-' | b'_' | b'.' | b':' | b'/')))
    {
        return super::tool_result(
            id,
            format!("gateway returned Runtime-v4 status {status}"),
            true,
        );
    }
    super::tool_result(id, body.to_json(), true)
}
