// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::json::JsonValue;

/// Canonical compact JSON encoding of an empty `items` array (`[]`).
pub(super) const EMPTY_ITEMS_PAYLOAD_BYTES: i64 = 2;

pub(super) fn unique(values: &[JsonValue]) -> bool {
    values
        .iter()
        .enumerate()
        .all(|(index, value)| !values[..index].iter().any(|previous| previous == value))
}

pub(super) fn validate_snapshot_policy(value: &JsonValue) -> Result<(), &'static str> {
    let object = super::exact(
        value,
        &[
            "supports_live",
            "lifetime_generations",
            "max_retained_snapshots",
            "expiry_behavior",
            "invalidated_by",
        ],
    )?;
    if object.get("supports_live") != Some(&JsonValue::Bool(true))
        || object.get("expiry_behavior")
            != Some(&JsonValue::String(String::from("reject_stale_snapshot")))
    {
        return Err("snapshot policy is unsupported");
    }
    super::bounded_number(object, "lifetime_generations", 1, 65_536)?;
    super::bounded_number(object, "max_retained_snapshots", 1, 64)?;
    let invalidated = object
        .get("invalidated_by")
        .and_then(JsonValue::as_array)
        .ok_or("invalidated_by is invalid")?;
    if invalidated.len() != 6 || !unique(invalidated) {
        return Err("snapshot invalidation list is invalid");
    }
    for value in invalidated {
        if ![
            "restore",
            "restart",
            "profile_change",
            "content_change",
            "run_change",
            "epoch_change",
        ]
        .contains(
            &value
                .as_string()
                .ok_or("snapshot invalidation kind is invalid")?,
        ) {
            return Err("snapshot invalidation kind is invalid");
        }
    }
    Ok(())
}

pub(super) fn validate_limits(value: &JsonValue) -> Result<(), &'static str> {
    let object = super::exact(
        value,
        &["page_items", "item_bytes", "page_bytes", "text_bytes"],
    )?;
    super::bounded_number(object, "page_items", 1, 128)?;
    super::bounded_number(object, "item_bytes", 1, 262_144)?;
    super::bounded_number(object, "page_bytes", 1, 262_144)?;
    super::bounded_number(object, "text_bytes", 1, 65_536)?;
    Ok(())
}

pub(super) fn validate_ordering(value: &JsonValue) -> Result<(), &'static str> {
    let object = super::exact(value, &["key", "direction", "algorithm", "deterministic"])?;
    super::enum_value(
        object,
        "key",
        &[
            "definition_ref",
            "instance_ref",
            "display_name",
            "namespaced_id",
        ],
    )?;
    super::enum_value(object, "direction", &["ascending", "descending"])?;
    super::enum_value(
        object,
        "algorithm",
        &["identity_bytes", "unicode_scalar_values"],
    )?;
    if object.get("deterministic") != Some(&JsonValue::Bool(true)) {
        return Err("ordering is not deterministic");
    }
    Ok(())
}

pub(super) fn validate_accounting(
    value: &JsonValue,
    page: &JsonValue,
    items: &[JsonValue],
    limits: &JsonValue,
) -> Result<(), &'static str> {
    let object = super::exact(
        value,
        &[
            "item_count",
            "item_bytes",
            "payload_bytes",
            "page_bytes",
            "text_bytes",
        ],
    )?;
    if object.get("item_count") != Some(&JsonValue::Number(items.len() as i64)) {
        return Err("item accounting does not match the page");
    }
    let expected_item_bytes = items
        .iter()
        .map(|item| item.to_json().len())
        .max()
        .unwrap_or(0);
    let expected_payload_bytes = JsonValue::Array(items.to_vec()).to_json().len();
    let mut page_without_accounting = page.as_object().cloned().ok_or("page is not an object")?;
    page_without_accounting.remove("accounting");
    let expected_page_bytes = JsonValue::Object(page_without_accounting).to_json().len();
    let expected_text_bytes = items.iter().map(text_bytes).sum::<usize>();
    if object.get("item_bytes") != Some(&JsonValue::Number(expected_item_bytes as i64))
        || object.get("payload_bytes") != Some(&JsonValue::Number(expected_payload_bytes as i64))
        || object.get("page_bytes") != Some(&JsonValue::Number(expected_page_bytes as i64))
        || object.get("text_bytes") != Some(&JsonValue::Number(expected_text_bytes as i64))
    {
        return Err("item, page, or text accounting does not match the page");
    }
    let payload = super::bounded_object_number(object, "payload_bytes")?;
    let item = super::bounded_object_number(object, "item_bytes")?;
    let page = super::bounded_object_number(object, "page_bytes")?;
    let text = super::bounded_object_number(object, "text_bytes")?;
    let max_item = limits
        .as_object()
        .and_then(|value| value.get("item_bytes"))
        .and_then(super::number)
        .ok_or("item byte limit is missing")?;
    let max_payload = limits
        .as_object()
        .and_then(|value| value.get("page_bytes"))
        .and_then(super::number)
        .ok_or("page byte limit is missing")?;
    let max_text = limits
        .as_object()
        .and_then(|value| value.get("text_bytes"))
        .and_then(super::number)
        .ok_or("text byte limit is missing")?;
    if item > max_item || payload > max_payload || page > max_payload || text > max_text {
        return Err("page accounting exceeds its declared limits");
    }
    Ok(())
}

fn text_bytes(item: &JsonValue) -> usize {
    item.as_object()
        .and_then(|object| object.get("fields"))
        .and_then(JsonValue::as_array)
        .map(|fields| {
            fields
                .iter()
                .filter_map(JsonValue::as_object)
                .filter(|field| field.get("availability") == Some(&JsonValue::string("available")))
                .map(|field| match field.get("kind") {
                    Some(JsonValue::String(kind)) if kind == "text" => field
                        .get("value")
                        .and_then(JsonValue::as_string)
                        .map_or(0, str::len),
                    Some(JsonValue::String(kind)) if kind == "text_list" => field
                        .get("value")
                        .and_then(JsonValue::as_array)
                        .map_or(0, |values| {
                            values
                                .iter()
                                .filter_map(JsonValue::as_string)
                                .map(str::len)
                                .sum()
                        }),
                    _ => 0,
                })
                .sum()
        })
        .unwrap_or(0)
}

pub(super) fn cursor_binding_for(query: &JsonValue) -> Result<JsonValue, &'static str> {
    let mut object = query.as_object().cloned().ok_or("query is not an object")?;
    object.remove("cursor");
    Ok(JsonValue::Object(object))
}

pub(super) fn enum_array(
    object: &BTreeMap<String, JsonValue>,
    key: &str,
    values: &[&str],
    minimum: usize,
    maximum: usize,
) -> Result<(), &'static str> {
    let array = object
        .get(key)
        .and_then(JsonValue::as_array)
        .ok_or("capability member must be an array")?;
    if !(minimum..=maximum).contains(&array.len()) || !unique(array) {
        return Err("capability array bounds or uniqueness are invalid");
    }
    if array.iter().any(|value| {
        !value
            .as_string()
            .is_some_and(|value| values.contains(&value))
    }) {
        return Err("capability array contains an unsupported value");
    }
    Ok(())
}
