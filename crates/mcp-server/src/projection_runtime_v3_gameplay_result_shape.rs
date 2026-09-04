// SPDX-License-Identifier: MIT

use super::observation::validate_observation;
use super::{
    MAX_GENERATION, bounded_number, observation_generation, optional_identity,
    project_legal_actions, require_error_code, require_null, safe_identifier,
    validate_identity_value,
};
use crate::json::JsonValue;
use std::collections::BTreeMap;

pub(super) fn validate_kind_shape(
    object: &BTreeMap<String, JsonValue>,
    kind: &str,
    generation: i64,
    request_generation: Option<i64>,
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
        _ => Err("Runtime-v3 response kind is not allowlisted"),
    }
}

pub(super) fn validate_result(
    object: &BTreeMap<String, JsonValue>,
    generation: i64,
    request_generation: Option<i64>,
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
            optional_identity(object.get("state_id"))?;
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
            if request_generation.is_some_and(|expected| generation <= expected) {
                return Err("Runtime-v3 settled result is not fresh");
            }
            validate_result_observation(object, generation)?;
            require_null(object, "error_code")?;
            validate_transition(
                object.get("transition"),
                generation,
                object.get("state_id"),
                request_generation,
            )?;
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

pub(super) fn validate_result_observation(
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

pub(super) fn validate_legal_actions(value: Option<&JsonValue>) -> Result<(), &'static str> {
    let value = value.ok_or("Runtime-v3 legal_actions is missing")?;
    let _ = project_legal_actions(value)?;
    Ok(())
}

pub(super) fn validate_state_id(object: &BTreeMap<String, JsonValue>) -> Result<(), &'static str> {
    validate_identity_value(object.get("state_id"))
}

pub(super) fn validate_transition(
    value: Option<&JsonValue>,
    generation: i64,
    state_id: Option<&JsonValue>,
    request_generation: Option<i64>,
) -> Result<(), &'static str> {
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
    if to != generation
        || to <= from
        || object.get("state_id") != state_id
        || request_generation.is_some_and(|expected| from != expected)
    {
        return Err("Runtime-v3 transition witness is not fresh");
    }
    validate_identity_value(object.get("state_id"))?;
    validate_identity_value(object.get("effect_kind"))
}
