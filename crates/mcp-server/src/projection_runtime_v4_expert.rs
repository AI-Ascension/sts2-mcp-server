// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::json::JsonValue;
use crate::protocol_artifact_runtime_v4_expert::{
    RUNTIME_V4_EXPERT_ACTION_ARTIFACT, RUNTIME_V4_EXPERT_ACTION_GENERATOR,
    RUNTIME_V4_EXPERT_ACTION_PROTOCOL_VERSION, RUNTIME_V4_EXPERT_ACTION_SCHEMA_DIGEST,
    RUNTIME_V4_EXPERT_ACTION_SCHEMA_SOURCE, RUNTIME_V4_EXPERT_ARTIFACT,
    RUNTIME_V4_EXPERT_GENERATOR, RUNTIME_V4_EXPERT_PROTOCOL_VERSION,
    RUNTIME_V4_EXPERT_SCHEMA_DIGEST, RUNTIME_V4_EXPERT_SCHEMA_SOURCE,
};

const MAX_GENERATION: i64 = 9_007_199_254_740_991;
const MAX_TEXT_BYTES: usize = 512;
const MAX_ITEMS: usize = 256;
const ROOT_FIELDS: [&str; 11] = [
    "protocol_version",
    "schema_digest",
    "provenance",
    "profile",
    "state_id",
    "generation",
    "visible_seed",
    "run",
    "player",
    "state",
    "legal_actions",
];

#[path = "projection_runtime_v4_expert_action.rs"]
mod action;
#[path = "projection_runtime_v4_expert_state.rs"]
mod state;

pub(crate) fn project_runtime_v4_expert_gateway_body(
    body: &JsonValue,
) -> Result<JsonValue, &'static str> {
    state::project(body)
}

pub(crate) fn project_runtime_v4_expert_action_gateway_body(
    body: &JsonValue,
) -> Result<JsonValue, &'static str> {
    action::project(body)
}

fn exact_object<'a>(
    value: Option<&'a JsonValue>,
    fields: &[&str],
    label: &str,
) -> Result<&'a BTreeMap<String, JsonValue>, &'static str> {
    let object = value
        .and_then(JsonValue::as_object)
        .ok_or("Runtime-v4 value must be an object")?;
    exact_fields(object, fields, label)?;
    Ok(object)
}

fn exact_fields(
    object: &BTreeMap<String, JsonValue>,
    fields: &[&str],
    _label: &str,
) -> Result<(), &'static str> {
    if object.len() == fields.len() && fields.iter().all(|field| object.contains_key(*field)) {
        Ok(())
    } else {
        Err("Runtime-v4 value contains unknown or missing fields")
    }
}

fn validate_identity(value: Option<&JsonValue>, _field: &str) -> Result<(), &'static str> {
    match value.and_then(JsonValue::as_string) {
        Some(value) if valid_identity(value) => Ok(()),
        _ => Err("Runtime-v4 identity is invalid"),
    }
}

fn validate_identity_value(value: &JsonValue) -> Result<(), &'static str> {
    validate_identity(Some(value), "identity")
}

fn optional_identity(value: Option<&JsonValue>, _field: &str) -> Result<(), &'static str> {
    match value {
        Some(JsonValue::Null) => Ok(()),
        Some(value) => validate_identity(Some(value), "identity"),
        None => Err("Runtime-v4 optional identity is missing"),
    }
}

fn validate_text(value: Option<&JsonValue>, _field: &str) -> Result<(), &'static str> {
    match value.and_then(JsonValue::as_string) {
        Some(value) if valid_text(value) => Ok(()),
        _ => Err("Runtime-v4 text is invalid"),
    }
}

fn optional_text(value: Option<&JsonValue>, _field: &str) -> Result<(), &'static str> {
    match value {
        Some(JsonValue::Null) => Ok(()),
        Some(value) => validate_text(Some(value), "text"),
        None => Err("Runtime-v4 optional text is missing"),
    }
}

fn require_bool(value: Option<&JsonValue>, _field: &str) -> Result<(), &'static str> {
    matches!(value, Some(JsonValue::Bool(_)))
        .then_some(())
        .ok_or("Runtime-v4 boolean is invalid")
}

fn optional_bool(value: Option<&JsonValue>, _field: &str) -> Result<(), &'static str> {
    match value {
        Some(JsonValue::Null) => Ok(()),
        Some(value) => require_bool(Some(value), "boolean"),
        None => Err("Runtime-v4 optional boolean is missing"),
    }
}

fn bounded_number(
    value: Option<&JsonValue>,
    minimum: i64,
    maximum: i64,
    _field: &str,
) -> Result<i64, &'static str> {
    match value {
        Some(JsonValue::Number(value)) if (*value >= minimum) && (*value <= maximum) => Ok(*value),
        _ => Err("Runtime-v4 number is outside its bound"),
    }
}

fn optional_number(
    value: Option<&JsonValue>,
    minimum: i64,
    maximum: i64,
    _field: &str,
) -> Result<(), &'static str> {
    match value {
        Some(JsonValue::Null) => Ok(()),
        Some(value) => bounded_number(Some(value), minimum, maximum, "number").map(|_| ()),
        None => Err("Runtime-v4 optional number is missing"),
    }
}

fn array(
    value: Option<&JsonValue>,
    maximum: usize,
    validate: fn(&JsonValue) -> Result<(), &'static str>,
    _field: &str,
) -> Result<(), &'static str> {
    let values = value
        .and_then(|value| match value {
            JsonValue::Array(values) => Some(values),
            _ => None,
        })
        .ok_or("Runtime-v4 value must be an array")?;
    if values.len() > maximum {
        return Err("Runtime-v4 array exceeds its bound");
    }
    values.iter().try_for_each(validate)
}

fn nullable_array(
    value: Option<&JsonValue>,
    maximum: usize,
    validate: fn(&JsonValue) -> Result<(), &'static str>,
    _field: &str,
) -> Result<(), &'static str> {
    match value {
        Some(JsonValue::Null) => Ok(()),
        Some(_) => array(value, maximum, validate, "array"),
        None => Err("Runtime-v4 nullable array is missing"),
    }
}

fn optional_identity_array(value: Option<&JsonValue>, field: &str) -> Result<(), &'static str> {
    match value {
        Some(JsonValue::Null) => Ok(()),
        Some(_) => array(value, 16, validate_identity_value, field),
        None => Err("Runtime-v4 target identity array is missing"),
    }
}

fn valid_identity(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_TEXT_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._:/-".contains(&byte))
}

fn valid_text(value: &str) -> bool {
    !value.is_empty() && value.len() <= MAX_TEXT_BYTES && !value.chars().any(char::is_control)
}
