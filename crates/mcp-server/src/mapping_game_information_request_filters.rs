// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::json::JsonValue;
use crate::protocol_artifact_game_information::{
    GAME_INFORMATION_MAX_CURSOR_BYTES, GAME_INFORMATION_MAX_PAGE_BYTES,
    GAME_INFORMATION_MAX_PAGE_ITEMS, GAME_INFORMATION_MAX_TEXT_BYTES,
};

use super::refs;
use super::{safe_text, valid_identity};

const FIELD_NAMES: [&str; 10] = [
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
];

pub(super) fn limits(arguments: &BTreeMap<String, JsonValue>) -> Result<JsonValue, &'static str> {
    let page_items = super::integer(arguments, "page_items")?;
    let item_bytes = super::integer(arguments, "item_bytes")?;
    let page_bytes = super::integer(arguments, "page_bytes")?;
    let text_bytes = super::integer(arguments, "text_bytes")?;
    if !(1..=GAME_INFORMATION_MAX_PAGE_ITEMS).contains(&page_items)
        || !(1..=GAME_INFORMATION_MAX_PAGE_BYTES).contains(&item_bytes)
        || !(1..=GAME_INFORMATION_MAX_PAGE_BYTES).contains(&page_bytes)
        || !(1..=GAME_INFORMATION_MAX_TEXT_BYTES).contains(&text_bytes)
    {
        return Err("page or text limits exceed the game-information bounds");
    }
    Ok(JsonValue::object([
        (String::from("page_items"), JsonValue::Number(page_items)),
        (String::from("item_bytes"), JsonValue::Number(item_bytes)),
        (String::from("page_bytes"), JsonValue::Number(page_bytes)),
        (String::from("text_bytes"), JsonValue::Number(text_bytes)),
    ]))
}

pub(super) fn fields(arguments: &BTreeMap<String, JsonValue>) -> Result<JsonValue, &'static str> {
    let values = arguments
        .get("fields")
        .and_then(JsonValue::as_array)
        .ok_or("fields must be an array")?;
    if values.len() > 32 {
        return Err("fields exceeds the 32-item bound");
    }
    if values.iter().enumerate().any(|(index, value)| {
        !value
            .as_string()
            .is_some_and(|field| FIELD_NAMES.contains(&field))
            || values[..index].contains(value)
    }) {
        return Err("fields contains an unsupported or duplicate field");
    }
    Ok(JsonValue::Array(values.to_vec()))
}

pub(super) fn filters(arguments: &BTreeMap<String, JsonValue>) -> Result<JsonValue, &'static str> {
    let display_name = nullable_text(arguments, "display_name")?;
    let namespaced_ids = string_list(arguments, "namespaced_ids")?;
    let definition_refs = definition_list(arguments, "definition_refs")?;
    let instance_ids = string_list(arguments, "instance_ids")?;
    Ok(JsonValue::object([
        (String::from("display_name"), display_name),
        (String::from("namespaced_ids"), namespaced_ids),
        (String::from("definition_refs"), definition_refs),
        (String::from("instance_ids"), instance_ids),
    ]))
}

fn definition_list(
    arguments: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<JsonValue, &'static str> {
    let Some(value) = arguments.get(key) else {
        return Ok(JsonValue::Array(Vec::new()));
    };
    let JsonValue::Array(values) = value else {
        return Err("definition_refs must be an array");
    };
    if values.len() > 64 {
        return Err("definition_refs exceeds the 64-item bound");
    }
    values
        .iter()
        .map(refs::validate_definition)
        .collect::<Result<Vec<_>, _>>()
        .map(JsonValue::Array)
}

fn string_list(
    arguments: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<JsonValue, &'static str> {
    let Some(value) = arguments.get(key) else {
        return Ok(JsonValue::Array(Vec::new()));
    };
    let JsonValue::Array(values) = value else {
        return Err("identifier filters must be arrays");
    };
    if values.len() > 64 {
        return Err("identifier filter exceeds the 64-item bound");
    }
    values
        .iter()
        .map(|value| {
            let text = value
                .as_string()
                .ok_or("identifier filter must contain strings")?;
            if !valid_identity(text) {
                return Err("identifier filter contains an unsafe value");
            }
            Ok(JsonValue::string(text))
        })
        .collect::<Result<Vec<_>, _>>()
        .map(JsonValue::Array)
}

fn nullable_text(
    arguments: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<JsonValue, &'static str> {
    match arguments.get(key) {
        None | Some(JsonValue::Null) => Ok(JsonValue::Null),
        Some(JsonValue::String(value))
            if !value.is_empty() && value.len() <= 1_024 && safe_text(value) =>
        {
            Ok(JsonValue::string(value))
        }
        _ => Err("display_name must be null or bounded text"),
    }
}

pub(super) fn nullable_cursor(
    arguments: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<JsonValue, &'static str> {
    match arguments.get(key) {
        None | Some(JsonValue::Null) => Ok(JsonValue::Null),
        Some(JsonValue::String(value))
            if !value.is_empty()
                && value.len() <= GAME_INFORMATION_MAX_CURSOR_BYTES
                && value.bytes().all(|byte| {
                    byte.is_ascii_alphanumeric()
                        || matches!(byte, b'.' | b'_' | b'~' | b':' | b'/' | b'+' | b'=' | b'-')
                }) =>
        {
            Ok(JsonValue::string(value))
        }
        _ => Err("cursor is null or outside the game-information bound"),
    }
}

pub(super) fn nullable_definition(
    arguments: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<Option<JsonValue>, &'static str> {
    match arguments.get(key) {
        None | Some(JsonValue::Null) => Ok(None),
        Some(value) => refs::validate_definition(value).map(Some),
    }
}

pub(super) fn nullable_instance(
    arguments: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<Option<JsonValue>, &'static str> {
    match arguments.get(key) {
        None | Some(JsonValue::Null) => Ok(None),
        Some(value) => refs::validate_instance(value).map(Some),
    }
}

pub(super) fn nullable_snapshot(
    arguments: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<Option<JsonValue>, &'static str> {
    match arguments.get(key) {
        None | Some(JsonValue::Null) => Ok(None),
        Some(value) => refs::validate_snapshot(value).map(Some),
    }
}

pub(super) fn nullable_parent(
    arguments: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<Option<JsonValue>, &'static str> {
    match arguments.get(key) {
        None | Some(JsonValue::Null) => Ok(None),
        Some(value) => refs::validate_parent(value).map(Some),
    }
}
