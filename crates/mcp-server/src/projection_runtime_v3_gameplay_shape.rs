// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::catalog::MAX_IDENTIFIER_BYTES;
use crate::json::JsonValue;

use super::action_shape::{project_legal_action, project_legal_actions};

const MAX_GENERATION: i64 = 9_007_199_254_740_991;
const ROOT_FIELDS: [&str; 21] = [
    "protocol_version",
    "schema_digest",
    "provenance",
    "correlation_id",
    "instance_id",
    "session_id",
    "lease_id",
    "lease_epoch",
    "generation",
    "kind",
    "state_id",
    "operation_id",
    "observation",
    "legal_actions",
    "action",
    "status",
    "transition",
    "error_code",
    "wait_for_millis",
    "wait_outcome",
    "recovery",
];

pub(super) fn validate_and_project(
    body: &JsonValue,
    expected_kind: &str,
    context: &super::RuntimeV3GameplayProjectionContext,
) -> Result<JsonValue, &'static str> {
    let object = exact_root(body)?;
    validate_metadata(object)?;
    validate_identity_field(object, "correlation_id")?;
    validate_identity_field(object, "instance_id")?;
    validate_identity_field(object, "session_id")?;
    validate_identity_field(object, "lease_id")?;
    if object.get("correlation_id").and_then(JsonValue::as_string)
        != Some(context.correlation_id.as_str())
        || object.get("instance_id").and_then(JsonValue::as_string)
            != Some(context.instance_id.as_str())
        || object.get("session_id").and_then(JsonValue::as_string)
            != Some(context.session_id.as_str())
        || object.get("lease_id").and_then(JsonValue::as_string) != Some(context.lease_id.as_str())
    {
        return Err("Runtime-v3 response identity does not match the request");
    }
    let lease_epoch = bounded_number(object.get("lease_epoch"), MAX_GENERATION)?;
    if lease_epoch != context.lease_epoch {
        return Err("Runtime-v3 response lease epoch does not match the request");
    }
    let generation = bounded_number(object.get("generation"), MAX_GENERATION)?;
    let kind = object
        .get("kind")
        .and_then(JsonValue::as_string)
        .ok_or("Runtime-v3 response kind must be a string")?;
    if kind != expected_kind {
        return Err("Runtime-v3 response kind does not match the requested operation");
    }
    validate_kind_shape(
        object,
        kind,
        generation,
        context.generation,
        context.operation_id.as_deref(),
    )?;
    project_root(object)
}

fn exact_root(body: &JsonValue) -> Result<&BTreeMap<String, JsonValue>, &'static str> {
    let Some(object) = body.as_object() else {
        return Err("Runtime-v3 gateway response must be an object");
    };
    if object.len() != ROOT_FIELDS.len()
        || ROOT_FIELDS.iter().any(|field| !object.contains_key(*field))
    {
        return Err("Runtime-v3 gateway response contains unknown or missing fields");
    }
    Ok(object)
}

fn validate_metadata(object: &BTreeMap<String, JsonValue>) -> Result<(), &'static str> {
    if object
        .get("protocol_version")
        .and_then(JsonValue::as_string)
        != Some("runtime-v3-gameplay")
        || object.get("schema_digest").and_then(JsonValue::as_string)
            != Some("fbfb18279b0c7ebb350ef0ce0d56547fa11e83985b13380cb2b0f1dba4cb56e9")
    {
        return Err("Runtime-v3 metadata is unsupported");
    }
    let Some(provenance) = object.get("provenance").and_then(JsonValue::as_object) else {
        return Err("Runtime-v3 provenance must be an object");
    };
    if provenance.len() != 3
        || provenance.get("artifact").and_then(JsonValue::as_string)
            != Some("sts2-protocol/runtime-v3-gameplay")
        || provenance.get("source").and_then(JsonValue::as_string)
            != Some("schemas/runtime-v3-gameplay.schema.json")
        || provenance.get("generator").and_then(JsonValue::as_string) != Some("hand-authored")
    {
        return Err("Runtime-v3 provenance is unsupported");
    }
    Ok(())
}

