// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::catalog::MAX_IDENTIFIER_BYTES;
use crate::json::JsonValue;
use crate::protocol_artifact_runtime_v2::{
    RUNTIME_V2_ACTION_ID, RUNTIME_V2_EFFECT_KIND, RUNTIME_V2_MAX_GENERATION,
    RUNTIME_V2_MAX_TURN_INDEX, RUNTIME_V2_PLAYER_TURN_PHASE,
};

pub(super) fn validate_kind_shape(
    object: &BTreeMap<String, JsonValue>,
    kind: &str,
    request_generation: i64,
) -> Result<(), &'static str> {
    let generation = bounded_number(
        object
            .get("generation")
            .ok_or("runtime-v2 gateway generation is missing")?,
        RUNTIME_V2_MAX_GENERATION,
    )?;
    let observation = object
        .get("observation")
        .ok_or("runtime-v2 gateway observation is missing")?;
    let action = object
        .get("action")
        .ok_or("runtime-v2 gateway action is missing")?;
    let status = object
        .get("status")
        .ok_or("runtime-v2 gateway status is missing")?;
    let error_code = object
        .get("error_code")
        .ok_or("runtime-v2 gateway error_code is missing")?;
    let effect_witness = object
        .get("effect_witness")
        .ok_or("runtime-v2 gateway effect_witness is missing")?;

    match kind {
        "state_request" => require_nulls(observation, action, status, error_code, effect_witness),
        "state_response" => {
            require_null(action, "runtime-v2 state response action must be null")?;
            require_null(status, "runtime-v2 state response status must be null")?;
            require_null(
                error_code,
                "runtime-v2 state response error_code must be null",
            )?;
            require_null(
                effect_witness,
                "runtime-v2 state response effect_witness must be null",
            )?;
            let observation = validate_observation(observation)?;
            if observation_generation(observation)? != generation {
                return Err("runtime-v2 observation generation does not match the envelope");
            }
            Ok(())
        }
        "action_request" => {
            require_null(
                observation,
                "runtime-v2 action request observation must be null",
            )?;
            require_null(status, "runtime-v2 action request status must be null")?;
            require_null(
                error_code,
                "runtime-v2 action request error_code must be null",
            )?;
            require_null(
                effect_witness,
                "runtime-v2 action request effect_witness must be null",
            )?;
            validate_action(action)
        }
        "reconcile_request" => {
            require_null(
                observation,
                "runtime-v2 reconcile request observation must be null",
            )?;
            require_null(action, "runtime-v2 reconcile request action must be null")?;
            require_null(status, "runtime-v2 reconcile request status must be null")?;
            require_null(
                error_code,
                "runtime-v2 reconcile request error_code must be null",
            )?;
            require_null(
                effect_witness,
                "runtime-v2 reconcile request effect_witness must be null",
            )
        }
        "action_response" | "reconcile_response" => {
            validate_action(action)?;
            validate_result(
                status,
                observation,
                error_code,
                effect_witness,
                generation,
                request_generation,
                kind,
            )
        }
        _ => Err("runtime-v2 gateway kind is not allowlisted"),
    }
}

fn validate_result(
    status: &JsonValue,
    observation: &JsonValue,
    error_code: &JsonValue,
    effect_witness: &JsonValue,
    generation: i64,
    request_generation: i64,
    kind: &str,
) -> Result<(), &'static str> {
    let Some(status) = status.as_string() else {
        return Err("runtime-v2 result status must be a string");
    };
    if !matches!(
        status,
        "accepted" | "settled" | "rejected" | "unknown" | "cancelled"
    ) {
        return Err("runtime-v2 result status is not allowlisted");
    }
    match status {
        "unknown" => {
            require_null(
                observation,
                "runtime-v2 unknown result observation must be null",
            )?;
            require_error_code(error_code)?;
            require_null(
                effect_witness,
                "runtime-v2 unknown result effect_witness must be null",
            )
        }
        "accepted" => {
            let observation = validate_observation(observation)?;
            if observation_generation(observation)? != generation {
                return Err("runtime-v2 observation generation does not match the envelope");
            }
            require_null(
                error_code,
                "runtime-v2 accepted result error_code must be null",
            )?;
            require_null(
                effect_witness,
                "runtime-v2 accepted result effect_witness must be null",
            )
        }
        "settled" => {
            if (kind == "action_response" && generation <= request_generation)
                || (kind == "reconcile_response" && generation < request_generation)
            {
                return Err(
                    "runtime-v2 settled result does not contain a non-regressing generation",
                );
            }
            let observation = validate_observation(observation)?;
            if observation_generation(observation)? != generation {
                return Err("runtime-v2 settled observation is not fresh");
            }
            require_null(
                error_code,
                "runtime-v2 settled result error_code must be null",
            )?;
            let witness_generation = validate_effect_witness(effect_witness)?;
            if witness_generation != generation {
                return Err("runtime-v2 settlement witness does not match the envelope");
            }
            Ok(())
        }
        "rejected" | "cancelled" => {
            let observation = validate_observation(observation)?;
            if observation_generation(observation)? != generation {
                return Err("runtime-v2 observation generation does not match the envelope");
            }
            require_error_code(error_code)?;
            require_null(
                effect_witness,
                "runtime-v2 rejected or cancelled result effect_witness must be null",
            )
        }
        _ => Err("runtime-v2 result status is not allowlisted"),
    }
}

