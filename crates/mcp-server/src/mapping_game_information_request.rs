// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::catalog::MAX_IDENTIFIER_BYTES;
use crate::json::JsonValue;

use super::{CallKind, GameInformationContext, identity, read_string};

#[path = "mapping_game_information_request_filters.rs"]
mod filters;
#[path = "mapping_game_information_request_refs.rs"]
mod refs;

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

pub(super) fn query(
    arguments: &BTreeMap<String, JsonValue>,
    kind: CallKind,
    context: GameInformationContext,
) -> Result<(GameInformationContext, JsonValue), &'static str> {
    let content_manifest_id = identity(arguments, "content_manifest_id")?;
    let locale = locale(arguments, "locale")?;
    let visibility_scope = identity(arguments, "visibility_scope")?;
    let entity_kind = enum_value(arguments, "entity_kind", &ENTITY_KINDS)?;
    let projection = enum_value(arguments, "projection", &["summary", "standard", "full"])?;
    let detail_level = enum_value(arguments, "detail_level", &["summary", "standard", "full"])?;
    let fields = filters::fields(arguments)?;
    let limits = filters::limits(arguments)?;
    let cursor = filters::nullable_cursor(arguments, "cursor")?;
    let filters = filters::filters(arguments)?;
    let definition_ref = filters::nullable_definition(arguments, "definition_ref")?;
    let instance_ref = filters::nullable_instance(arguments, "instance_ref")?;
    let snapshot_ref = filters::nullable_snapshot(arguments, "snapshot_ref")?;
    let parent_observation = filters::nullable_parent(arguments, "parent_observation")?;
    let mode = match kind {
        CallKind::Detail => "live",
        CallKind::Availability => enum_value(arguments, "mode", &["static", "live"])?,
        _ => "static",
    };
    let binding_instance = if mode == "live" {
        let instance = instance_ref
            .clone()
            .ok_or("live query requires instance_ref")?;
        let snapshot = snapshot_ref
            .as_ref()
            .ok_or("live query requires snapshot_ref")?;
        if snapshot_instance(snapshot) != Some(&instance) {
            return Err("snapshot_ref instance_ref does not match instance_ref");
        }
        if let Some(parent) = parent_observation.as_ref() {
            if parent_instance(parent) != Some(&instance)
                || parent_snapshot(parent) != Some(snapshot)
                || parent_generation(parent) != snapshot_generation(snapshot)
            {
                return Err("parent_observation does not match the live snapshot");
            }
        } else {
            return Err("live query requires parent_observation");
        }
        if instance_authority(&instance, &context.instance_id).is_err() {
            return Err("live instance_ref is foreign to the gateway target");
        }
        Some(instance)
    } else {
        if instance_ref.is_some() || snapshot_ref.is_some() || parent_observation.is_some() {
            return Err("static query cannot carry live snapshot fields");
        }
        None
    };
    if kind == CallKind::Get && definition_ref.is_none() {
        return Err("get requires definition_ref");
    }
    if matches!(kind, CallKind::List | CallKind::Search) && definition_ref.is_some() {
        return Err("list/search cannot target one definition");
    }
    if let Some(definition) = definition_ref.as_ref()
        && (definition_field(definition, "content_manifest_id")
            != Some(&JsonValue::String(content_manifest_id.to_owned()))
            || definition_field(definition, "entity_kind")
                != Some(&JsonValue::String(entity_kind.to_owned())))
    {
        return Err("definition_ref does not match the query binding");
    }
    if let Some(instance) = binding_instance.as_ref()
        && instance_field(instance, "entity_kind")
            != Some(&JsonValue::String(entity_kind.to_owned()))
    {
        return Err("instance_ref entity_kind does not match the query");
    }
    let query = JsonValue::object([
        (String::from("query_kind"), JsonValue::string(kind.as_str())),
        (String::from("entity_kind"), JsonValue::string(entity_kind)),
        (
            String::from("target"),
            JsonValue::object([
                (
                    String::from("definition_ref"),
                    definition_ref.unwrap_or(JsonValue::Null),
                ),
                (
                    String::from("instance_ref"),
                    binding_instance.clone().unwrap_or(JsonValue::Null),
                ),
            ]),
        ),
        (String::from("filters"), filters),
        (String::from("projection"), JsonValue::string(projection)),
        (
            String::from("detail_level"),
            JsonValue::string(detail_level),
        ),
        (String::from("fields"), fields),
        (
            String::from("binding"),
            JsonValue::object([
                (String::from("mode"), JsonValue::string(mode)),
                (
                    String::from("content_manifest_id"),
                    JsonValue::string(content_manifest_id),
                ),
                (String::from("locale"), JsonValue::string(locale)),
                (
                    String::from("visibility_scope"),
                    JsonValue::string(visibility_scope),
                ),
                (
                    String::from("instance_ref"),
                    binding_instance.clone().unwrap_or(JsonValue::Null),
                ),
                (
                    String::from("snapshot_ref"),
                    snapshot_ref.unwrap_or(JsonValue::Null),
                ),
            ]),
        ),
        (
            String::from("parent_observation"),
            parent_observation.unwrap_or(JsonValue::Null),
        ),
        (String::from("limits"), limits),
        (String::from("cursor"), cursor),
    ]);
    if query.to_json().len() > MAX_IDENTIFIER_BYTES * 128 {
        return Err("game-information query exceeds the request bound");
    }
    Ok((context, query))
}

