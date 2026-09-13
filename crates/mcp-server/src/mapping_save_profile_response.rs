// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::catalog::{SAVE_PROFILE_CONTRACT, SAVE_PROFILE_MAX_BODY_BYTES};
use crate::gateway::{GatewayError, GatewayResponse};
use crate::json::JsonValue;

use super::{CallKind, Context};

#[path = "mapping_save_profile_response_validation.rs"]
mod validation;

const RESPONSE_FIELDS: [&str; 19] = [
    "contract",
    "schema_revision",
    "operation_id",
    "route",
    "status",
    "profile_id",
    "save_profile_id",
    "baseline",
    "user_data",
    "guidance",
    "downstream",
    "error_code",
    "instance_id",
    "caller_id",
    "session_id",
    "mcp_session_id",
    "lease_id",
    "lease_epoch",
    "correlation_id",
];
const ROUTES: [&str; 4] = ["list", "current", "select", "createdisposable"];
const STATUSES: [&str; 8] = [
    "accepted",
    "settled",
    "rejected",
    "unknown",
    "blocked",
    "cancelled",
    "pending",
    "created",
];

pub(super) fn gateway_success(
    context: Context,
    response: GatewayResponse,
) -> crate::protocol::RpcResponse {
    let body = match normalize(&context, &response) {
        Ok(body) => body,
        Err(message) => {
            return super::super::tool_error_result(
                context.request_id,
                "save_profile_malformed_response",
                "malformed_response",
                message,
            );
        }
    };
    let encoded = body.to_json();
    if encoded.len() > SAVE_PROFILE_MAX_BODY_BYTES {
        return super::super::tool_error_result(
            context.request_id,
            "save_profile_response_too_large",
            "size",
            "save-profile response exceeded the byte limit",
        );
    }
    let status = body
        .as_object()
        .and_then(|object| object.get("status"))
        .and_then(JsonValue::as_string)
        .unwrap_or("rejected");
    let status_error = matches!(status, "rejected" | "unknown" | "blocked" | "cancelled");
    if status_error || !(200..300).contains(&response.status) {
        let code = error_code(&body).unwrap_or_else(|| format!("save_profile_{status}"));
        return super::super::tool_error_result_with_metadata(
            context.request_id,
            code,
            status_category(status, error_code(&body)),
            encoded,
            None,
            Some(response.status),
        );
    }
    super::super::tool_result(context.request_id, encoded, false)
}

pub(super) fn unknown_result(
    context: Context,
    error: GatewayError,
) -> crate::protocol::RpcResponse {
    let error_code = match error {
        GatewayError::Timeout => "save_profile_unknown_after_timeout",
        GatewayError::Unavailable => "save_profile_unknown_after_disconnect",
        GatewayError::MalformedResponse => "save_profile_unknown_after_malformed_response",
        GatewayError::ResponseTooLarge => "save_profile_unknown_after_oversized_response",
        _ => "save_profile_unknown",
    };
    let body = JsonValue::object([
        ("baseline".to_owned(), JsonValue::Null),
        (
            "contract".to_owned(),
            JsonValue::string(SAVE_PROFILE_CONTRACT),
        ),
        ("downstream".to_owned(), JsonValue::Null),
        ("error_code".to_owned(), JsonValue::string(error_code)),
        (
            "guidance".to_owned(),
            guidance("save_profile_reconcile_required"),
        ),
        (
            "operation_id".to_owned(),
            JsonValue::string(context.operation_id),
        ),
        ("profile_id".to_owned(), JsonValue::Null),
        (
            "route".to_owned(),
            JsonValue::string(context.kind.expected_route()),
        ),
        (
            "schema_revision".to_owned(),
            JsonValue::string(SAVE_PROFILE_CONTRACT),
        ),
        ("status".to_owned(), JsonValue::string("unknown")),
        ("user_data".to_owned(), JsonValue::Null),
    ]);
    let text = body.to_json();
    super::super::tool_error_result_with_metadata(
        context.request_id,
        error_code,
        "transport",
        text,
        None,
        None,
    )
}

