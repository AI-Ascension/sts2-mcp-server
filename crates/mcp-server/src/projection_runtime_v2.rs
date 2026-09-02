// SPDX-License-Identifier: MIT

use crate::json::JsonValue;

#[path = "projection_runtime_v2_validation.rs"]
mod validation;

pub(crate) use validation::RuntimeV2Context;

/// Validates and returns the complete Runtime-v2 envelope without dropping
/// outcome, identity, or error-origin fields.
pub(crate) fn project_runtime_v2_gateway_body(
    body: &JsonValue,
    context: &RuntimeV2Context,
    expected_kind: &str,
) -> Result<JsonValue, &'static str> {
    validation::validate_runtime_v2_gateway_body(body, context, expected_kind)?;
    Ok(body.clone())
}

/// Returns whether a validated Runtime-v2 result must be surfaced as an MCP
/// tool error. Unknown is intentionally an error result, not a retryable
/// success, because mutation may already have happened.
pub(crate) fn runtime_v2_result_is_error(body: &JsonValue) -> bool {
    let Some(object) = body.as_object() else {
        return true;
    };
    matches!(
        object.get("status").and_then(JsonValue::as_string),
        Some("rejected" | "unknown" | "cancelled")
    )
}
