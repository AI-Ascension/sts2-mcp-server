// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::json::JsonValue;

use super::action_shape::{project_legal_action, project_legal_actions};

#[path = "projection_runtime_v3_gameplay_observation_shape.rs"]
mod observation;
#[path = "projection_runtime_v3_gameplay_result_shape.rs"]
mod result;
#[path = "projection_runtime_v3_gameplay_state_shape.rs"]
mod state;

use result::validate_kind_shape;

const MAX_GENERATION: i64 = 9_007_199_254_740_991;
const ROOT_FIELDS: [&str; 21] = [
    "protocol_version",
    "schema_digest",
    "provenance",
    "correlation_id",
    "instance_id",
    "session_id",
    "lease_id",
    "lease_epoch",
    "generation",
    "kind",
    "state_id",
    "operation_id",
    "observation",
    "legal_actions",
    "action",
    "status",
    "transition",
    "error_code",
    "wait_for_millis",
    "wait_outcome",
    "recovery",
];

pub(super) fn validate_and_project(
    body: &JsonValue,
    expected_kind: &str,
    context: &super::RuntimeV3GameplayProjectionContext,
) -> Result<JsonValue, &'static str> {
    let object = exact_root(body)?;
    validate_metadata(object)?;
    validate_identity_field(object, "correlation_id")?;
    validate_identity_field(object, "instance_id")?;
    validate_identity_field(object, "session_id")?;
    validate_identity_field(object, "lease_id")?;
    if object.get("correlation_id").and_then(JsonValue::as_string)
        != Some(context.correlation_id.as_str())
        || object.get("instance_id").and_then(JsonValue::as_string)
            != Some(context.instance_id.as_str())
        || object.get("session_id").and_then(JsonValue::as_string)
            != Some(context.session_id.as_str())
        || object.get("lease_id").and_then(JsonValue::as_string) != Some(context.lease_id.as_str())
    {
        return Err("Runtime-v3 response identity does not match the request");
    }
    let lease_epoch = bounded_number(object.get("lease_epoch"), MAX_GENERATION)?;
    if lease_epoch != context.lease_epoch {
        return Err("Runtime-v3 response lease epoch does not match the request");
    }
    let generation = bounded_number(object.get("generation"), MAX_GENERATION)?;
    let kind = object
        .get("kind")
        .and_then(JsonValue::as_string)
        .ok_or("Runtime-v3 response kind must be a string")?;
    if kind != expected_kind {
        return Err("Runtime-v3 response kind does not match the requested operation");
    }
    if kind == "legal_actions_response"
        && (generation != context.generation
            || object.get("state_id").and_then(JsonValue::as_string) != context.state_id.as_deref())
    {
        return Err("Runtime-v3 legal action catalog does not match the requested observation");
    }
    validate_kind_shape(
        object,
        kind,
        generation,
        (kind == "dispatch_action_response").then_some(context.generation),
        context
            .operation_id
            .as_deref()
            .or_else(|| (kind == "recover_response").then_some(context.correlation_id.as_str())),
    )?;
    project_root(object)
}

fn exact_root(body: &JsonValue) -> Result<&BTreeMap<String, JsonValue>, &'static str> {
    let Some(object) = body.as_object() else {
        return Err("Runtime-v3 gateway response must be an object");
    };
    if object.len() != ROOT_FIELDS.len()
        || ROOT_FIELDS.iter().any(|field| !object.contains_key(*field))
    {
        return Err("Runtime-v3 gateway response contains unknown or missing fields");
    }
    Ok(object)
}

