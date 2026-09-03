// SPDX-License-Identifier: MIT

use std::collections::BTreeSet;

use crate::json::JsonValue;
use crate::protocol_artifact_runtime_v3_gameplay::{
    RUNTIME_V3_GAMEPLAY_ACTION_ID, RUNTIME_V3_GAMEPLAY_EFFECT_KIND,
    RUNTIME_V3_GAMEPLAY_MAX_CARD_INDEX, RUNTIME_V3_GAMEPLAY_MAX_ENEMIES,
    RUNTIME_V3_GAMEPLAY_MAX_ENERGY, RUNTIME_V3_GAMEPLAY_MAX_GENERATION,
    RUNTIME_V3_GAMEPLAY_MAX_PILE_COUNT, RUNTIME_V3_GAMEPLAY_MAX_TURN_INDEX,
};

use super::{RuntimeV3GameplayContext, bounded_max};

pub(super) fn validate_observation(value: &JsonValue) -> Result<(), &'static str> {
    let object = value
        .as_object()
        .ok_or("runtime-v3 gameplay observation must be an object")?;
    const REQUIRED: [&str; 10] = [
        "combat_phase",
        "turn_index",
        "host_ready",
        "generation",
        "hand_count",
        "energy",
        "draw_pile_count",
        "discard_pile_count",
        "exhaust_pile_count",
        "enemies",
    ];
    if object.len() != REQUIRED.len() || REQUIRED.iter().any(|key| !object.contains_key(*key)) {
        return Err("runtime-v3 gameplay observation contains unknown or missing fields");
    }
    if !matches!(
        object.get("combat_phase").and_then(JsonValue::as_string),
        Some("outside_combat" | "combat/player_turn" | "combat/enemy_turn")
    ) || !matches!(object.get("host_ready"), Some(JsonValue::Bool(_)))
    {
        return Err("runtime-v3 gameplay observation has an invalid phase or readiness value");
    }
    for (key, maximum) in [
        ("turn_index", RUNTIME_V3_GAMEPLAY_MAX_TURN_INDEX),
        ("hand_count", RUNTIME_V3_GAMEPLAY_MAX_CARD_INDEX),
        ("energy", RUNTIME_V3_GAMEPLAY_MAX_ENERGY),
        ("draw_pile_count", RUNTIME_V3_GAMEPLAY_MAX_PILE_COUNT),
        ("discard_pile_count", RUNTIME_V3_GAMEPLAY_MAX_PILE_COUNT),
        ("exhaust_pile_count", RUNTIME_V3_GAMEPLAY_MAX_PILE_COUNT),
        ("generation", RUNTIME_V3_GAMEPLAY_MAX_GENERATION),
    ] {
        bounded_max(object.get(key), maximum)?;
    }
    let enemies = match object.get("enemies") {
        Some(JsonValue::Array(enemies)) => enemies,
        _ => return Err("runtime-v3 gameplay enemies must be an array"),
    };
    if enemies.len() > RUNTIME_V3_GAMEPLAY_MAX_ENEMIES {
        return Err("runtime-v3 gameplay enemy list exceeds its bound");
    }
    let mut ids = BTreeSet::new();
    for enemy in enemies {
        let enemy = enemy
            .as_object()
            .ok_or("runtime-v3 gameplay enemy must be an object")?;
        if enemy.len() != 3
            || !ENEMY_FIELDS.iter().all(|key| enemy.contains_key(*key))
            || !matches!(enemy.get("alive"), Some(JsonValue::Bool(_)))
            || !matches!(enemy.get("hittable"), Some(JsonValue::Bool(_)))
        {
            return Err("runtime-v3 gameplay enemy shape is invalid");
        }
        let target_id = require_identity(enemy.get("target_id"))?;
        if !ids.insert(target_id) {
            return Err("runtime-v3 gameplay enemy target is duplicated");
        }
    }
    Ok(())
}

pub(super) fn validate_action(
    value: &JsonValue,
    context: &RuntimeV3GameplayContext,
) -> Result<(), &'static str> {
    let object = value
        .as_object()
        .ok_or("runtime-v3 gameplay action must be an object")?;
    if object.len() != 3
        || !ACTION_FIELDS.iter().all(|key| object.contains_key(*key))
        || object.get("action_id").and_then(JsonValue::as_string)
            != Some(RUNTIME_V3_GAMEPLAY_ACTION_ID)
        || bounded_max(object.get("card_index"), RUNTIME_V3_GAMEPLAY_MAX_CARD_INDEX)?
            != context.card_index
        || optional_identity(object.get("target_id"))? != context.target_id
    {
        return Err("runtime-v3 gameplay action does not match the request");
    }
    Ok(())
}

pub(super) fn validate_witness(
    value: &JsonValue,
    generation: i64,
    context: &RuntimeV3GameplayContext,
) -> Result<(), &'static str> {
    let object = value
        .as_object()
        .ok_or("runtime-v3 gameplay witness must be an object")?;
    if object.len() != 4
        || !WITNESS_FIELDS.iter().all(|key| object.contains_key(*key))
        || object.get("kind").and_then(JsonValue::as_string)
            != Some(RUNTIME_V3_GAMEPLAY_EFFECT_KIND)
        || bounded_max(object.get("generation"), RUNTIME_V3_GAMEPLAY_MAX_GENERATION)? != generation
        || bounded_max(object.get("card_index"), RUNTIME_V3_GAMEPLAY_MAX_CARD_INDEX)?
            != context.card_index
        || optional_identity(object.get("target_id"))? != context.target_id
    {
        return Err("runtime-v3 gameplay settlement witness does not match the request");
    }
    Ok(())
}

pub(super) fn observation_generation(value: &JsonValue) -> Result<i64, &'static str> {
    let object = value
        .as_object()
        .ok_or("runtime-v3 gameplay observation must be an object")?;
    bounded_max(object.get("generation"), RUNTIME_V3_GAMEPLAY_MAX_GENERATION)
}

pub(super) fn require_identity(value: Option<&JsonValue>) -> Result<&str, &'static str> {
    let value = value
        .and_then(JsonValue::as_string)
        .ok_or("runtime-v3 gameplay identity must be a string")?;
    if safe_identity(value) {
        Ok(value)
    } else {
        Err("runtime-v3 gameplay identity is unsafe or oversized")
    }
}

pub(super) fn optional_identity(value: Option<&JsonValue>) -> Result<Option<String>, &'static str> {
    match value {
        Some(JsonValue::Null) => Ok(None),
        Some(value) => require_identity(Some(value)).map(|value| Some(value.to_owned())),
        None => Err("runtime-v3 gameplay identity is missing"),
    }
}

pub(super) fn safe_identity(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._:/-".contains(&byte))
}

const ENEMY_FIELDS: [&str; 3] = ["target_id", "alive", "hittable"];
const ACTION_FIELDS: [&str; 3] = ["action_id", "card_index", "target_id"];
const WITNESS_FIELDS: [&str; 4] = ["kind", "generation", "card_index", "target_id"];
