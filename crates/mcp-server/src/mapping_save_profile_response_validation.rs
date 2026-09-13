// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::catalog::SAVE_PROFILE_LAUNCH_PROFILE_CONTRACT;
use crate::json::JsonValue;

use super::{CallKind, Context, RESPONSE_FIELDS, ROUTES};

#[path = "mapping_save_profile_response_validation_text.rs"]
mod text;
pub(super) use text::{
    safe_identity, safe_operation_id, validate_error_code, validate_error_string, validate_guidance,
};

pub(super) fn validate_fields(object: &BTreeMap<String, JsonValue>) -> Result<(), &'static str> {
    if object
        .keys()
        .any(|key| !RESPONSE_FIELDS.contains(&key.as_str()))
    {
        return Err("save-profile response has unsupported fields");
    }
    Ok(())
}

pub(super) fn validate_route(
    value: Option<&JsonValue>,
    kind: CallKind,
    status: &str,
) -> Result<(), &'static str> {
    if matches!(status, "pending" | "created") {
        if !matches!(kind, CallKind::CreateDisposable | CallKind::Status) {
            return Err("provisioning status is invalid for this save-profile tool");
        }
        if value.is_some() {
            return Err("provisioning status must not carry a ledger route");
        }
        return Ok(());
    }
    let Some(value) = value else {
        return if kind == CallKind::Status
            || (kind == CallKind::CreateDisposable
                && matches!(
                    status,
                    "pending" | "created" | "unknown" | "blocked" | "rejected"
                ))
        {
            Ok(())
        } else {
            Err("save-profile response route is missing")
        };
    };
    let route = value
        .as_string()
        .ok_or("save-profile response route is not a string")?;
    if !ROUTES.contains(&route) {
        return Err("save-profile response route is unsupported");
    }
    if kind != CallKind::Status
        && kind.expected_route() != route
        && !(kind == CallKind::CreateDisposable && route == "createdisposable")
    {
        return Err("save-profile response route does not match the request");
    }
    Ok(())
}

pub(super) fn validate_identity_echo(
    object: &BTreeMap<String, JsonValue>,
    context: &Context,
) -> Result<(), &'static str> {
    for (field, expected) in [
        ("instance_id", context.instance_id.as_str()),
        ("session_id", context.gateway_session_id.as_str()),
        ("mcp_session_id", context.mcp_session_id.as_str()),
        ("lease_id", context.lease_id.as_str()),
        ("correlation_id", context.correlation_id.as_str()),
    ] {
        if let Some(value) = object.get(field)
            && value.as_string() != Some(expected)
        {
            return Err("save-profile response authority identity does not match");
        }
    }
    if let Some(value) = object.get("caller_id") {
        let caller = value
            .as_string()
            .ok_or("save-profile response caller identity is invalid")?;
        if !safe_identity(caller) {
            return Err("save-profile response caller identity is invalid");
        }
    }
    if let Some(value) = object.get("lease_epoch")
        && value != &JsonValue::Number(context.lease_epoch)
    {
        return Err("save-profile response lease epoch does not match");
    }
    Ok(())
}

pub(super) fn validate_result_requirements(
    object: &BTreeMap<String, JsonValue>,
    context: &Context,
    status: &str,
) -> Result<(), &'static str> {
    if status == "created"
        && context.kind == CallKind::CreateDisposable
        && object.get("route").is_none()
    {
        if object.get("user_data").is_none() || object.get("user_data") == Some(&JsonValue::Null) {
            return Err("created disposable provisioning lacks user-data identity");
        }
        return Ok(());
    }
    if !matches!(status, "settled" | "created") {
        return Ok(());
    }
    match context.kind {
        CallKind::Select => {
            let returned = object
                .get("profile_id")
                .or_else(|| object.get("save_profile_id"))
                .and_then(JsonValue::as_string);
            if returned != context.requested_profile.as_deref()
                || object.get("baseline").is_none()
                || object.get("baseline") == Some(&JsonValue::Null)
            {
                return Err("settled save-profile selection lacks authoritative readback");
            }
        }
        CallKind::CreateDisposable => {
            if object.get("user_data").is_none()
                || object.get("user_data") == Some(&JsonValue::Null)
                || object.get("baseline").is_none()
                || object.get("baseline") == Some(&JsonValue::Null)
            {
                return Err("settled disposable profile lacks authoritative readback");
            }
        }
        CallKind::List | CallKind::Current | CallKind::Status => {}
    }
    Ok(())
}