fn validate_metadata(object: &BTreeMap<String, JsonValue>) -> Result<(), &'static str> {
    if object
        .get("protocol_version")
        .and_then(JsonValue::as_string)
        != Some("runtime-v3-gameplay")
        || object.get("schema_digest").and_then(JsonValue::as_string)
            != Some("b37c80f583aeaf4f81ede2083bcfb4129196baf5eb092470e8738173c4b7226c")
    {
        return Err("Runtime-v3 metadata is unsupported");
    }
    let Some(provenance) = object.get("provenance").and_then(JsonValue::as_object) else {
        return Err("Runtime-v3 provenance must be an object");
    };
    if provenance.len() != 3
        || provenance.get("artifact").and_then(JsonValue::as_string)
            != Some("sts2-protocol/runtime-v3-gameplay")
        || provenance.get("source").and_then(JsonValue::as_string)
            != Some("schemas/runtime-v3-gameplay.schema.json")
        || provenance.get("generator").and_then(JsonValue::as_string) != Some("hand-authored")
    {
        return Err("Runtime-v3 provenance is unsupported");
    }
    Ok(())
}

fn require_error_code(object: &BTreeMap<String, JsonValue>) -> Result<(), &'static str> {
    let Some(value) = object.get("error_code").and_then(JsonValue::as_string) else {
        return Err("Runtime-v3 result error_code must be a string");
    };
    if safe_identifier(value) {
        Ok(())
    } else {
        Err("Runtime-v3 result error_code is unsafe or oversized")
    }
}

fn require_null(object: &BTreeMap<String, JsonValue>, field: &str) -> Result<(), &'static str> {
    if matches!(object.get(field), Some(JsonValue::Null)) {
        Ok(())
    } else {
        Err("Runtime-v3 response field has an invalid shape")
    }
}

fn validate_identity_field(
    object: &BTreeMap<String, JsonValue>,
    field: &str,
) -> Result<(), &'static str> {
    validate_identity_value(object.get(field))
}

fn validate_identity_value(value: Option<&JsonValue>) -> Result<(), &'static str> {
    let Some(value) = value.and_then(JsonValue::as_string) else {
        return Err("Runtime-v3 identity must be a string");
    };
    if safe_identifier(value) {
        Ok(())
    } else {
        Err("Runtime-v3 identity is unsafe or oversized")
    }
}

fn optional_identity(value: Option<&JsonValue>) -> Result<(), &'static str> {
    match value {
        Some(JsonValue::Null) => Ok(()),
        Some(JsonValue::String(_)) => validate_identity_value(value),
        _ => Err("Runtime-v3 optional identity is invalid"),
    }
}

fn observation_generation(object: &BTreeMap<String, JsonValue>) -> Result<i64, &'static str> {
    bounded_number(object.get("generation"), MAX_GENERATION)
}

fn bounded_number(value: Option<&JsonValue>, maximum: i64) -> Result<i64, &'static str> {
    match value {
        Some(JsonValue::Number(value)) if *value >= 0 && *value <= maximum => Ok(*value),
        _ => Err("Runtime-v3 numeric field is outside the bound"),
    }
}

fn safe_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 512
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._:/-".contains(&byte))
}

fn safe_text(value: &str) -> bool {
    !value.is_empty() && value.len() <= 512 && !value.chars().any(char::is_control)
}

fn project_root(object: &BTreeMap<String, JsonValue>) -> Result<JsonValue, &'static str> {
    let mut entries = Vec::with_capacity(ROOT_FIELDS.len());
    for field in ROOT_FIELDS {
        let value = match field {
            "legal_actions" if matches!(object.get(field), Some(JsonValue::Null)) => {
                JsonValue::Null
            }
            "legal_actions" => project_legal_actions(
                object
                    .get(field)
                    .ok_or("Runtime-v3 legal_actions is missing")?,
            )?,
            "action" if !matches!(object.get(field), Some(JsonValue::Null)) => {
                project_legal_action(object.get(field).ok_or("Runtime-v3 action is missing")?)?
            }
            _ => object
                .get(field)
                .cloned()
                .ok_or("Runtime-v3 response field is missing")?,
        };
        entries.push((String::from(field), value));
    }
    Ok(JsonValue::object(entries))
}