fn integer(arguments: &BTreeMap<String, JsonValue>, key: &str) -> Result<i64, &'static str> {
    refs::integer_object(arguments, key)
}

fn enum_value<'a>(
    arguments: &'a BTreeMap<String, JsonValue>,
    key: &str,
    values: &[&str],
) -> Result<&'a str, &'static str> {
    let value = read_string(arguments, key).ok_or("enum argument must be a string")?;
    values
        .contains(&value)
        .then_some(value)
        .ok_or("enum value is unsupported")
}

fn locale<'a>(
    arguments: &'a BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<&'a str, &'static str> {
    let value = read_string(arguments, key).ok_or("locale must be a string")?;
    if value.len() > 35 || value.len() < 2 {
        return Err("locale is outside the protocol bound");
    }
    let mut parts = value.split('-');
    if !parts.next().is_some_and(|part| {
        (2..=3).contains(&part.len()) && part.bytes().all(|byte| byte.is_ascii_alphabetic())
    }) || parts.any(|part| {
        !(1..=8).contains(&part.len()) || !part.bytes().all(|byte| byte.is_ascii_alphanumeric())
    }) {
        return Err("locale is outside the protocol bound");
    }
    Ok(value)
}

fn valid_identity(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_IDENTIFIER_BYTES
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'/' | b'-')
        })
}

fn safe_text(value: &str) -> bool {
    value.chars().all(|character| {
        !matches!(
            character,
            '\u{0000}'..='\u{001F}' | '\u{007F}'..='\u{009F}'
        )
    })
}

fn definition_field<'a>(value: &'a JsonValue, key: &str) -> Option<&'a JsonValue> {
    value.as_object()?.get(key)
}

fn instance_field<'a>(value: &'a JsonValue, key: &str) -> Option<&'a JsonValue> {
    value.as_object()?.get(key)
}

fn snapshot_instance(value: &JsonValue) -> Option<&JsonValue> {
    value.as_object()?.get("instance_ref")
}

fn snapshot_generation(value: &JsonValue) -> Option<&JsonValue> {
    value.as_object()?.get("state_generation")
}

fn parent_instance(value: &JsonValue) -> Option<&JsonValue> {
    value.as_object()?.get("instance_ref")
}

fn parent_snapshot(value: &JsonValue) -> Option<&JsonValue> {
    value.as_object()?.get("snapshot_ref")
}

fn parent_generation(value: &JsonValue) -> Option<&JsonValue> {
    value.as_object()?.get("state_generation")
}

fn instance_authority(value: &JsonValue, expected: &str) -> Result<(), ()> {
    value
        .as_object()
        .and_then(|object| object.get("instance_id"))
        .and_then(JsonValue::as_string)
        .filter(|value| *value == expected)
        .map_or(Err(()), |_| Ok(()))
}
