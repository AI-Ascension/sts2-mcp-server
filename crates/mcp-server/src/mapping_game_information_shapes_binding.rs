// SPDX-License-Identifier: MIT

use crate::json::JsonValue;

use super::{bounded_object_number, enum_value, exact, identity, values};

pub(super) fn validate(value: &JsonValue) -> Result<(), &'static str> {
    let object = exact(
        value,
        &[
            "mode",
            "content_manifest_id",
            "locale",
            "visibility_scope",
            "instance_ref",
            "snapshot_ref",
        ],
    )?;
    enum_value(object, "mode", &["static", "live"])?;
    identity(object, "content_manifest_id")?;
    let locale = object
        .get("locale")
        .and_then(JsonValue::as_string)
        .ok_or("binding locale is invalid")?;
    let mut locale_parts = locale.split('-');
    if locale.len() < 2
        || locale.len() > 35
        || !locale_parts.next().is_some_and(|part| {
            (2..=3).contains(&part.len()) && part.bytes().all(|byte| byte.is_ascii_alphabetic())
        })
        || locale_parts.any(|part| {
            !(1..=8).contains(&part.len()) || !part.bytes().all(|byte| byte.is_ascii_alphanumeric())
        })
    {
        return Err("binding locale is invalid");
    }
    identity(object, "visibility_scope")?;
    let mode = object
        .get("mode")
        .and_then(JsonValue::as_string)
        .ok_or("binding mode is invalid")?;
    match object.get("instance_ref") {
        Some(JsonValue::Null) => {}
        Some(JsonValue::Object(_)) => {
            values::validate_instance(object.get("instance_ref").ok_or("instance is missing")?)?;
        }
        _ => return Err("binding instance_ref is invalid"),
    }
    match object.get("snapshot_ref") {
        Some(JsonValue::Null) => {}
        Some(JsonValue::Object(snapshot)) => {
            if snapshot.len() != 3
                || !["snapshot_id", "instance_ref", "state_generation"]
                    .iter()
                    .all(|key| snapshot.contains_key(*key))
            {
                return Err("snapshot_ref has unknown or missing fields");
            }
            identity(snapshot, "snapshot_id")?;
            let instance = values::validate_instance(
                snapshot
                    .get("instance_ref")
                    .ok_or("snapshot instance is missing")?,
            )?;
            bounded_object_number(snapshot, "state_generation")?;
            if object.get("instance_ref") != Some(&instance) {
                return Err("snapshot instance does not match binding");
            }
        }
        _ => return Err("binding snapshot_ref is invalid"),
    }
    let has_instance = !matches!(object.get("instance_ref"), Some(JsonValue::Null));
    let has_snapshot = !matches!(object.get("snapshot_ref"), Some(JsonValue::Null));
    if (mode == "static" && (has_instance || has_snapshot))
        || (mode == "live" && (!has_instance || !has_snapshot))
    {
        return Err("binding references do not match its mode");
    }
    Ok(())
}
