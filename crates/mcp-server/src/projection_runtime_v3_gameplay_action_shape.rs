// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::json::JsonValue;

const MAX_ACTIONS: usize = 256;

pub(super) fn project_legal_action(value: &JsonValue) -> Result<JsonValue, &'static str> {
    let Some(object) = value.as_object() else {
        return Err("Runtime-v3 legal action must be an object");
    };
    if !exact_keys(object, &["action_id", "action"]) {
        return Err("Runtime-v3 legal action contains unknown or missing fields");
    }
    let Some(action_id) = object.get("action_id").and_then(JsonValue::as_string) else {
        return Err("Runtime-v3 legal action_id must be a string");
    };
    if !safe_identifier(action_id) {
        return Err("Runtime-v3 legal action_id is unsafe or oversized");
    }
    let action = object
        .get("action")
        .ok_or("Runtime-v3 legal action payload is missing")?;
    Ok(JsonValue::object([
        (String::from("action_id"), JsonValue::string(action_id)),
        (String::from("action"), project_action_payload(action)?),
    ]))
}

pub(super) fn project_legal_actions(value: &JsonValue) -> Result<JsonValue, &'static str> {
    let Some(values) = (match value {
        JsonValue::Array(values) => Some(values),
        _ => None,
    }) else {
        return Err("Runtime-v3 legal_actions must be an array");
    };
    if values.len() > MAX_ACTIONS {
        return Err("Runtime-v3 legal_actions exceeds the bound");
    }
    let mut action_ids = Vec::with_capacity(values.len());
    let mut projected = Vec::with_capacity(values.len());
    for value in values {
        let action = project_legal_action(value)?;
        let Some(action_id) = action
            .as_object()
            .and_then(|object| object.get("action_id"))
            .and_then(JsonValue::as_string)
        else {
            return Err("Runtime-v3 legal action_id is missing after projection");
        };
        if action_ids.iter().any(|previous| previous == action_id) {
            return Err("Runtime-v3 legal action IDs must be unique");
        }
        action_ids.push(String::from(action_id));
        projected.push(action);
    }
    Ok(JsonValue::Array(projected))
}

fn project_action_payload(value: &JsonValue) -> Result<JsonValue, &'static str> {
    let Some(object) = value.as_object() else {
        return Err("Runtime-v3 action payload must be an object");
    };
    let Some(kind) = object.get("kind").and_then(JsonValue::as_string) else {
        return Err("Runtime-v3 action payload kind must be a string");
    };
    match kind {
        "end_turn" | "skip_reward" | "rest" | "confirm_victory" | "save_quit" => {
            if !exact_keys(object, &["kind"]) {
                return Err("Runtime-v3 action payload contains unknown fields");
            }
            Ok(JsonValue::object([(
                String::from("kind"),
                JsonValue::string(kind),
            )]))
        }
        "start_run" => one_argument(object, kind, "character_id"),
        "select_map_node" => one_argument(object, kind, "node_id"),
        "choose_reward" => one_argument(object, kind, "reward_id"),
        "shop_purchase" => one_argument(object, kind, "item_id"),
        "shop_remove" | "smith" | "select_card" => one_argument(object, kind, "card_id"),
        "event_choice" => one_argument(object, kind, "choice_id"),
        "play_card" => play_card(object, kind),
        _ => Err("Runtime-v3 action kind is not allowlisted"),
    }
}

fn one_argument(
    object: &BTreeMap<String, JsonValue>,
    kind: &str,
    field: &str,
) -> Result<JsonValue, &'static str> {
    if !exact_keys(object, &["kind", field]) {
        return Err("Runtime-v3 action payload contains unknown or missing fields");
    }
    let Some(value) = object.get(field).and_then(JsonValue::as_string) else {
        return Err("Runtime-v3 action argument must be a string");
    };
    if !safe_identifier(value) {
        return Err("Runtime-v3 action argument is unsafe or oversized");
    }
    Ok(JsonValue::object([
        (String::from("kind"), JsonValue::string(kind)),
        (String::from(field), JsonValue::string(value)),
    ]))
}

fn play_card(object: &BTreeMap<String, JsonValue>, kind: &str) -> Result<JsonValue, &'static str> {
    if !exact_keys(object, &["kind", "card_id", "target_id"]) {
        return Err("Runtime-v3 play_card contains unknown or missing fields");
    }
    let Some(card_id) = object.get("card_id").and_then(JsonValue::as_string) else {
        return Err("Runtime-v3 play_card card_id must be a string");
    };
    if !safe_identifier(card_id) {
        return Err("Runtime-v3 play_card card_id is unsafe or oversized");
    }
    let target_id = match object.get("target_id") {
        Some(JsonValue::Null) => JsonValue::Null,
        Some(JsonValue::String(value)) if safe_identifier(value) => JsonValue::string(value),
        _ => return Err("Runtime-v3 play_card target_id is invalid"),
    };
    Ok(JsonValue::object([
        (String::from("kind"), JsonValue::string(kind)),
        (String::from("card_id"), JsonValue::string(card_id)),
        (String::from("target_id"), target_id),
    ]))
}

fn exact_keys(object: &BTreeMap<String, JsonValue>, keys: &[&str]) -> bool {
    object.len() == keys.len() && keys.iter().all(|key| object.contains_key(*key))
}

fn safe_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 512
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._:/-".contains(&byte))
}
