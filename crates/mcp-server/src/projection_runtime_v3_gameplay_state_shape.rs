// SPDX-License-Identifier: MIT

use super::{bounded_number, optional_identity, safe_text, validate_identity_value};
use crate::json::JsonValue;
use std::collections::BTreeMap;

pub(super) fn validate_state(value: Option<&JsonValue>) -> Result<(), &'static str> {
    let Some(object) = value.and_then(JsonValue::as_object) else {
        return Err("Runtime-v3 state must be an object");
    };
    let Some(state) = object.get("state").and_then(JsonValue::as_string) else {
        return Err("Runtime-v3 state discriminator must be a string");
    };
    match state {
        "setup" => id_list_shape(object, &["state", "characters"]),
        "map" => validate_map_state(object),
        "combat" => validate_combat_state(object),
        "reward" | "rest" => id_list_shape(object, &["state", "options"]),
        "event" | "selection" => id_list_shape(object, &["state", "choices"]),
        "shop" => {
            if object.len() != 2 || !object.contains_key("items") {
                return Err("Runtime-v3 shop state contains unknown or missing fields");
            }
            validate_shop_items(object.get("items"))
        }
        "victory" => {
            if object.len() != 1 {
                Err("Runtime-v3 victory state contains unknown fields")
            } else {
                Ok(())
            }
        }
        "defeat" => {
            if object.len() != 2 || !object.contains_key("reason") {
                return Err("Runtime-v3 defeat state contains unknown or missing fields");
            }
            if !matches!(object.get("reason"), Some(JsonValue::Null))
                && !object
                    .get("reason")
                    .and_then(JsonValue::as_string)
                    .is_some_and(safe_text)
            {
                return Err("Runtime-v3 defeat reason is invalid");
            }
            Ok(())
        }
        "recovery" => {
            if object.len() != 2 || !object.contains_key("code") {
                return Err("Runtime-v3 recovery state contains unknown or missing fields");
            }
            validate_identity_value(object.get("code"))
        }
        _ => Err("Runtime-v3 state is not allowlisted"),
    }
}

pub(super) fn id_list_shape(
    object: &BTreeMap<String, JsonValue>,
    fields: &[&str],
) -> Result<(), &'static str> {
    if object.len() != fields.len() || fields.iter().any(|field| !object.contains_key(*field)) {
        return Err("Runtime-v3 state contains unknown or missing fields");
    }
    validate_id_list(object.get(fields[1]))
}

pub(super) fn validate_id_list(value: Option<&JsonValue>) -> Result<(), &'static str> {
    let Some(values) = value.and_then(|value| match value {
        JsonValue::Array(values) => Some(values),
        _ => None,
    }) else {
        return Err("Runtime-v3 state choices must be an array");
    };
    if values.len() > 256 {
        return Err("Runtime-v3 state choices exceed the bound");
    }
    for value in values {
        validate_identity_value(Some(value))?;
    }
    Ok(())
}

pub(super) fn validate_enemies(value: Option<&JsonValue>) -> Result<(), &'static str> {
    let Some(values) = value.and_then(|value| match value {
        JsonValue::Array(values) => Some(values),
        _ => None,
    }) else {
        return Err("Runtime-v3 enemies must be an array");
    };
    if values.len() > 256 {
        return Err("Runtime-v3 enemies exceed the bound");
    }
    for value in values {
        let Some(enemy) = value.as_object() else {
            return Err("Runtime-v3 enemy must be an object");
        };
        if enemy.len() != 5
            || ["enemy_id", "name", "hp", "max_hp", "intent"]
                .iter()
                .any(|field| !enemy.contains_key(*field))
        {
            return Err("Runtime-v3 enemy contains unknown or missing fields");
        }
        validate_identity_value(enemy.get("enemy_id"))?;
        if !enemy
            .get("name")
            .and_then(JsonValue::as_string)
            .is_some_and(safe_text)
        {
            return Err("Runtime-v3 enemy name is invalid");
        }
        let hp = bounded_number(enemy.get("hp"), 65_535)?;
        let max_hp = bounded_number(enemy.get("max_hp"), 65_535)?;
        if hp > max_hp {
            return Err("Runtime-v3 enemy hp exceeds max_hp");
        }
        validate_intent(enemy.get("intent"))?;
    }
    Ok(())
}

pub(super) fn validate_intent(value: Option<&JsonValue>) -> Result<(), &'static str> {
    let Some(object) = value.and_then(JsonValue::as_object) else {
        return Err("Runtime-v3 enemy intent must be an object");
    };
    let Some(kind) = object.get("kind").and_then(JsonValue::as_string) else {
        return Err("Runtime-v3 enemy intent kind must be a string");
    };
    match kind {
        "attack" => {
            if object.len() != 3 || !object.contains_key("damage") || !object.contains_key("hits") {
                return Err("Runtime-v3 attack intent contains unknown or missing fields");
            }
            bounded_number(object.get("damage"), 65_535)?;
            let hits = bounded_number(object.get("hits"), 255)?;
            if hits == 0 {
                Err("Runtime-v3 attack intent hits must be positive")
            } else {
                Ok(())
            }
        }
        "defend" | "buff" | "debuff" | "unknown" => {
            if object.len() != 1 {
                Err("Runtime-v3 enemy intent contains unknown fields")
            } else {
                Ok(())
            }
        }
        _ => Err("Runtime-v3 enemy intent is not allowlisted"),
    }
}

pub(super) fn validate_shop_items(value: Option<&JsonValue>) -> Result<(), &'static str> {
    let Some(values) = value.and_then(|value| match value {
        JsonValue::Array(values) => Some(values),
        _ => None,
    }) else {
        return Err("Runtime-v3 shop items must be an array");
    };
    if values.len() > 256 {
        return Err("Runtime-v3 shop items exceed the bound");
    }
    for value in values {
        let Some(item) = value.as_object() else {
            return Err("Runtime-v3 shop item must be an object");
        };
        if item.len() != 3
            || ["item_id", "name", "price"]
                .iter()
                .any(|field| !item.contains_key(*field))
        {
            return Err("Runtime-v3 shop item contains unknown or missing fields");
        }
        validate_identity_value(item.get("item_id"))?;
        if !item
            .get("name")
            .and_then(JsonValue::as_string)
            .is_some_and(safe_text)
        {
            return Err("Runtime-v3 shop item name is invalid");
        }
        bounded_number(item.get("price"), 4_294_967_295_i64)?;
    }
    Ok(())
}

fn validate_map_state(object: &BTreeMap<String, JsonValue>) -> Result<(), &'static str> {
    if object.len() != 3 || !object.contains_key("node_id") || !object.contains_key("options") {
        return Err("Runtime-v3 map state contains unknown or missing fields");
    }
    optional_identity(object.get("node_id"))?;
    validate_id_list(object.get("options"))
}

fn validate_combat_state(object: &BTreeMap<String, JsonValue>) -> Result<(), &'static str> {
    if object.len() != 3 || !object.contains_key("turn_index") || !object.contains_key("enemies") {
        return Err("Runtime-v3 combat state contains unknown or missing fields");
    }
    bounded_number(object.get("turn_index"), 65_535)?;
    validate_enemies(object.get("enemies"))
}
