// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::json::JsonValue;
use crate::protocol_artifact_runtime_v3_gameplay::{
    RUNTIME_V3_GAMEPLAY_ARTIFACT, RUNTIME_V3_GAMEPLAY_GENERATOR,
    RUNTIME_V3_GAMEPLAY_MAX_GENERATION, RUNTIME_V3_GAMEPLAY_PROTOCOL_VERSION,
    RUNTIME_V3_GAMEPLAY_SCHEMA_DIGEST, RUNTIME_V3_GAMEPLAY_SCHEMA_SOURCE,
};

#[path = "projection_runtime_v3_gameplay_shape.rs"]
mod shape;

use shape::{
    observation_generation, require_identity, safe_identity, validate_action, validate_observation,
    validate_witness,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RuntimeV3GameplayContext {
    pub(crate) correlation_id: String,
    pub(crate) instance_id: String,
    pub(crate) session_id: String,
    pub(crate) mcp_session_id: String,
    pub(crate) lease_id: String,
    pub(crate) lease_epoch: i64,
    pub(crate) generation: i64,
    pub(crate) operation_id: String,
    pub(crate) card_index: i64,
    pub(crate) target_id: Option<String>,
}

pub(crate) fn validate_gateway_body(
    body: &JsonValue,
    context: &RuntimeV3GameplayContext,
    expected_kind: &str,
) -> Result<(), &'static str> {
    let object = body
        .as_object()
        .ok_or("runtime-v3 gameplay response must be an object")?;
    let kind = validate_envelope(object)?;
    if kind != expected_kind {
        return Err("runtime-v3 gameplay response kind does not match the request");
    }
    validate_context(object, context, kind)?;
    match kind {
        "state_response" => validate_state(object),
        "action_response" | "reconcile_response" => {
            validate_result(object, context.generation, kind, context)
        }
        _ => Err("runtime-v3 gameplay response kind is not allowlisted"),
    }
}

fn validate_envelope(object: &BTreeMap<String, JsonValue>) -> Result<&str, &'static str> {
    const REQUIRED: [&str; 16] = [
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
        "operation_id",
        "observation",
        "action",
        "status",
        "error_code",
        "effect_witness",
    ];
    if object.len() != REQUIRED.len() || REQUIRED.iter().any(|key| !object.contains_key(*key)) {
        return Err("runtime-v3 gameplay envelope contains unknown or missing fields");
    }
    if object
        .get("protocol_version")
        .and_then(JsonValue::as_string)
        != Some(RUNTIME_V3_GAMEPLAY_PROTOCOL_VERSION)
        || object.get("schema_digest").and_then(JsonValue::as_string)
            != Some(RUNTIME_V3_GAMEPLAY_SCHEMA_DIGEST)
    {
        return Err("runtime-v3 gameplay envelope metadata is unsupported");
    }
    let provenance = object
        .get("provenance")
        .and_then(JsonValue::as_object)
        .ok_or("runtime-v3 gameplay provenance is missing")?;
    if provenance.len() != 3
        || provenance.get("artifact").and_then(JsonValue::as_string)
            != Some(RUNTIME_V3_GAMEPLAY_ARTIFACT)
        || provenance.get("source").and_then(JsonValue::as_string)
            != Some(RUNTIME_V3_GAMEPLAY_SCHEMA_SOURCE)
        || provenance.get("generator").and_then(JsonValue::as_string)
            != Some(RUNTIME_V3_GAMEPLAY_GENERATOR)
    {
        return Err("runtime-v3 gameplay provenance is unsupported");
    }
    for key in ["correlation_id", "instance_id", "session_id", "lease_id"] {
        if !object
            .get(key)
            .and_then(JsonValue::as_string)
            .is_some_and(safe_identity)
        {
            return Err("runtime-v3 gameplay identity is unsafe or missing");
        }
    }
    bounded(object.get("lease_epoch"))?;
    bounded(object.get("generation"))?;
    let kind = object
        .get("kind")
        .and_then(JsonValue::as_string)
        .ok_or("runtime-v3 gameplay kind is missing")?;
    if !matches!(
        kind,
        "state_response" | "action_response" | "reconcile_response"
    ) {
        return Err("runtime-v3 gameplay kind is not allowlisted");
    }
    if matches!(kind, "state_response") {
        if !matches!(object.get("operation_id"), Some(JsonValue::Null)) {
            return Err("runtime-v3 gameplay state response operation_id must be null");
        }
    } else if !object
        .get("operation_id")
        .and_then(JsonValue::as_string)
        .is_some_and(safe_identity)
    {
        return Err("runtime-v3 gameplay operation_id is unsafe or missing");
    }
    Ok(kind)
}

