// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::json::JsonValue;

use super::{ENTITY_KINDS, MAX_INTEGER, valid_identity};

pub(super) fn validate_definition(value: &JsonValue) -> Result<JsonValue, &'static str> {
    let object = exact_object(
        value,
        &[
            "content_manifest_id",
            "entity_kind",
            "namespaced_id",
            "variant",
        ],
    )?;
    let content = identity_object(object, "content_manifest_id")?;
    let kind = enum_object(object, "entity_kind", &ENTITY_KINDS)?;
    let namespaced = identity_object(object, "namespaced_id")?;
    let variant = match object.get("variant") {
        Some(JsonValue::Null) => JsonValue::Null,
        Some(JsonValue::String(value)) if valid_identity(value) => JsonValue::string(value),
        _ => return Err("definition_ref variant must be null or an identity"),
    };
    Ok(JsonValue::object([
        (
            String::from("content_manifest_id"),
            JsonValue::string(content),
        ),
        (String::from("entity_kind"), JsonValue::string(kind)),
        (String::from("namespaced_id"), JsonValue::string(namespaced)),
        (String::from("variant"), variant),
    ]))
}

pub(super) fn validate_instance(value: &JsonValue) -> Result<JsonValue, &'static str> {
    let object = exact_object(
        value,
        &["instance_id", "run_id", "epoch", "entity_kind", "entity_id"],
    )?;
    let instance_id = identity_object(object, "instance_id")?;
    let run_id = identity_object(object, "run_id")?;
    let epoch = integer_object(object, "epoch")?;
    let kind = enum_object(object, "entity_kind", &ENTITY_KINDS)?;
    let entity_id = identity_object(object, "entity_id")?;
    Ok(JsonValue::object([
        (String::from("instance_id"), JsonValue::string(instance_id)),
        (String::from("run_id"), JsonValue::string(run_id)),
        (String::from("epoch"), JsonValue::Number(epoch)),
        (String::from("entity_kind"), JsonValue::string(kind)),
        (String::from("entity_id"), JsonValue::string(entity_id)),
    ]))
}

pub(super) fn validate_snapshot(value: &JsonValue) -> Result<JsonValue, &'static str> {
    let object = exact_object(value, &["snapshot_id", "instance_ref", "state_generation"])?;
    let snapshot_id = identity_object(object, "snapshot_id")?;
    let instance = validate_instance(
        object
            .get("instance_ref")
            .ok_or("snapshot_ref requires instance_ref")?,
    )?;
    let generation = integer_object(object, "state_generation")?;
    Ok(JsonValue::object([
        (String::from("snapshot_id"), JsonValue::string(snapshot_id)),
        (String::from("instance_ref"), instance),
        (
            String::from("state_generation"),
            JsonValue::Number(generation),
        ),
    ]))
}

pub(super) fn validate_parent(value: &JsonValue) -> Result<JsonValue, &'static str> {
    let object = exact_object(value, &["instance_ref", "snapshot_ref", "state_generation"])?;
    let instance = validate_instance(
        object
            .get("instance_ref")
            .ok_or("parent_observation requires instance_ref")?,
    )?;
    let snapshot = validate_snapshot(
        object
            .get("snapshot_ref")
            .ok_or("parent_observation requires snapshot_ref")?,
    )?;
    let generation = integer_object(object, "state_generation")?;
    Ok(JsonValue::object([
        (String::from("instance_ref"), instance),
        (String::from("snapshot_ref"), snapshot),
        (
            String::from("state_generation"),
            JsonValue::Number(generation),
        ),
    ]))
}

fn exact_object<'a>(
    value: &'a JsonValue,
    required: &[&str],
) -> Result<&'a BTreeMap<String, JsonValue>, &'static str> {
    let object = value
        .as_object()
        .ok_or("nested game-information value must be an object")?;
    if object.len() != required.len() || required.iter().any(|key| !object.contains_key(*key)) {
        return Err("nested game-information object has unknown or missing fields");
    }
    Ok(object)
}

fn identity_object<'a>(
    object: &'a BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<&'a str, &'static str> {
    let value = object
        .get(key)
        .and_then(JsonValue::as_string)
        .ok_or("identity must be a string")?;
    if valid_identity(value) {
        Ok(value)
    } else {
        Err("identity is empty, unsafe, or oversized")
    }
}

fn enum_object<'a>(
    object: &'a BTreeMap<String, JsonValue>,
    key: &str,
    values: &[&str],
) -> Result<&'a str, &'static str> {
    let value = object
        .get(key)
        .and_then(JsonValue::as_string)
        .ok_or("enum must be a string")?;
    if values.contains(&value) {
        Ok(value)
    } else {
        Err("enum value is unsupported")
    }
}

pub(super) fn integer_object(
    object: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<i64, &'static str> {
    match object.get(key) {
        Some(JsonValue::Number(value)) if (0..=MAX_INTEGER).contains(value) => Ok(*value),
        _ => Err("integer is outside the protocol bound"),
    }
}
