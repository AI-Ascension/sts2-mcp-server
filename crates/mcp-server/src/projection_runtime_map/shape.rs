// SPDX-License-Identifier: MIT

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::json::JsonValue;

use super::MAX_ID_BYTES;

pub(super) const ROOT_FIELDS: [&str; 12] = [
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
    "snapshot",
    "timeout",
];
pub(super) const SNAPSHOT_FIELDS: [&str; 19] = [
    "state_id",
    "generation",
    "schema_version",
    "projection_version",
    "game_build",
    "mod_version",
    "map_instance_id",
    "act_id",
    "scope_id",
    "availability",
    "completeness",
    "freshness",
    "reason",
    "nodes",
    "edges",
    "position",
    "history",
    "terminal_node_ids",
    "bindings",
];

pub(super) fn validate_timeout(value: Option<&JsonValue>) -> Result<(), &'static str> {
    let Some(value) = value else {
        return Err("Runtime-map timeout is missing");
    };
    if matches!(value, JsonValue::Null) {
        return Ok(());
    }
    let object = exact_object(
        value,
        &["timeout_millis", "elapsed_millis"],
        "Runtime-map timeout",
    )?;
    let timeout = bounded_number(object.get("timeout_millis"), 120_000)?;
    let elapsed = bounded_number(object.get("elapsed_millis"), 120_000)?;
    if timeout == 0 || elapsed > timeout {
        return Err("Runtime-map timeout metadata is invalid");
    }
    Ok(())
}

pub(super) fn exact_object<'a>(
    value: &'a JsonValue,
    fields: &[&str],
    label: &'static str,
) -> Result<&'a BTreeMap<String, JsonValue>, &'static str> {
    let object = value.as_object().ok_or(label)?;
    if object.len() != fields.len() || fields.iter().any(|field| !object.contains_key(*field)) {
        return Err(label);
    }
    Ok(object)
}

pub(super) fn require_string(
    object: &BTreeMap<String, JsonValue>,
    field: &str,
    expected: &str,
) -> Result<(), &'static str> {
    let value = object.get(field).and_then(JsonValue::as_string);
    if value == Some(expected) {
        Ok(())
    } else {
        Err("Runtime-map string metadata is unsupported")
    }
}

pub(super) fn require_identity<'a>(
    object: &'a BTreeMap<String, JsonValue>,
    field: &str,
    maximum: usize,
) -> Result<&'a str, &'static str> {
    let value = object.get(field).and_then(JsonValue::as_string);
    if value.is_some_and(|value| valid_identity(value, maximum)) {
        Ok(value.unwrap_or_default())
    } else {
        Err("Runtime-map identity is invalid")
    }
}

pub(super) fn optional_identity(
    object: &BTreeMap<String, JsonValue>,
    field: &str,
    maximum: usize,
) -> Result<(), &'static str> {
    match object.get(field) {
        Some(JsonValue::Null) => Ok(()),
        Some(JsonValue::String(value)) if valid_identity(value, maximum) => Ok(()),
        _ => Err("Runtime-map optional identity is invalid"),
    }
}

pub(super) fn optional_u32(
    object: &BTreeMap<String, JsonValue>,
    field: &str,
) -> Result<(), &'static str> {
    match object.get(field) {
        Some(JsonValue::Null) => Ok(()),
        Some(JsonValue::Number(value)) if *value >= 0 && *value <= i64::from(u32::MAX) => Ok(()),
        _ => Err("Runtime-map act identity is invalid"),
    }
}

pub(super) fn enum_string<'a>(
    object: &'a BTreeMap<String, JsonValue>,
    field: &str,
    allowed: &[&str],
) -> Result<&'a str, &'static str> {
    let value = object
        .get(field)
        .and_then(JsonValue::as_string)
        .ok_or("Runtime-map enum field is invalid")?;
    if allowed.contains(&value) {
        Ok(value)
    } else {
        Err("Runtime-map enum field is unsupported")
    }
}

pub(super) fn require_text(
    object: &BTreeMap<String, JsonValue>,
    field: &str,
    maximum: usize,
) -> Result<(), &'static str> {
    let value = object.get(field).and_then(JsonValue::as_string);
    if value.is_some_and(|value| valid_text(value, maximum)) {
        Ok(())
    } else {
        Err("Runtime-map text field is invalid")
    }
}

pub(super) fn valid_identity(value: &str, maximum: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._:/-".contains(&byte))
}

pub(super) fn valid_text(value: &str, maximum: usize) -> bool {
    !value.is_empty() && value.len() <= maximum && !value.chars().any(char::is_control)
}

pub(super) fn bounded_number(value: Option<&JsonValue>, maximum: i64) -> Result<i64, &'static str> {
    match value {
        Some(JsonValue::Number(value)) if *value >= 0 && *value <= maximum => Ok(*value),
        _ => Err("Runtime-map number is outside the bound"),
    }
}

pub(super) fn bounded_signed(
    value: Option<&JsonValue>,
    minimum: i64,
    maximum: i64,
) -> Result<i64, &'static str> {
    match value {
        Some(JsonValue::Number(value)) if (minimum..=maximum).contains(value) => Ok(*value),
        _ => Err("Runtime-map coordinate is outside the bound"),
    }
}

pub(super) fn bounded_id_array<'a>(
    object: &'a BTreeMap<String, JsonValue>,
    field: &str,
    maximum: usize,
    node_ids: &BTreeSet<String>,
) -> Result<Vec<&'a str>, &'static str> {
    let values = object
        .get(field)
        .and_then(JsonValue::as_array)
        .ok_or("Runtime-map identity collection is missing")?;
    if values.len() > maximum {
        return Err("Runtime-map identity collection exceeds the bound");
    }
    let mut seen = BTreeSet::new();
    values
        .iter()
        .map(|value| {
            let id = value
                .as_string()
                .filter(|id| valid_identity(id, MAX_ID_BYTES))
                .ok_or("Runtime-map identity collection contains an invalid ID")?;
            if !node_ids.contains(id) || !seen.insert(id) {
                return Err("Runtime-map identity collection contains an invalid ID");
            }
            Ok(id)
        })
        .collect()
}

pub(super) fn has_cycle(adjacency: &BTreeMap<String, Vec<String>>) -> bool {
    let mut indegree = adjacency
        .keys()
        .map(|key| (key.clone(), 0_usize))
        .collect::<BTreeMap<_, _>>();
    for targets in adjacency.values() {
        for target in targets {
            if let Some(degree) = indegree.get_mut(target) {
                *degree += 1;
            }
        }
    }
    let mut queue = VecDeque::new();
    for (id, degree) in &indegree {
        if *degree == 0 {
            queue.push_back(id.clone());
        }
    }
    let mut visited = 0;
    while let Some(id) = queue.pop_front() {
        visited += 1;
        if let Some(targets) = adjacency.get(&id) {
            for target in targets {
                if let Some(degree) = indegree.get_mut(target) {
                    *degree -= 1;
                    if *degree == 0 {
                        queue.push_back(target.clone());
                    }
                }
            }
        }
    }
    visited != adjacency.len()
}
