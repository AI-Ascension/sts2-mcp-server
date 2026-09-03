// SPDX-License-Identifier: MIT

use crate::catalog::MAX_IDENTIFIER_BYTES;
use crate::json::JsonValue;
use crate::protocol_artifact::{POC_MAX_GENERATION, POC_MAX_SETTLED_EFFECTS, POC_MAX_UNITS};

#[path = "projection_runtime.rs"]
mod runtime;
#[path = "projection_runtime_v2.rs"]
mod runtime_v2;
#[path = "projection_runtime_v3_gameplay.rs"]
mod runtime_v3_gameplay;

pub(crate) use runtime::project_runtime_gateway_body;
pub(crate) use runtime_v2::{
    RuntimeV2Context, project_runtime_v2_gateway_body, runtime_v2_result_is_error,
};
pub(crate) use runtime_v3_gameplay::{
    RuntimeV3GameplayContext, project_runtime_v3_gameplay_gateway_body,
    runtime_v3_gameplay_result_is_error,
};

const ALLOWLISTED_KINDS: [&str; 4] = [
    "state_request",
    "state_response",
    "action_request",
    "action_response",
];

pub(crate) fn project_gateway_body(body: &JsonValue) -> Result<JsonValue, &'static str> {
    let Some(object) = body.as_object() else {
        return Err("gateway response must be an object");
    };
    let mut projection = Vec::new();
    if let Some(value) = object.get("kind") {
        let Some(kind) = value.as_string() else {
            return Err("gateway kind must be a string");
        };
        if !ALLOWLISTED_KINDS.contains(&kind) {
            return Err("gateway kind is not allowlisted");
        }
        projection.push((String::from("kind"), JsonValue::string(kind)));
    }
    if let Some(value) = object.get("generation") {
        projection.push((
            String::from("generation"),
            JsonValue::Number(bounded_number(value, POC_MAX_GENERATION)?),
        ));
    }
    if let Some(value) = object.get("observation")
        && !matches!(value, JsonValue::Null)
    {
        projection.push((String::from("observation"), project_observation(value)?));
    }
    if let Some(value) = object.get("action")
        && !matches!(value, JsonValue::Null)
    {
        projection.push((String::from("action"), project_action(value)?));
    }
    if let Some(value) = object.get("status")
        && !matches!(value, JsonValue::Null)
    {
        let Some(status) = value.as_string() else {
            return Err("gateway status must be a string");
        };
        if !matches!(status, "accepted" | "rejected") {
            return Err("gateway status is not allowlisted");
        }
        projection.push((String::from("status"), JsonValue::string(status)));
    }
    if let Some(value) = object.get("error_code")
        && !matches!(value, JsonValue::Null)
    {
        let Some(error_code) = value.as_string() else {
            return Err("gateway error_code must be a string");
        };
        if !safe_identifier(error_code) {
            return Err("gateway error_code is unsafe or oversized");
        }
        projection.push((String::from("error_code"), JsonValue::string(error_code)));
    }
    if !projection.iter().any(|(key, _)| {
        matches!(
            key.as_str(),
            "observation" | "action" | "status" | "error_code"
        )
    }) {
        return Err("gateway response contains no allowlisted fields");
    }
    Ok(JsonValue::object(projection))
}

pub(crate) fn projection_is_error(body: &JsonValue) -> bool {
    let Some(object) = body.as_object() else {
        return true;
    };
    matches!(
        object.get("status").and_then(JsonValue::as_string),
        Some("rejected")
    ) || matches!(
        object.get("error_code").and_then(JsonValue::as_string),
        Some(value) if !value.is_empty()
    )
}

fn project_observation(value: &JsonValue) -> Result<JsonValue, &'static str> {
    let Some(object) = value.as_object() else {
        return Err("gateway observation must be an object");
    };
    let available_units = bounded_number(
        object
            .get("available_units")
            .ok_or("gateway observation is missing available_units")?,
        i64::from(POC_MAX_UNITS),
    )?;
    let settled_effects = bounded_number(
        object
            .get("settled_effects")
            .ok_or("gateway observation is missing settled_effects")?,
        i64::from(POC_MAX_SETTLED_EFFECTS),
    )?;
    Ok(JsonValue::object([
        (
            String::from("available_units"),
            JsonValue::Number(available_units),
        ),
        (
            String::from("settled_effects"),
            JsonValue::Number(settled_effects),
        ),
    ]))
}

fn project_action(value: &JsonValue) -> Result<JsonValue, &'static str> {
    let Some(object) = value.as_object() else {
        return Err("gateway action must be an object");
    };
    let Some(action_id) = object.get("action_id").and_then(JsonValue::as_string) else {
        return Err("gateway action is missing action_id");
    };
    if action_id != "use_budget" {
        return Err("gateway action_id is not allowlisted");
    }
    let units = bounded_number(
        object
            .get("units")
            .ok_or("gateway action is missing units")?,
        i64::from(POC_MAX_UNITS),
    )?;
    Ok(JsonValue::object([
        (String::from("action_id"), JsonValue::string(action_id)),
        (String::from("units"), JsonValue::Number(units)),
    ]))
}

fn bounded_number(value: &JsonValue, maximum: i64) -> Result<i64, &'static str> {
    match value {
        JsonValue::Number(value) if *value >= 0 && *value <= maximum => Ok(*value),
        _ => Err("gateway numeric field is outside the POC bound"),
    }
}

fn safe_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_IDENTIFIER_BYTES
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':' | b'/')
        })
}
