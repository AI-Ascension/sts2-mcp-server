// SPDX-License-Identifier: MIT

use crate::json::JsonValue;

#[path = "projection_runtime_v3_gameplay_action_shape.rs"]
mod action_shape;
#[path = "projection_runtime_v3_gameplay_shape.rs"]
mod shape;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RuntimeV3GameplayProjectionContext {
    pub(crate) correlation_id: String,
    pub(crate) instance_id: String,
    pub(crate) session_id: String,
    pub(crate) lease_id: String,
    pub(crate) lease_epoch: i64,
    pub(crate) generation: i64,
    pub(crate) state_id: Option<String>,
    pub(crate) operation_id: Option<String>,
}

pub(crate) fn project_runtime_v3_gateway_body(
    body: &JsonValue,
    context: &RuntimeV3GameplayProjectionContext,
    expected_kind: &str,
) -> Result<JsonValue, &'static str> {
    shape::validate_and_project(body, expected_kind, context)
}

pub(crate) fn project_runtime_v3_legal_action(
    value: &JsonValue,
) -> Result<JsonValue, &'static str> {
    action_shape::project_legal_action(value)
}

pub(crate) fn runtime_v3_result_is_error(body: &JsonValue) -> bool {
    let Some(object) = body.as_object() else {
        return true;
    };
    matches!(
        object.get("status").and_then(JsonValue::as_string),
        Some("rejected" | "unknown" | "cancelled")
    ) || object
        .get("error_code")
        .and_then(JsonValue::as_string)
        .is_some()
}
