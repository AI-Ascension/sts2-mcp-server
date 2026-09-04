// SPDX-License-Identifier: MIT

use super::state::validate_state;
use super::{MAX_GENERATION, bounded_number, safe_text, validate_identity_value};
use crate::json::JsonValue;
use std::collections::BTreeMap;

pub(super) fn validate_observation(
    value: Option<&JsonValue>,
) -> Result<&BTreeMap<String, JsonValue>, &'static str> {
    let Some(object) = value.and_then(JsonValue::as_object) else {
        return Err("Runtime-v3 observation must be an object");
    };
    const FIELDS: [&str; 5] = ["state_id", "generation", "visible_seed", "player", "state"];
    if object.len() != FIELDS.len() || FIELDS.iter().any(|field| !object.contains_key(*field)) {
        return Err("Runtime-v3 observation contains unknown or missing fields");
    }
    validate_identity_value(object.get("state_id"))?;
    bounded_number(object.get("generation"), MAX_GENERATION)?;
    if let Some(value) = object.get("visible_seed")
        && !matches!(value, JsonValue::Null)
        && !value.as_string().is_some_and(safe_text)
    {
        return Err("Runtime-v3 visible_seed is invalid");
    }
    validate_player(object.get("player"))?;
    validate_state(object.get("state"))?;
    Ok(object)
}

pub(super) fn validate_player(value: Option<&JsonValue>) -> Result<(), &'static str> {
    let Some(object) = value.and_then(JsonValue::as_object) else {
        return Err("Runtime-v3 player must be an object");
    };
    const FIELDS: [&str; 8] = [
        "hp", "max_hp", "energy", "gold", "hand", "deck", "discard", "exhaust",
    ];
    if object.len() != FIELDS.len() || FIELDS.iter().any(|field| !object.contains_key(*field)) {
        return Err("Runtime-v3 player contains unknown or missing fields");
    }
    let hp = bounded_number(object.get("hp"), 65_535)?;
    let max_hp = bounded_number(object.get("max_hp"), 65_535)?;
    if hp > max_hp {
        return Err("Runtime-v3 player hp exceeds max_hp");
    }
    bounded_number(object.get("energy"), 255)?;
    bounded_number(object.get("gold"), 4_294_967_295_i64)?;
    for field in ["hand", "deck", "discard", "exhaust"] {
        validate_cards(object.get(field))?;
    }
    Ok(())
}

pub(super) fn validate_cards(value: Option<&JsonValue>) -> Result<(), &'static str> {
    let Some(values) = value.and_then(|value| match value {
        JsonValue::Array(values) => Some(values),
        _ => None,
    }) else {
        return Err("Runtime-v3 card collection must be an array");
    };
    if values.len() > 256 {
        return Err("Runtime-v3 card collection exceeds the bound");
    }
    for value in values {
        let Some(card) = value.as_object() else {
            return Err("Runtime-v3 card must be an object");
        };
        if card.len() != 4
            || ["card_id", "name", "cost", "upgraded"]
                .iter()
                .any(|field| !card.contains_key(*field))
        {
            return Err("Runtime-v3 card contains unknown or missing fields");
        }
        validate_identity_value(card.get("card_id"))?;
        if !card
            .get("name")
            .and_then(JsonValue::as_string)
            .is_some_and(safe_text)
        {
            return Err("Runtime-v3 card name is invalid");
        }
        bounded_number(card.get("cost"), 255)?;
        if !matches!(card.get("upgraded"), Some(JsonValue::Bool(_))) {
            return Err("Runtime-v3 card upgraded must be a boolean");
        }
    }
    Ok(())
}