fn validate_observation(value: &JsonValue) -> Result<&BTreeMap<String, JsonValue>, &'static str> {
    let Some(object) = value.as_object() else {
        return Err("runtime-v2 observation must be an object");
    };
    const REQUIRED: [&str; 4] = ["combat_phase", "turn_index", "host_ready", "generation"];
    if object.len() != REQUIRED.len() || REQUIRED.iter().any(|key| !object.contains_key(*key)) {
        return Err("runtime-v2 observation contains unknown or missing fields");
    }
    let Some(combat_phase) = object.get("combat_phase").and_then(JsonValue::as_string) else {
        return Err("runtime-v2 observation combat_phase must be a string");
    };
    if !matches!(
        combat_phase,
        "outside_combat" | RUNTIME_V2_PLAYER_TURN_PHASE | "combat/enemy_turn"
    ) {
        return Err("runtime-v2 observation combat_phase is not allowlisted");
    }
    bounded_number(
        object
            .get("turn_index")
            .ok_or("runtime-v2 observation turn_index is missing")?,
        RUNTIME_V2_MAX_TURN_INDEX,
    )?;
    if !matches!(object.get("host_ready"), Some(JsonValue::Bool(_))) {
        return Err("runtime-v2 observation host_ready must be a boolean");
    }
    bounded_number(
        object
            .get("generation")
            .ok_or("runtime-v2 observation generation is missing")?,
        RUNTIME_V2_MAX_GENERATION,
    )?;
    Ok(object)
}

fn observation_generation(object: &BTreeMap<String, JsonValue>) -> Result<i64, &'static str> {
    bounded_number(
        object
            .get("generation")
            .ok_or("runtime-v2 observation generation is missing")?,
        RUNTIME_V2_MAX_GENERATION,
    )
}

fn validate_action(value: &JsonValue) -> Result<(), &'static str> {
    let Some(object) = value.as_object() else {
        return Err("runtime-v2 action must be an object");
    };
    if object.len() != 1 {
        return Err("runtime-v2 action contains unknown or missing fields");
    }
    if object.get("action_id").and_then(JsonValue::as_string) != Some(RUNTIME_V2_ACTION_ID) {
        return Err("runtime-v2 action_id is not the fixed end_turn action");
    }
    Ok(())
}

fn validate_effect_witness(value: &JsonValue) -> Result<i64, &'static str> {
    let Some(object) = value.as_object() else {
        return Err("runtime-v2 effect_witness must be an object");
    };
    if object.len() != 2
        || object.get("kind").and_then(JsonValue::as_string) != Some(RUNTIME_V2_EFFECT_KIND)
    {
        return Err("runtime-v2 effect_witness is not the fixed settlement witness");
    }
    bounded_number(
        object
            .get("generation")
            .ok_or("runtime-v2 effect_witness generation is missing")?,
        RUNTIME_V2_MAX_GENERATION,
    )
}

fn require_nulls(
    observation: &JsonValue,
    action: &JsonValue,
    status: &JsonValue,
    error_code: &JsonValue,
    effect_witness: &JsonValue,
) -> Result<(), &'static str> {
    require_null(observation, "runtime-v2 request observation must be null")?;
    require_null(action, "runtime-v2 request action must be null")?;
    require_null(status, "runtime-v2 request status must be null")?;
    require_null(error_code, "runtime-v2 request error_code must be null")?;
    require_null(
        effect_witness,
        "runtime-v2 request effect_witness must be null",
    )
}

fn require_null(value: &JsonValue, message: &'static str) -> Result<(), &'static str> {
    if matches!(value, JsonValue::Null) {
        Ok(())
    } else {
        Err(message)
    }
}

fn require_error_code(value: &JsonValue) -> Result<(), &'static str> {
    let Some(error_code) = value.as_string() else {
        return Err("runtime-v2 result error_code must be a string");
    };
    if safe_identity(error_code) {
        Ok(())
    } else {
        Err("runtime-v2 result error_code is unsafe or oversized")
    }
}

pub(super) fn bounded_number(value: &JsonValue, maximum: i64) -> Result<i64, &'static str> {
    match value {
        JsonValue::Number(value) if *value >= 0 && *value <= maximum => Ok(*value),
        _ => Err("runtime-v2 numeric field is outside the protocol bound"),
    }
}

pub(super) fn safe_identity(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_IDENTIFIER_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._:/-".contains(&byte))
}