fn validate_kind_shape(
    object: &BTreeMap<String, JsonValue>,
    kind: &str,
    generation: i64,
    request_generation: i64,
    expected_operation_id: Option<&str>,
) -> Result<(), &'static str> {
    match kind {
        "state_response" | "reobserve_response" => {
            validate_state_id(object)?;
            let observation = validate_observation(object.get("observation"))?;
            if observation_generation(observation)? != generation {
                return Err("Runtime-v3 observation generation does not match the envelope");
            }
            if observation.get("state_id") != object.get("state_id") {
                return Err("Runtime-v3 observation state_id does not match the envelope");
            }
            validate_legal_actions(object.get("legal_actions"))?;
            require_null(object, "operation_id")?;
            require_null(object, "action")?;
            require_null(object, "status")?;
            require_null(object, "transition")?;
            require_null(object, "error_code")?;
            require_null(object, "wait_for_millis")?;
            require_null(object, "wait_outcome")?;
            require_null(object, "recovery")
        }
        "legal_actions_response" => {
            validate_state_id(object)?;
            validate_legal_actions(object.get("legal_actions"))?;
            require_null(object, "operation_id")?;
            require_null(object, "observation")?;
            require_null(object, "action")?;
            require_null(object, "status")?;
            require_null(object, "transition")?;
            require_null(object, "error_code")?;
            require_null(object, "wait_for_millis")?;
            require_null(object, "wait_outcome")?;
            require_null(object, "recovery")
        }
        "dispatch_action_response" | "recover_response" => {
            validate_result(
                object,
                generation,
                request_generation,
                expected_operation_id,
            )?;
            require_null(object, "wait_for_millis")?;
            require_null(object, "wait_outcome")?;
            require_null(object, "recovery")
        }
        "wait_response" => {
            validate_result(
                object,
                generation,
                request_generation,
                expected_operation_id,
            )?;
            let Some(outcome) = object.get("wait_outcome").and_then(JsonValue::as_string) else {
                return Err("Runtime-v3 wait outcome is missing");
            };
            if !matches!(
                outcome,
                "successor" | "same_state_mutation" | "timeout" | "recovery_required"
            ) {
                return Err("Runtime-v3 wait outcome is not allowlisted");
            }
            match outcome {
                "successor" | "same_state_mutation"
                    if object.get("status").and_then(JsonValue::as_string) == Some("settled") => {}
                "timeout" | "recovery_required"
                    if object.get("status").and_then(JsonValue::as_string) == Some("unknown") => {}
                _ => return Err("Runtime-v3 wait outcome does not match status"),
            }
            require_null(object, "wait_for_millis")?;
            require_null(object, "recovery")
        }
        _ => return Err("Runtime-v3 response kind is not allowlisted"),
    }
}

fn validate_result(
    object: &BTreeMap<String, JsonValue>,
    generation: i64,
    request_generation: i64,
    expected_operation_id: Option<&str>,
) -> Result<(), &'static str> {
    let Some(operation_id) = object.get("operation_id").and_then(JsonValue::as_string) else {
        return Err("Runtime-v3 result operation_id is missing");
    };
    if !safe_identifier(operation_id) {
        return Err("Runtime-v3 result operation_id is unsafe or oversized");
    }
    if expected_operation_id.is_some_and(|expected| expected != operation_id) {
        return Err("Runtime-v3 result operation_id does not match the request");
    }
    require_null(object, "action")?;
    let Some(status) = object.get("status").and_then(JsonValue::as_string) else {
        return Err("Runtime-v3 result status must be a string");
    };
    if !matches!(
        status,
        "accepted" | "settled" | "rejected" | "unknown" | "cancelled"
    ) {
        return Err("Runtime-v3 result status is not allowlisted");
    }
    match status {
        "unknown" => {
            require_null(object, "observation")?;
            require_null(object, "legal_actions")?;
            require_error_code(object)?;
            require_null(object, "transition")
        }
        "accepted" => {
            validate_result_observation(object, generation)?;
            require_null(object, "error_code")?;
            require_null(object, "transition")
        }
        "settled" => {
            if generation <= request_generation {
                return Err("Runtime-v3 settled result is not fresh");
            }
            validate_result_observation(object, generation)?;
            require_null(object, "error_code")?;
            validate_transition(object.get("transition"), generation)?;
            Ok(())
        }
        "rejected" | "cancelled" => {
            validate_result_observation(object, generation)?;
            require_error_code(object)?;
            require_null(object, "transition")
        }
        _ => Err("Runtime-v3 result status is not allowlisted"),
    }
}

