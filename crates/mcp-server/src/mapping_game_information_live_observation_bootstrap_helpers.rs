// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::json::JsonValue;

pub(crate) const MAX_INTEGER: i64 = 9_007_199_254_740_991;
const ENTITY_KINDS: [&str; 10] = [
    "card",
    "character",
    "enemy",
    "event",
    "map_node",
    "potion",
    "power",
    "relic",
    "room",
    "status",
];

pub(crate) fn definition_ref(arguments: Option<&JsonValue>) -> Result<JsonValue, &'static str> {
    let value = arguments.ok_or("definition_ref is required")?;
    let object = definition_ref_value(value)?;
    Ok(JsonValue::Object(object.clone()))
}

pub(crate) fn definition_ref_value(
    value: &JsonValue,
) -> Result<&BTreeMap<String, JsonValue>, &'static str> {
    let object = exact_object(
        value,
        &[
            "content_manifest_id",
            "entity_kind",
            "namespaced_id",
            "variant",
        ],
    )?;
    if !object
        .get("content_manifest_id")
        .and_then(JsonValue::as_string)
        .is_some_and(valid_identity)
        || !object
            .get("namespaced_id")
            .and_then(JsonValue::as_string)
            .is_some_and(valid_identity)
        || !object
            .get("entity_kind")
            .and_then(JsonValue::as_string)
            .is_some_and(|value| ENTITY_KINDS.contains(&value))
    {
        return Err("definition_ref identity is invalid");
    }
    if object.get("variant") != Some(&JsonValue::Null)
        && !object
            .get("variant")
            .and_then(JsonValue::as_string)
            .is_some_and(valid_identity)
    {
        return Err("definition_ref variant is invalid");
    }
    Ok(object)
}

pub(crate) fn instance_ref_value(value: &JsonValue) -> Result<JsonValue, &'static str> {
    let object = exact_object(
        value,
        &["instance_id", "run_id", "epoch", "entity_kind", "entity_id"],
    )?;
    if !object
        .get("instance_id")
        .and_then(JsonValue::as_string)
        .is_some_and(valid_identity)
        || !object
            .get("run_id")
            .and_then(JsonValue::as_string)
            .is_some_and(valid_identity)
        || !object
            .get("entity_id")
            .and_then(JsonValue::as_string)
            .is_some_and(valid_identity)
        || !object
            .get("entity_kind")
            .and_then(JsonValue::as_string)
            .is_some_and(|value| ENTITY_KINDS.contains(&value))
    {
        return Err("instance_ref identity is invalid");
    }
    number(object, "epoch", 0, MAX_INTEGER)?;
    Ok(JsonValue::Object(object.clone()))
}

pub(crate) fn identity(
    arguments: &BTreeMap<String, JsonValue>,
    key: &str,
    instance: bool,
) -> Result<String, &'static str> {
    let value = arguments
        .get(key)
        .and_then(JsonValue::as_string)
        .filter(|value| valid_identity(value))
        .ok_or("identity argument is missing or invalid")?;
    if instance
        && !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    {
        return Err("instance_id is unsafe or oversized");
    }
    Ok(value.to_owned())
}

pub(crate) fn bounded_int(
    arguments: &BTreeMap<String, JsonValue>,
    key: &str,
    minimum: i64,
    maximum: i64,
) -> Result<i64, &'static str> {
    match arguments.get(key) {
        Some(JsonValue::Number(value)) if (minimum..=maximum).contains(value) => Ok(*value),
        _ => Err("integer argument is outside the protocol bound"),
    }
}

pub(crate) fn locale(
    arguments: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<String, &'static str> {
    let value = arguments
        .get(key)
        .and_then(JsonValue::as_string)
        .ok_or("locale is missing")?;
    let mut parts = value.split('-');
    if value.len() < 2
        || value.len() > 35
        || !parts.next().is_some_and(|part| {
            (2..=3).contains(&part.len()) && part.bytes().all(|byte| byte.is_ascii_alphabetic())
        })
        || parts.any(|part| {
            !(1..=8).contains(&part.len()) || !part.bytes().all(|byte| byte.is_ascii_alphanumeric())
        })
    {
        return Err("locale is outside the protocol bound");
    }
    Ok(value.to_owned())
}

pub(crate) fn exact_object<'a>(
    value: &'a JsonValue,
    expected: &[&str],
) -> Result<&'a BTreeMap<String, JsonValue>, &'static str> {
    let object = value
        .as_object()
        .ok_or("bootstrap value is not an object")?;
    if object.len() != expected.len() || expected.iter().any(|key| !object.contains_key(*key)) {
        return Err("bootstrap object has unknown or missing fields");
    }
    Ok(object)
}

pub(crate) fn number(
    object: &BTreeMap<String, JsonValue>,
    key: &str,
    minimum: i64,
    maximum: i64,
) -> Result<i64, &'static str> {
    match object.get(key) {
        Some(JsonValue::Number(value)) if (minimum..=maximum).contains(value) => Ok(*value),
        _ => Err("bootstrap integer is outside the protocol bound"),
    }
}

pub(crate) fn bounded(value: &JsonValue, maximum: usize) -> Result<(), &'static str> {
    if value.to_json().len() <= maximum {
        Ok(())
    } else {
        Err("bootstrap response exceeds max_message_bytes")
    }
}

pub(crate) fn valid_identity(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'/' | b'-')
        })
}

pub(crate) fn error_category(code: &str) -> &'static str {
    match code {
        "unsupported_version" => "unsupported",
        "invalid_bounds" => "size",
        "unavailable" | "not_observable" => "missing",
        "stale_snapshot" => "stale",
        "invalid_request" | "invalid_binding" | "ambiguous_entity" => "invalid_input",
        _ => "malformed_response",
    }
}

pub(crate) fn status_category(status: u16) -> &'static str {
    match status {
        400 | 422 => "invalid_input",
        401 | 403 => "denied",
        404 => "missing",
        409 => "stale",
        413 => "size",
        408 | 429 | 500..=599 => "transport",
        _ => "malformed_response",
    }
}
