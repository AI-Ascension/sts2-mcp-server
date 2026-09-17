// SPDX-License-Identifier: MIT
#![allow(clippy::too_many_arguments)]

use std::collections::BTreeSet;

use crate::json::JsonValue;

use super::RequestContext;
#[path = "mapping_game_information_live_observation_bootstrap_helpers.rs"]
mod helpers;
pub(super) use helpers::{
    MAX_INTEGER, bounded, bounded_int, definition_ref, definition_ref_value, error_category,
    exact_object, identity, instance_ref_value, locale, number, status_category, valid_identity,
};

pub(super) fn validate_parent(
    parent: &JsonValue,
    context: &RequestContext,
) -> Result<(), &'static str> {
    let object = exact_object(
        parent,
        &["instance_ref", "snapshot_ref", "state_generation"],
    )?;
    let instance = instance_ref_value(
        object
            .get("instance_ref")
            .ok_or("bootstrap parent instance_ref is missing")?,
    )?;
    let snapshot = exact_object(
        object
            .get("snapshot_ref")
            .ok_or("bootstrap parent snapshot_ref is missing")?,
        &["snapshot_id", "instance_ref", "state_generation"],
    )?;
    let snapshot_instance = instance_ref_value(
        snapshot
            .get("instance_ref")
            .ok_or("bootstrap snapshot instance_ref is missing")?,
    )?;
    if !snapshot
        .get("snapshot_id")
        .and_then(JsonValue::as_string)
        .is_some_and(valid_identity)
    {
        return Err("bootstrap snapshot identity is invalid");
    }
    number(snapshot, "state_generation", 0, MAX_INTEGER)?;
    number(object, "state_generation", 0, MAX_INTEGER)?;
    if instance != snapshot_instance
        || object.get("state_generation") != snapshot.get("state_generation")
    {
        return Err("bootstrap parent observation does not match its snapshot");
    }
    let instance_object = instance
        .as_object()
        .ok_or("bootstrap parent instance_ref is malformed")?;
    if instance_object
        .get("instance_id")
        .and_then(JsonValue::as_string)
        != Some(context.instance_id.as_str())
        || instance_object.get("run_id").and_then(JsonValue::as_string)
            != Some(context.run_id.as_str())
    {
        return Err("bootstrap parent observation is foreign to the requested scope");
    }
    Ok(())
}

pub(super) fn validate_visible_entity(
    value: &JsonValue,
    parent_instance: &JsonValue,
    parent_generation: &JsonValue,
    parent_snapshot: &JsonValue,
    selector_definition: &JsonValue,
    selector_instance: &JsonValue,
    identities: &mut BTreeSet<String>,
    context: &RequestContext,
    max_item_bytes: i64,
) -> Result<(), &'static str> {
    if value.to_json().len() > max_item_bytes as usize {
        return Err("bootstrap visible entity exceeds max_item_bytes");
    }
    let object = exact_object(value, &["instance_ref", "snapshot_ref", "definition_ref"])?;
    let instance = instance_ref_value(
        object
            .get("instance_ref")
            .ok_or("bootstrap visible instance_ref is missing")?,
    )?;
    let snapshot = exact_object(
        object
            .get("snapshot_ref")
            .ok_or("bootstrap visible snapshot_ref is missing")?,
        &["snapshot_id", "instance_ref", "state_generation"],
    )?;
    let snapshot_instance = instance_ref_value(
        snapshot
            .get("instance_ref")
            .ok_or("bootstrap visible snapshot instance_ref is missing")?,
    )?;
    if !snapshot
        .get("snapshot_id")
        .and_then(JsonValue::as_string)
        .is_some_and(valid_identity)
    {
        return Err("bootstrap visible snapshot identity is invalid");
    }
    number(snapshot, "state_generation", 0, MAX_INTEGER)?;
    if instance != snapshot_instance
        || snapshot.get("snapshot_id") != Some(parent_snapshot)
        || snapshot.get("state_generation") != Some(parent_generation)
    {
        return Err("bootstrap visible entity snapshot is incoherent");
    }
    let instance_object = instance
        .as_object()
        .ok_or("bootstrap visible instance_ref is malformed")?;
    if instance_object
        .get("instance_id")
        .and_then(JsonValue::as_string)
        != Some(context.instance_id.as_str())
        || instance_object.get("run_id").and_then(JsonValue::as_string)
            != Some(context.run_id.as_str())
    {
        return Err("bootstrap visible entity is foreign to the requested scope");
    }
    let parent_object = parent_instance
        .as_object()
        .ok_or("bootstrap parent instance_ref is malformed")?;
    if instance_object.get("epoch") != parent_object.get("epoch") {
        return Err("bootstrap visible entity crosses native instance epoch");
    }
    let entity_id = instance_object
        .get("entity_id")
        .and_then(JsonValue::as_string)
        .ok_or("bootstrap visible entity identity is missing")?;
    if !identities.insert(entity_id.to_owned()) {
        return Err("bootstrap visible entities contain a duplicate instance");
    }
    if selector_instance != &JsonValue::Null && instance != selector_instance.clone() {
        return Err("bootstrap selector instance does not match a visible entity");
    }
    let definition = object
        .get("definition_ref")
        .ok_or("bootstrap visible definition_ref is missing")?;
    if definition != &JsonValue::Null {
        definition_ref_value(definition)?;
        if definition != selector_definition {
            return Err("bootstrap visible definition is foreign to the selector");
        }
        let definition_kind = definition
            .as_object()
            .and_then(|object| object.get("entity_kind"))
            .and_then(JsonValue::as_string)
            .ok_or("bootstrap visible definition kind is missing")?;
        if instance_object
            .get("entity_kind")
            .and_then(JsonValue::as_string)
            != Some(definition_kind)
        {
            return Err("bootstrap visible definition kind mismatches its instance");
        }
    } else if instance == parent_instance.clone() {
        return Err("bootstrap parent entity is missing its definition");
    }
    if instance == parent_instance.clone()
        && snapshot.get("state_generation") != Some(parent_generation)
    {
        return Err("bootstrap parent entity generation mismatch");
    }
    Ok(())
}