fn validate_result_observation(
    object: &BTreeMap<String, JsonValue>,
    generation: i64,
) -> Result<(), &'static str> {
    let observation = validate_observation(object.get("observation"))?;
    if observation_generation(observation)? != generation {
        return Err("Runtime-v3 result observation is not fresh");
    }
    validate_state_id(object)?;
    if observation.get("state_id") != object.get("state_id") {
        return Err("Runtime-v3 result observation state_id does not match the envelope");
    }
    validate_legal_actions(object.get("legal_actions"))
}

fn validate_observation(
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
    if let Some(value) = object.get("visible_seed") {
        if !matches!(value, JsonValue::Null) && !value.as_string().is_some_and(safe_text) {
            return Err("Runtime-v3 visible_seed is invalid");
        }
    }
    validate_player(object.get("player"))?;
    validate_state(object.get("state"))?;
    Ok(object)
}

fn validate_player(value: Option<&JsonValue>) -> Result<(), &'static str> {
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

fn validate_cards(value: Option<&JsonValue>) -> Result<(), &'static str> {
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

fn validate_state(value: Option<&JsonValue>) -> Result<(), &'static str> {
    let Some(object) = value.and_then(JsonValue::as_object) else {
        return Err("Runtime-v3 state must be an object");
    };
    let Some(state) = object.get("state").and_then(JsonValue::as_string) else {
        return Err("Runtime-v3 state discriminator must be a string");
    };
    match state {
        "setup" => id_list_shape(object, &["state", "characters"]),
        "map" => {
            if object.len() != 3
                || !object.contains_key("node_id")
                || !object.contains_key("options")
            {
                return Err("Runtime-v3 map state contains unknown or missing fields");
            }
            optional_identity(object.get("node_id"))?;
            validate_id_list(object.get("options"))
        }
        "combat" => {
            if object.len() != 3
                || !object.contains_key("turn_index")
                || !object.contains_key("enemies")
            {
                return Err("Runtime-v3 combat state contains unknown or missing fields");
            }
            bounded_number(object.get("turn_index"), 65_535)?;
            validate_enemies(object.get("enemies"))
        }
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

fn id_list_shape(
    object: &BTreeMap<String, JsonValue>,
    fields: &[&str],
) -> Result<(), &'static str> {
    if object.len() != fields.len() || fields.iter().any(|field| !object.contains_key(*field)) {
        return Err("Runtime-v3 state contains unknown or missing fields");
    }
    validate_id_list(object.get(fields[1]))
}

fn validate_id_list(value: Option<&JsonValue>) -> Result<(), &'static str> {
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

fn validate_enemies(value: Option<&JsonValue>) -> Result<(), &'static str> {
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

fn validate_intent(value: Option<&JsonValue>) -> Result<(), &'static str> {
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

fn validate_shop_items(value: Option<&JsonValue>) -> Result<(), &'static str> {
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

fn validate_legal_actions(value: Option<&JsonValue>) -> Result<(), &'static str> {
    let value = value.ok_or("Runtime-v3 legal_actions is missing")?;
    let _ = project_legal_actions(value)?;
    Ok(())
}

fn validate_state_id(object: &BTreeMap<String, JsonValue>) -> Result<(), &'static str> {
    validate_identity_value(object.get("state_id"))
}