pub(super) fn validate_optional_profile(value: Option<&JsonValue>) -> Result<(), &'static str> {
    let Some(value) = value else {
        return Ok(());
    };
    if matches!(value, JsonValue::Null) {
        return Ok(());
    }
    let value = value
        .as_string()
        .ok_or("save-profile identity is not a string")?;
    if !safe_identity(value) {
        return Err("save-profile identity is unsafe or oversized");
    }
    Ok(())
}

pub(super) fn validate_baseline(value: Option<&JsonValue>) -> Result<(), &'static str> {
    let Some(value) = value else {
        return Ok(());
    };
    if matches!(value, JsonValue::Null) {
        return Ok(());
    }
    let object = value
        .as_object()
        .ok_or("save-profile baseline is not an object")?;
    if object.len() != 2
        || object
            .keys()
            .any(|key| !["identity", "digest"].contains(&key.as_str()))
    {
        return Err("save-profile baseline has unsupported fields");
    }
    let identity = object
        .get("identity")
        .and_then(JsonValue::as_string)
        .ok_or("save-profile baseline identity is missing")?;
    let digest = object
        .get("digest")
        .and_then(JsonValue::as_string)
        .ok_or("save-profile baseline digest is missing")?;
    if !safe_identity(identity) || !is_digest(digest) {
        return Err("save-profile baseline is invalid");
    }
    Ok(())
}

pub(super) fn validate_user_data(
    value: Option<&JsonValue>,
    context: &Context,
) -> Result<(), &'static str> {
    let Some(value) = value else {
        return Ok(());
    };
    if matches!(value, JsonValue::Null) {
        return Ok(());
    }
    let object = value
        .as_object()
        .ok_or("save-profile user_data is not an object")?;
    if object
        .keys()
        .any(|key| !["identity", "provenance", "baseline"].contains(&key.as_str()))
    {
        return Err("save-profile user_data has unsupported fields");
    }
    object
        .get("identity")
        .and_then(number)
        .filter(|value| *value > 0)
        .ok_or("save-profile user_data identity is invalid")?;
    let provenance = object
        .get("provenance")
        .and_then(JsonValue::as_object)
        .ok_or("save-profile user_data provenance is missing")?;
    if provenance.len() != 4
        || provenance.keys().any(|key| {
            !["owner", "instance_id", "operation_id", "contract"].contains(&key.as_str())
        })
    {
        return Err("save-profile user_data provenance is invalid");
    }
    for key in ["owner", "instance_id", "operation_id", "contract"] {
        let value = provenance
            .get(key)
            .and_then(JsonValue::as_string)
            .ok_or("save-profile user_data provenance identity is invalid")?;
        if !safe_identity(value) {
            return Err("save-profile user_data provenance identity is invalid");
        }
    }
    if provenance.get("owner").and_then(JsonValue::as_string) != Some("gateway")
        || provenance.get("instance_id").and_then(JsonValue::as_string)
            != Some(context.instance_id.as_str())
        || provenance
            .get("operation_id")
            .and_then(JsonValue::as_string)
            != Some(context.operation_id.as_str())
        || provenance.get("contract").and_then(JsonValue::as_string)
            != Some(SAVE_PROFILE_LAUNCH_PROFILE_CONTRACT)
    {
        return Err("save-profile user_data provenance is not gateway-owned");
    }
    validate_baseline(object.get("baseline"))
}

fn number(value: &JsonValue) -> Option<i64> {
    match value {
        JsonValue::Number(value) => Some(*value),
        _ => None,
    }
}

fn is_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