fn validate_context(
    object: &BTreeMap<String, JsonValue>,
    context: &RuntimeV3GameplayContext,
    kind: &str,
) -> Result<(), &'static str> {
    for (key, expected) in [
        ("correlation_id", context.correlation_id.as_str()),
        ("instance_id", context.instance_id.as_str()),
        ("session_id", context.session_id.as_str()),
        ("lease_id", context.lease_id.as_str()),
    ] {
        if object.get(key).and_then(JsonValue::as_string) != Some(expected) {
            return Err("runtime-v3 gameplay response identity does not match the request");
        }
    }
    if bounded(object.get("lease_epoch"))? != context.lease_epoch
        || (kind != "state_response"
            && object.get("operation_id").and_then(JsonValue::as_string)
                != Some(context.operation_id.as_str()))
    {
        return Err("runtime-v3 gameplay response lease or operation fence does not match");
    }
    Ok(())
}

fn validate_state(object: &BTreeMap<String, JsonValue>) -> Result<(), &'static str> {
    require_null(object, "operation_id")?;
    for key in ["action", "status", "error_code", "effect_witness"] {
        require_null(object, key)?;
    }
    let observation = object
        .get("observation")
        .ok_or("runtime-v3 gameplay observation is missing")?;
    validate_observation(observation)?;
    if observation_generation(observation)? != bounded(object.get("generation"))? {
        return Err("runtime-v3 gameplay state generation does not match the envelope");
    }
    Ok(())
}

fn validate_result(
    object: &BTreeMap<String, JsonValue>,
    request_generation: i64,
    kind: &str,
    context: &RuntimeV3GameplayContext,
) -> Result<(), &'static str> {
    let action = object
        .get("action")
        .ok_or("runtime-v3 gameplay action is missing")?;
    // Reconciliation queries an existing operation, not a default card/target.
    // Its authenticated receipt supplies the action; submission must still echo
    // the exact action we sent. The witness below always binds to that action.
    validate_action(action, (kind == "action_response").then_some(context))?;
    let generation = bounded(object.get("generation"))?;
    let observation = object
        .get("observation")
        .ok_or("runtime-v3 gameplay observation is missing")?;
    let error_code = object
        .get("error_code")
        .ok_or("runtime-v3 gameplay error_code is missing")?;
    let witness = object
        .get("effect_witness")
        .ok_or("runtime-v3 gameplay effect_witness is missing")?;
    let status = object
        .get("status")
        .and_then(JsonValue::as_string)
        .ok_or("runtime-v3 gameplay status is missing")?;
    match status {
        "unknown" => {
            require_null_value(observation)?;
            require_error_code(error_code)?;
            require_null_value(witness)
        }
        "accepted" => {
            validate_observation(observation)?;
            if observation_generation(observation)? != generation {
                return Err("runtime-v3 gameplay accepted observation is stale");
            }
            require_null_value(error_code)?;
            require_null_value(witness)
        }
        "settled" => {
            if (kind == "action_response" && generation <= request_generation)
                || (kind == "reconcile_response" && generation < request_generation)
            {
                return Err("runtime-v3 gameplay settled result is not fresh");
            }
            validate_observation(observation)?;
            if observation_generation(observation)? != generation {
                return Err("runtime-v3 gameplay settled observation is stale");
            }
            require_null_value(error_code)?;
            validate_witness(witness, generation, action)
        }
        "rejected" | "cancelled" => {
            validate_observation(observation)?;
            if observation_generation(observation)? != generation {
                return Err("runtime-v3 gameplay rejected observation is stale");
            }
            require_error_code(error_code)?;
            require_null_value(witness)
        }
        _ => Err("runtime-v3 gameplay status is not allowlisted"),
    }
}

fn bounded(value: Option<&JsonValue>) -> Result<i64, &'static str> {
    bounded_max(value, RUNTIME_V3_GAMEPLAY_MAX_GENERATION)
}

fn bounded_max(value: Option<&JsonValue>, maximum: i64) -> Result<i64, &'static str> {
    match value {
        Some(JsonValue::Number(value)) if *value >= 0 && *value <= maximum => Ok(*value),
        _ => Err("runtime-v3 gameplay numeric field is outside its bound"),
    }
}

fn require_error_code(value: &JsonValue) -> Result<(), &'static str> {
    require_identity(Some(value)).map(|_| ())
}

fn require_null(object: &BTreeMap<String, JsonValue>, key: &str) -> Result<(), &'static str> {
    require_null_value(
        object
            .get(key)
            .ok_or("runtime-v3 gameplay field is missing")?,
    )
}

fn require_null_value(value: &JsonValue) -> Result<(), &'static str> {
    if matches!(value, JsonValue::Null) {
        Ok(())
    } else {
        Err("runtime-v3 gameplay field must be null")
    }
}