fn validate_transition(value: Option<&JsonValue>, generation: i64) -> Result<(), &'static str> {
    let Some(object) = value.and_then(JsonValue::as_object) else {
        return Err("Runtime-v3 transition witness must be an object");
    };
    if object.len() != 4
        || [
            "from_generation",
            "to_generation",
            "state_id",
            "effect_kind",
        ]
        .iter()
        .any(|field| !object.contains_key(*field))
    {
        return Err("Runtime-v3 transition witness contains unknown or missing fields");
    }
    let from = bounded_number(object.get("from_generation"), MAX_GENERATION)?;
    let to = bounded_number(object.get("to_generation"), MAX_GENERATION)?;
    if to != generation || to <= from {
        return Err("Runtime-v3 transition witness is not fresh");
    }
    validate_identity_value(object.get("state_id"))?;
    validate_identity_value(object.get("effect_kind"))
}

fn require_error_code(object: &BTreeMap<String, JsonValue>) -> Result<(), &'static str> {
    let Some(value) = object.get("error_code").and_then(JsonValue::as_string) else {
        return Err("Runtime-v3 result error_code must be a string");
    };
    if safe_identifier(value) {
        Ok(())
    } else {
        Err("Runtime-v3 result error_code is unsafe or oversized")
    }
}

fn require_null(object: &BTreeMap<String, JsonValue>, field: &str) -> Result<(), &'static str> {
    if matches!(object.get(field), Some(JsonValue::Null)) {
        Ok(())
    } else {
        Err("Runtime-v3 response field has an invalid shape")
    }
}

fn validate_identity_field(
    object: &BTreeMap<String, JsonValue>,
    field: &str,
) -> Result<(), &'static str> {
    validate_identity_value(object.get(field))
}

fn validate_identity_value(value: Option<&JsonValue>) -> Result<(), &'static str> {
    let Some(value) = value.and_then(JsonValue::as_string) else {
        return Err("Runtime-v3 identity must be a string");
    };
    if safe_identifier(value) {
        Ok(())
    } else {
        Err("Runtime-v3 identity is unsafe or oversized")
    }
}

fn optional_identity(value: Option<&JsonValue>) -> Result<(), &'static str> {
    match value {
        Some(JsonValue::Null) => Ok(()),
        Some(JsonValue::String(_)) => validate_identity_value(value),
        _ => Err("Runtime-v3 optional identity is invalid"),
    }
}

fn observation_generation(object: &BTreeMap<String, JsonValue>) -> Result<i64, &'static str> {
    bounded_number(object.get("generation"), MAX_GENERATION)
}

fn bounded_number(value: Option<&JsonValue>, maximum: i64) -> Result<i64, &'static str> {
    match value {
        Some(JsonValue::Number(value)) if *value >= 0 && *value <= maximum => Ok(*value),
        _ => Err("Runtime-v3 numeric field is outside the bound"),
    }
}

fn safe_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_IDENTIFIER_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._:/-".contains(&byte))
}

fn safe_text(value: &str) -> bool {
    !value.is_empty() && value.len() <= 512 && !value.chars().any(char::is_control)
}

fn project_root(object: &BTreeMap<String, JsonValue>) -> Result<JsonValue, &'static str> {
    let mut entries = Vec::with_capacity(ROOT_FIELDS.len());
    for field in ROOT_FIELDS {
        let value = match field {
            "legal_actions" if matches!(object.get(field), Some(JsonValue::Null)) => {
                JsonValue::Null
            }
            "legal_actions" => project_legal_actions(
                object
                    .get(field)
                    .ok_or("Runtime-v3 legal_actions is missing")?,
            )?,
            "action" if !matches!(object.get(field), Some(JsonValue::Null)) => {
                project_legal_action(object.get(field).ok_or("Runtime-v3 action is missing")?)?
            }
            _ => object
                .get(field)
                .cloned()
                .ok_or("Runtime-v3 response field is missing")?,
        };
        entries.push((String::from(field), value));
    }
    Ok(JsonValue::object(entries))
}
