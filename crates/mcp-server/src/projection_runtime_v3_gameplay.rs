// SPDX-License-Identifier: MIT

use crate::json::JsonValue;

#[path = "projection_runtime_v3_gameplay_validation.rs"]
mod validation;

pub(crate) use validation::RuntimeV3GameplayContext;

pub(crate) fn project_runtime_v3_gameplay_gateway_body(
    body: &JsonValue,
    context: &RuntimeV3GameplayContext,
    expected_kind: &str,
) -> Result<JsonValue, &'static str> {
    validation::validate_gateway_body(body, context, expected_kind)?;
    Ok(body.clone())
}

pub(crate) fn runtime_v3_gameplay_result_is_error(body: &JsonValue) -> bool {
    let Some(object) = body.as_object() else {
        return true;
    };
    matches!(
        object.get("status").and_then(JsonValue::as_string),
        Some("rejected" | "unknown" | "cancelled")
    )
}