fn normalize(context: &Context, response: &GatewayResponse) -> Result<JsonValue, &'static str> {
    let object = response
        .body
        .as_object()
        .ok_or("save-profile response must be an object")?;
    if object.get("contract").is_none() {
        return normalize_error(context, response.status, object);
    }
    validation::validate_fields(object)?;
    if object.get("contract").and_then(JsonValue::as_string) != Some(SAVE_PROFILE_CONTRACT) {
        return Err("save-profile response contract is unsupported");
    }
    if let Some(revision) = object.get("schema_revision")
        && revision.as_string() != Some(SAVE_PROFILE_CONTRACT)
    {
        return Err("save-profile response schema revision is unsupported");
    }
    let operation_id = object
        .get("operation_id")
        .and_then(JsonValue::as_string)
        .ok_or("save-profile response operation_id is missing")?;
    if operation_id != context.operation_id || !validation::safe_operation_id(operation_id) {
        return Err("save-profile response operation identity does not match");
    }
    let status = object
        .get("status")
        .and_then(JsonValue::as_string)
        .ok_or("save-profile response status is missing")?;
    if !STATUSES.contains(&status) {
        return Err("save-profile response status is unsupported");
    }
    validation::validate_route(object.get("route"), context.kind, status)?;
    validation::validate_identity_echo(object, context)?;
    validation::validate_optional_profile(object.get("profile_id"))?;
    validation::validate_optional_profile(object.get("save_profile_id"))?;
    if object.get("profile_id").is_some() && object.get("save_profile_id").is_some() {
        return Err("save-profile response contains duplicate profile identities");
    }
    validation::validate_baseline(object.get("baseline"))?;
    validation::validate_user_data(object.get("user_data"), context)?;
    validation::validate_guidance(object.get("guidance"))?;
    validation::validate_error_code(object.get("error_code"))?;
    if let Some(downstream) = object.get("downstream")
        && downstream.to_json().len() > SAVE_PROFILE_MAX_BODY_BYTES
    {
        return Err("save-profile downstream content exceeds the byte limit");
    }
    validation::validate_result_requirements(object, context, status)?;
    Ok(response.body.clone())
}

fn normalize_error(
    context: &Context,
    status: u16,
    object: &BTreeMap<String, JsonValue>,
) -> Result<JsonValue, &'static str> {
    if object
        .keys()
        .any(|key| !["error_code", "retryable", "retry_after_ms"].contains(&key.as_str()))
    {
        return Err("save-profile error response has unsupported fields");
    }
    let code = object
        .get("error_code")
        .and_then(JsonValue::as_string)
        .ok_or("save-profile error response has no error_code")?;
    validation::validate_error_string(code)?;
    if object.get("retryable").is_some() && object.get("retryable") != Some(&JsonValue::Bool(true))
    {
        return Err("save-profile retryable guidance is invalid");
    }
    if let Some(JsonValue::Number(value)) = object.get("retry_after_ms") {
        if !(0..=60_000).contains(value) {
            return Err("save-profile retry guidance is outside the bound");
        }
    } else if object.get("retry_after_ms").is_some() {
        return Err("save-profile retry guidance is invalid");
    }
    let wire_status = match status {
        408 | 502 | 503 | 504 => "unknown",
        _ => "rejected",
    };
    Ok(JsonValue::object([
        ("baseline".to_owned(), JsonValue::Null),
        (
            "contract".to_owned(),
            JsonValue::string(SAVE_PROFILE_CONTRACT),
        ),
        ("downstream".to_owned(), JsonValue::Object(object.clone())),
        ("error_code".to_owned(), JsonValue::string(code)),
        ("guidance".to_owned(), guidance_for_status(wire_status)),
        (
            "operation_id".to_owned(),
            JsonValue::string(context.operation_id.as_str()),
        ),
        ("profile_id".to_owned(), JsonValue::Null),
        (
            "route".to_owned(),
            JsonValue::string(context.kind.expected_route()),
        ),
        (
            "schema_revision".to_owned(),
            JsonValue::string(SAVE_PROFILE_CONTRACT),
        ),
        ("status".to_owned(), JsonValue::string(wire_status)),
        ("user_data".to_owned(), JsonValue::Null),
    ]))
}

fn error_code(value: &JsonValue) -> Option<String> {
    value
        .as_object()?
        .get("error_code")
        .and_then(JsonValue::as_string)
        .map(str::to_owned)
}

fn status_category(status: &str, code: Option<String>) -> &'static str {
    if code
        .as_deref()
        .is_some_and(|value| value.contains("fence") || value.contains("stale"))
    {
        return "stale";
    }
    if code
        .as_deref()
        .is_some_and(|value| value.contains("not_found"))
    {
        return "missing";
    }
    match status {
        "unknown" => "transport",
        "blocked" => "denied",
        "cancelled" => "cancelled",
        _ => "invalid_input",
    }
}

fn guidance(code: &str) -> JsonValue {
    guidance_for_status(code)
}

fn guidance_for_status(status: &str) -> JsonValue {
    JsonValue::object([
        (
            "action".to_owned(),
            JsonValue::string(if status == "save_profile_reconcile_required" {
                "lookup the original operation before retrying"
            } else {
                "reconcile the retained operation"
            }),
        ),
        ("code".to_owned(), JsonValue::string(status)),
    ])
}
