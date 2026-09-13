// SPDX-License-Identifier: MIT

use crate::json::JsonValue;

use super::helpers::{
    EMPTY_ITEMS_PAYLOAD_BYTES, cursor_binding_for, validate_accounting, validate_ordering,
};
use super::values::validate_item;
use super::{enum_value, number};

pub(super) fn validate(
    value: &JsonValue,
    query: &JsonValue,
    mode: &str,
) -> Result<(), &'static str> {
    let object = super::exact(
        value,
        &[
            "items",
            "next_cursor",
            "cursor_binding",
            "final_page",
            "total_count_known",
            "total_count",
            "coverage",
            "ordering",
            "limits",
            "accounting",
        ],
    )?;
    let query_object = query.as_object().ok_or("query is not an object")?;
    let query_limits = query_object
        .get("limits")
        .ok_or("query limits are missing")?;
    if object.get("limits") != Some(query_limits) {
        return Err("page limits do not match the query");
    }
    let items = object
        .get("items")
        .and_then(JsonValue::as_array)
        .ok_or("page items must be an array")?;
    let page_items = query_limits
        .as_object()
        .and_then(|limits| limits.get("page_items"))
        .and_then(number)
        .ok_or("page_items is missing")?;
    if items.len() as i64 > page_items || items.len() > 128 {
        return Err("page item count exceeds the declared bound");
    }
    let binding = query_object
        .get("binding")
        .ok_or("query binding is missing")?;
    let target_definition = query_object
        .get("target")
        .and_then(JsonValue::as_object)
        .and_then(|target| target.get("definition_ref"))
        .ok_or("query target definition is missing")?;
    for item in items {
        validate_item(
            item,
            binding,
            target_definition,
            query_object.get("entity_kind"),
            mode,
        )?;
    }
    let next_cursor = object.get("next_cursor").ok_or("next cursor is missing")?;
    if let Some(cursor) = next_cursor.as_string() {
        if !super::valid_cursor(cursor) {
            return Err("next cursor is outside the protocol bound");
        }
    } else if next_cursor != &JsonValue::Null {
        return Err("next_cursor must be null or a cursor");
    }
    let final_page = object.get("final_page") == Some(&JsonValue::Bool(true));
    if object.get("final_page") != Some(&JsonValue::Bool(true))
        && object.get("final_page") != Some(&JsonValue::Bool(false))
    {
        return Err("final_page must be boolean");
    }
    let cursor_binding = object
        .get("cursor_binding")
        .ok_or("cursor binding is missing")?;
    match (final_page, next_cursor, cursor_binding) {
        (true, JsonValue::Null, JsonValue::Null) => {}
        (false, JsonValue::String(_), JsonValue::Object(value)) => {
            let expected = cursor_binding_for(query)?;
            if value
                != expected
                    .as_object()
                    .ok_or("cursor binding is not an object")?
            {
                return Err("cursor binding does not match the query");
            }
        }
        (true, _, _) => return Err("final page has invalid continuation state"),
        (false, _, _) => return Err("non-final page has invalid continuation state"),
    }
    enum_value(
        object,
        "coverage",
        &["complete", "partial", "unavailable", "not_observable"],
    )?;
    validate_ordering(object.get("ordering").ok_or("ordering is missing")?)?;
    validate_accounting(
        object.get("accounting").ok_or("accounting is missing")?,
        value,
        items,
        query_limits,
    )?;
    let total_known = object.get("total_count_known") == Some(&JsonValue::Bool(true));
    match (total_known, object.get("total_count")) {
        (true, Some(JsonValue::Number(value))) if (0..=1_000_000).contains(value) => {}
        (false, Some(JsonValue::Null)) => {}
        _ => return Err("total count state is invalid"),
    }
    if matches!(object.get("coverage"), Some(JsonValue::String(value)) if
        matches!(value.as_str(), "unavailable" | "not_observable"))
        && (!items.is_empty()
            || next_cursor != &JsonValue::Null
            || cursor_binding != &JsonValue::Null
            || !final_page
            || object.get("total_count_known") != Some(&JsonValue::Bool(false))
            || object.get("total_count") != Some(&JsonValue::Null)
            || object
                .get("accounting")
                .and_then(JsonValue::as_object)
                .is_none_or(|accounting| {
                    accounting.get("item_count") != Some(&JsonValue::Number(0))
                        || accounting.get("item_bytes") != Some(&JsonValue::Number(0))
                        // Even an unavailable page has an empty `items` array;
                        // its canonical UTF-8 payload is therefore `[]` (2 bytes).
                        || accounting.get("payload_bytes")
                            != Some(&JsonValue::Number(EMPTY_ITEMS_PAYLOAD_BYTES))
                        || accounting.get("text_bytes") != Some(&JsonValue::Number(0))
                }))
    {
        return Err("unavailable page carries result data");
    }
    Ok(())
}
