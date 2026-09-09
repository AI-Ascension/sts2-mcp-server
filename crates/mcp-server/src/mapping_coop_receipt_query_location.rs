// SPDX-License-Identifier: MIT

use crate::json::JsonValue;
use std::collections::BTreeSet;

use super::safe_header;

pub(super) fn location(value: Option<&JsonValue>) -> Result<JsonValue, &'static str> {
    let object = value
        .and_then(JsonValue::as_object)
        .ok_or("location must be an object")?;
    if object.len() != 3
        || !["act_index", "room_id", "coord"]
            .into_iter()
            .all(|field| object.contains_key(field))
    {
        return Err("location has unknown or missing fields");
    }
    signed_i32(object.get("act_index"))?;
    optional_signed_i32(object.get("room_id"))?;
    match object.get("coord") {
        Some(JsonValue::Null) => {}
        Some(JsonValue::Object(coord))
            if coord.len() == 2 && coord.contains_key("col") && coord.contains_key("row") =>
        {
            signed_i32(coord.get("col"))?;
            signed_i32(coord.get("row"))?;
        }
        _ => return Err("location coord must be null or a col/row object"),
    }
    Ok(value.cloned().unwrap_or(JsonValue::Null))
}

fn signed_i32(value: Option<&JsonValue>) -> Result<i64, &'static str> {
    match value {
        Some(JsonValue::Number(value))
            if (i64::from(i32::MIN)..=i64::from(i32::MAX)).contains(value) =>
        {
            Ok(*value)
        }
        _ => Err("location coordinate must be a signed 32-bit integer"),
    }
}

fn optional_signed_i32(value: Option<&JsonValue>) -> Result<(), &'static str> {
    match value {
        Some(JsonValue::Null) => Ok(()),
        _ => signed_i32(value).map(|_| ()),
    }
}

pub(super) fn participants(
    value: Option<&JsonValue>,
    actor: &str,
) -> Result<Vec<String>, &'static str> {
    let values = value
        .and_then(JsonValue::as_array)
        .ok_or("participant_ids must be an array")?;
    if !(2..=4).contains(&values.len()) {
        return Err("participant_ids must contain two to four peers");
    }
    let mut unique = BTreeSet::new();
    let mut result = Vec::with_capacity(values.len());
    for value in values {
        let id = value
            .as_string()
            .filter(|value| safe_header(value))
            .ok_or("participant_ids contains an invalid identity")?;
        if !unique.insert(id.to_owned()) {
            return Err("participant_ids must contain unique peers");
        }
        result.push(id.to_owned());
    }
    if result.windows(2).any(|window| window[0] >= window[1]) {
        return Err("participant_ids must be sorted ascending");
    }
    if !unique.contains(actor) {
        return Err("actor_id must be a participant");
    }
    Ok(result)
}
