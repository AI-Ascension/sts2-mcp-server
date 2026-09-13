// SPDX-License-Identifier: MIT

use crate::json::JsonValue;

use super::{
    ENTITY_KINDS, MAX_REASON, bounded_object_number, enum_value, exact, identity, safe_text,
    valid_identity,
};

const MAX_TEXT: usize = 1_024;

pub(super) fn validate_item(
    value: &JsonValue,
    binding: &JsonValue,
    target_definition: &JsonValue,
    entity_kind: Option<&JsonValue>,
    mode: &str,
) -> Result<(), &'static str> {
    let object = exact(value, &["definition_ref", "instance_ref", "fields"])?;
    let definition = validate_definition(
        object
            .get("definition_ref")
            .ok_or("item definition reference is missing")?,
    )?;
    let binding_object = binding.as_object().ok_or("binding is not an object")?;
    let definition_object = definition
        .as_object()
        .ok_or("item definition is not an object")?;
    if definition_object.get("content_manifest_id") != binding_object.get("content_manifest_id")
        || definition_object.get("entity_kind") != entity_kind
        || (target_definition != &JsonValue::Null && target_definition != &definition)
    {
        return Err("item definition reference is foreign to the query");
    }
    match (mode, object.get("instance_ref")) {
        ("static", Some(JsonValue::Null)) => {}
        ("live", Some(JsonValue::Object(_))) => {
            let instance =
                validate_instance(object.get("instance_ref").ok_or("instance is missing")?)?;
            if binding_object.get("instance_ref") != Some(&instance) {
                return Err("item instance reference is foreign to the snapshot");
            }
        }
        _ => return Err("item instance reference does not match query mode"),
    }
    let fields = object
        .get("fields")
        .and_then(JsonValue::as_array)
        .ok_or("item fields must be an array")?;
    if fields.len() > 64 {
        return Err("item has too many fields");
    }
    let mut previous = None;
    for field in fields {
        let name = field
            .as_object()
            .and_then(|field| field.get("name"))
            .and_then(JsonValue::as_string)
            .ok_or("item field name is missing")?;
        if previous.is_some_and(|previous: &str| name <= previous) {
            return Err("item fields are not in deterministic order");
        }
        previous = Some(name);
        validate_field(field)?;
    }
    Ok(())
}

fn validate_source(value: &JsonValue) -> Result<(), &'static str> {
    let object = exact(value, &["kind", "ref"])?;
    enum_value(
        object,
        "kind",
        &[
            "content_manifest",
            "game_mod",
            "gateway_projection",
            "synthetic",
        ],
    )?;
    match object.get("ref") {
        Some(JsonValue::Null) => Ok(()),
        Some(JsonValue::String(value)) if valid_identity(value) => Ok(()),
        _ => Err("field source reference is invalid"),
    }
}

fn validate_field(value: &JsonValue) -> Result<(), &'static str> {
    let object = exact(
        value,
        &[
            "name",
            "kind",
            "availability",
            "value",
            "unit",
            "source",
            "reason",
        ],
    )?;
    enum_value(
        object,
        "name",
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
    )?;
    let kind = enum_value(
        object,
        "kind",
        &[
            "boolean",
            "definition_ref",
            "integer",
            "instance_ref",
            "text",
            "text_list",
        ],
    )?;
    let availability = enum_value(
        object,
        "availability",
        &[
            "available",
            "unavailable",
            "not_observable",
            "redacted",
            "unsupported",
            "missing",
        ],
    )?;
    let value = object.get("value").ok_or("field value is missing")?;
    if availability == "available" {
        validate_field_value(value, kind)?;
        if object.get("reason") != Some(&JsonValue::Null) {
            return Err("available field has a reason");
        }
    } else {
        if value != &JsonValue::Null {
            return Err("unavailable field carries a value");
        }
        let reason = object
            .get("reason")
            .and_then(JsonValue::as_string)
            .ok_or("unavailable field has no reason")?;
        if reason.is_empty() || reason.len() > MAX_REASON || !safe_text(reason) {
            return Err("field reason is outside the bound");
        }
    }
    match object.get("unit") {
        Some(JsonValue::Null) => {}
        Some(JsonValue::String(unit))
            if ["none", "count", "gold", "hp", "block", "percent"].contains(&unit.as_str()) => {}
        _ => return Err("field unit is invalid"),
    }
    validate_source(object.get("source").ok_or("field source is missing")?)
}

fn validate_field_value(value: &JsonValue, kind: &str) -> Result<(), &'static str> {
    match (kind, value) {
        ("boolean", JsonValue::Bool(_)) => Ok(()),
        ("integer", JsonValue::Number(value))
            if (-2_147_483_648..=2_147_483_647).contains(value) =>
        {
            Ok(())
        }
        ("text", JsonValue::String(value))
            if !value.is_empty() && value.len() <= MAX_TEXT && safe_text(value) =>
        {
            Ok(())
        }
        ("text_list", JsonValue::Array(values)) if values.len() <= 64 => {
            if values.iter().all(|value| {
                matches!(
                    value,
                    JsonValue::String(text)
                        if !text.is_empty() && text.len() <= MAX_TEXT && safe_text(text)
                )
            }) {
                Ok(())
            } else {
                Err("text list contains invalid text")
            }
        }
        ("definition_ref", JsonValue::Object(_)) => validate_definition(value).map(|_| ()),
        ("instance_ref", JsonValue::Object(_)) => validate_instance(value).map(|_| ()),
        _ => Err("field kind and value do not match"),
    }
}

pub(super) fn validate_definition(value: &JsonValue) -> Result<JsonValue, &'static str> {
    let object = exact(
        value,
        &[
            "content_manifest_id",
            "entity_kind",
            "namespaced_id",
            "variant",
        ],
    )?;
    let content = identity(object, "content_manifest_id")?;
    let kind = enum_value(object, "entity_kind", &ENTITY_KINDS)?;
    let namespaced = identity(object, "namespaced_id")?;
    let variant = match object.get("variant") {
        Some(JsonValue::Null) => JsonValue::Null,
        Some(JsonValue::String(value)) if valid_identity(value) => JsonValue::string(value),
        _ => return Err("definition variant is invalid"),
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
    let object = exact(
        value,
        &["instance_id", "run_id", "epoch", "entity_kind", "entity_id"],
    )?;
    let instance = identity(object, "instance_id")?;
    let run = identity(object, "run_id")?;
    let epoch = bounded_object_number(object, "epoch")?;
    let kind = enum_value(object, "entity_kind", &ENTITY_KINDS)?;
    let entity = identity(object, "entity_id")?;
    Ok(JsonValue::object([
        (String::from("instance_id"), JsonValue::string(instance)),
        (String::from("run_id"), JsonValue::string(run)),
        (String::from("epoch"), JsonValue::Number(epoch)),
        (String::from("entity_kind"), JsonValue::string(kind)),
        (String::from("entity_id"), JsonValue::string(entity)),
    ]))
}