pub(super) fn validate_owner(value: &JsonValue) -> Result<(), &'static str> {
    let object = exact_object(
        value,
        &[
            "native_snapshot_owner",
            "content_manifest_owner",
            "instance_fence_owner",
            "authority_epoch_owner",
            "instance_ref_epoch_owner",
            "transport_lease_epoch_role",
        ],
    )?;
    let expected = [
        ("native_snapshot_owner", "sts2-game-mod"),
        ("content_manifest_owner", "sts2-game-mod"),
        ("instance_fence_owner", "sts2-gateway"),
        ("authority_epoch_owner", "sts2-harness"),
        ("instance_ref_epoch_owner", "sts2-game-mod"),
        ("transport_lease_epoch_role", "fence_only"),
    ];
    for (key, value) in expected {
        if object.get(key) != Some(&JsonValue::string(value)) {
            return Err("bootstrap owner provenance is invalid");
        }
    }
    Ok(())
}

pub(super) fn validate_error(value: &JsonValue) -> Result<(), &'static str> {
    let object = exact_object(value, &["code", "field", "reason", "retryable"])?;
    let code = object
        .get("code")
        .and_then(JsonValue::as_string)
        .ok_or("bootstrap error code is missing")?;
    if !matches!(
        code,
        "unsupported_version"
            | "invalid_request"
            | "invalid_bounds"
            | "unavailable"
            | "not_observable"
            | "stale_snapshot"
            | "invalid_binding"
            | "ambiguous_entity"
    ) {
        return Err("bootstrap error code is unsupported");
    }
    if object.get("field") != Some(&JsonValue::Null)
        && !object
            .get("field")
            .and_then(JsonValue::as_string)
            .is_some_and(valid_identity)
    {
        return Err("bootstrap error field is invalid");
    }
    let reason = object
        .get("reason")
        .and_then(JsonValue::as_string)
        .ok_or("bootstrap error reason is missing")?;
    if reason.is_empty()
        || reason.len() > 256
        || reason.chars().any(|character| {
            matches!(
                character,
                '\u{0000}'..='\u{001F}' | '\u{007F}'..='\u{009F}'
            )
        })
        || !matches!(object.get("retryable"), Some(JsonValue::Bool(_)))
    {
        return Err("bootstrap error details are invalid");
    }
    Ok(())
}

pub(super) fn validate_limits(value: &JsonValue) -> Result<(i64, i64, i64), &'static str> {
    let object = exact_object(
        value,
        &[
            "max_visible_entities",
            "max_item_bytes",
            "max_message_bytes",
        ],
    )?;
    let visible = number(object, "max_visible_entities", 1, 64)?;
    let item = number(object, "max_item_bytes", 1, 65_536)?;
    let message = number(object, "max_message_bytes", 1, 262_144)?;
    Ok((visible, item, message))
}

pub(super) fn limits(visible: i64, item: i64, message: i64) -> JsonValue {
    JsonValue::object([
        (
            "max_visible_entities".to_owned(),
            JsonValue::Number(visible),
        ),
        ("max_item_bytes".to_owned(), JsonValue::Number(item)),
        ("max_message_bytes".to_owned(), JsonValue::Number(message)),
    ])
}
