// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::json::JsonValue;

#[path = "mapping_game_information_shapes_binding.rs"]
mod binding;
#[path = "mapping_game_information_shapes_helpers.rs"]
mod helpers;
#[path = "mapping_game_information_shapes_page.rs"]
mod page;
#[path = "mapping_game_information_shapes_values.rs"]
mod values;

const MAX_IDENTITY: usize = 128;
const MAX_REASON: usize = 256;
const MAX_INTEGER: i64 = 9_007_199_254_740_991;
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

pub(super) fn validate_capabilities(value: &JsonValue) -> Result<(), &'static str> {
    let object = exact(
        value,
        &[
            "profile",
            "query_kinds",
            "entity_kinds",
            "projections",
            "detail_levels",
            "fields",
            "limits",
            "max_message_bytes",
            "max_cursor_bytes",
            "snapshot_policy",
        ],
    )?;
    if object.get("profile")
        != Some(&JsonValue::String(String::from(
            "game-information-query-v1",
        )))
    {
        return Err("capabilities profile is unsupported");
    }
    helpers::enum_array(
        object,
        "query_kinds",
        &["list", "search", "get", "detail", "availability"],
        1,
        5,
    )?;
    helpers::enum_array(object, "entity_kinds", &ENTITY_KINDS, 1, 10)?;
    helpers::enum_array(
        object,
        "projections",
        &["summary", "standard", "full"],
        1,
        3,
    )?;
    helpers::enum_array(
        object,
        "detail_levels",
        &["summary", "standard", "full"],
        1,
        3,
    )?;
    helpers::enum_array(
        object,
        "fields",
        &[
            "amount",
            "cost",
            "description",
            "display_name",
            "flags",
            "owner",
            "position",
            "rarity",
            "source_id",
            "tags",
        ],
        1,
        10,
    )?;
    helpers::validate_limits(
        object
            .get("limits")
            .ok_or("capabilities limits are missing")?,
    )?;
    bounded_number(object, "max_message_bytes", 1, 262_144)?;
    bounded_number(object, "max_cursor_bytes", 1, 512)?;
    helpers::validate_snapshot_policy(
        object
            .get("snapshot_policy")
            .ok_or("snapshot policy is missing")?,
    )?;
    Ok(())
}

pub(super) fn validate_query_result(
    result: &JsonValue,
    query: &JsonValue,
) -> Result<(), &'static str> {
    let object = exact(
        result,
        &[
            "page",
            "result_generation",
            "parent_observation",
            "read_only",
        ],
    )?;
    if object.get("read_only") != Some(&JsonValue::Bool(true)) {
        return Err("game-information result is not read-only");
    }
    let query_object = query.as_object().ok_or("query is not an object")?;
    let binding = query_object
        .get("binding")
        .ok_or("query binding is missing")?;
    binding::validate(binding)?;
    let mode = binding
        .as_object()
        .and_then(|value| value.get("mode"))
        .and_then(JsonValue::as_string)
        .ok_or("query binding mode is missing")?;
    let parent = query_object
        .get("parent_observation")
        .ok_or("query parent observation is missing")?;
    if object.get("parent_observation") != Some(parent) {
        return Err("result parent observation does not match the query");
    }
    let generation = object
        .get("result_generation")
        .ok_or("result generation is missing")?;
    if mode == "static" {
        if generation != &JsonValue::Null || parent != &JsonValue::Null {
            return Err("static result has live identity");
        }
    } else {
        let expected = binding
            .as_object()
            .and_then(|value| value.get("snapshot_ref"))
            .and_then(JsonValue::as_object)
            .and_then(|value| value.get("state_generation"))
            .ok_or("live binding has no snapshot generation")?;
        if generation != expected {
            return Err("live result generation is mixed or stale");
        }
        if parent == &JsonValue::Null {
            return Err("live result has no parent observation");
        }
    }
    page::validate(
        object.get("page").ok_or("result page is missing")?,
        query,
        mode,
    )
}

pub(super) fn validate_error(value: &JsonValue) -> Result<(), &'static str> {
    let object = exact(value, &["code", "field", "reason", "retryable"])?;
    enum_value(
        object,
        "code",
        &[
            "unknown_kind",
            "unknown_id",
            "ambiguous_id",
            "unsupported_filter",
            "unsupported_projection",
            "unsupported_version",
            "denied_scope",
            "stale_snapshot",
            "stale_cursor",
            "result_limit_exceeded",
            "missing_capability",
            "unsupported_field",
            "invalid_identity",
            "invalid_bounds",
            "mixed_generation",
            "malformed",
            "read_only_violation",
        ],
    )?;
    match object.get("field") {
        Some(JsonValue::Null) => {}
        Some(JsonValue::String(value)) if valid_identity(value) => {}
        _ => return Err("error field is invalid"),
    }
    match object.get("reason") {
        Some(JsonValue::Null) => {}
        Some(JsonValue::String(value))
            if !value.is_empty() && value.len() <= MAX_REASON && safe_text(value) => {}
        _ => return Err("error reason is invalid"),
    }
    if !matches!(object.get("retryable"), Some(JsonValue::Bool(_))) {
        return Err("error retryable member is invalid");
    }
    Ok(())
}

fn exact<'a>(
    value: &'a JsonValue,
    keys: &[&str],
) -> Result<&'a BTreeMap<String, JsonValue>, &'static str> {
    let object = value
        .as_object()
        .ok_or("game-information value must be an object")?;
    if object.len() != keys.len() || keys.iter().any(|key| !object.contains_key(*key)) {
        return Err("game-information object has unknown or missing fields");
    }
    Ok(object)
}

fn enum_value<'a>(
    object: &'a BTreeMap<String, JsonValue>,
    key: &str,
    values: &[&str],
) -> Result<&'a str, &'static str> {
    let value = object
        .get(key)
        .and_then(JsonValue::as_string)
        .ok_or("enum value is not a string")?;
    values
        .contains(&value)
        .then_some(value)
        .ok_or("enum value is unsupported")
}

fn identity<'a>(
    object: &'a BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<&'a str, &'static str> {
    let value = object
        .get(key)
        .and_then(JsonValue::as_string)
        .ok_or("identity is not a string")?;
    valid_identity(value)
        .then_some(value)
        .ok_or("identity is empty, unsafe, or oversized")
}

fn bounded_number(
    object: &BTreeMap<String, JsonValue>,
    key: &str,
    minimum: i64,
    maximum: i64,
) -> Result<(), &'static str> {
    let value = bounded_object_number(object, key)?;
    if (minimum..=maximum).contains(&value) {
        Ok(())
    } else {
        Err("integer is outside its bound")
    }
}

fn bounded_object_number(
    object: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<i64, &'static str> {
    match object.get(key) {
        Some(JsonValue::Number(value)) if (0..=MAX_INTEGER).contains(value) => Ok(*value),
        _ => Err("integer is outside the protocol bound"),
    }
}

fn number(value: &JsonValue) -> Option<i64> {
    match value {
        JsonValue::Number(value) => Some(*value),
        _ => None,
    }
}

fn valid_identity(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_IDENTITY
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'/' | b'-')
        })
}

fn valid_cursor(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 512
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(byte, b'.' | b'_' | b'~' | b':' | b'/' | b'+' | b'=' | b'-')
        })
}

fn safe_text(value: &str) -> bool {
    value
        .bytes()
        .all(|byte| !(byte < 0x20 || (0x7f..=0x9f).contains(&byte)))
}
