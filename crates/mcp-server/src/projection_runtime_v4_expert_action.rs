// SPDX-License-Identifier: MIT

use super::*;

pub(super) fn project(body: &JsonValue) -> Result<JsonValue, &'static str> {
    let root = exact_object(
        Some(body),
        &[
            "protocol_version",
            "schema_digest",
            "provenance",
            "profile",
            "correlation_id",
            "instance_id",
            "session_id",
            "lease_id",
            "lease_epoch",
            "generation",
            "state_id",
            "operation_id",
            "kind",
            "action",
            "status",
            "observation",
            "transition",
            "error_code",
        ],
        "Runtime-v4 expert action",
    )?;
    if root.get("protocol_version").and_then(JsonValue::as_string)
        != Some(RUNTIME_V4_EXPERT_ACTION_PROTOCOL_VERSION)
        || root.get("schema_digest").and_then(JsonValue::as_string)
            != Some(RUNTIME_V4_EXPERT_ACTION_SCHEMA_DIGEST)
        || root.get("profile").and_then(JsonValue::as_string) != Some("expert-action")
        || root.get("kind").and_then(JsonValue::as_string) != Some("action_response")
    {
        return Err("Runtime-v4 expert action metadata is unsupported");
    }
    let provenance = exact_object(
        root.get("provenance"),
        &["artifact", "source", "generator"],
        "Runtime-v4 action provenance",
    )?;
    if provenance.get("artifact").and_then(JsonValue::as_string)
        != Some(RUNTIME_V4_EXPERT_ACTION_ARTIFACT)
        || provenance.get("source").and_then(JsonValue::as_string)
            != Some(RUNTIME_V4_EXPERT_ACTION_SCHEMA_SOURCE)
        || provenance.get("generator").and_then(JsonValue::as_string)
            != Some(RUNTIME_V4_EXPERT_ACTION_GENERATOR)
    {
        return Err("Runtime-v4 action provenance is unsupported");
    }
    for field in [
        "correlation_id",
        "instance_id",
        "session_id",
        "lease_id",
        "state_id",
        "operation_id",
    ] {
        validate_identity(root.get(field), "Runtime-v4 action identity")?;
    }
    bounded_number(root.get("lease_epoch"), 0, MAX_GENERATION, "lease_epoch")?;
    let generation = bounded_number(root.get("generation"), 0, MAX_GENERATION, "generation")?;
    let status = root
        .get("status")
        .and_then(JsonValue::as_string)
        .ok_or("Runtime-v4 action status is invalid")?;
    if !matches!(
        status,
        "accepted" | "settled" | "rejected" | "unknown" | "cancelled"
    ) {
        return Err("Runtime-v4 action status is invalid");
    }
    match status {
        "accepted" => {
            validate_action_reference(root.get("action"))?;
            require_null(root.get("observation"), "action.observation")?;
            require_null(root.get("transition"), "action.transition")?;
            require_null(root.get("error_code"), "action.error_code")?;
        }
        "settled" => {
            validate_action_reference(root.get("action"))?;
            let observation = root
                .get("observation")
                .filter(|value| !matches!(value, JsonValue::Null))
                .ok_or("Runtime-v4 settled observation is missing")?;
            let observation = super::state::project(observation)?;
            let observed_generation = observation
                .as_object()
                .and_then(|value| value.get("generation"))
                .and_then(|value| match value {
                    JsonValue::Number(value) => Some(*value),
                    _ => None,
                })
                .ok_or("Runtime-v4 settled observation generation is missing")?;
            if observed_generation != generation {
                return Err("Runtime-v4 settled observation generation does not match response");
            }
            let transition = exact_object(
                root.get("transition"),
                &[
                    "kind",
                    "before_generation",
                    "after_generation",
                    "potion_id",
                    "removed",
                ],
                "Runtime-v4 action transition",
            )?;
            if transition.get("kind").and_then(JsonValue::as_string) != Some("potion_use_settled")
                || transition.get("removed") != Some(&JsonValue::Bool(true))
            {
                return Err("Runtime-v4 action transition is invalid");
            }
            let before = bounded_number(
                transition.get("before_generation"),
                0,
                MAX_GENERATION,
                "before_generation",
            )?;
            let after = bounded_number(
                transition.get("after_generation"),
                0,
                MAX_GENERATION,
                "after_generation",
            )?;
            if after <= before || after != generation {
                return Err("Runtime-v4 action transition generation is invalid");
            }
            let potion_id = transition
                .get("potion_id")
                .and_then(JsonValue::as_string)
                .ok_or("Runtime-v4 action transition potion_id is invalid")?;
            let action_potion_id = root
                .get("action")
                .and_then(JsonValue::as_object)
                .and_then(|value| value.get("action"))
                .and_then(JsonValue::as_object)
                .and_then(|value| value.get("potion_id"))
                .and_then(JsonValue::as_string)
                .ok_or("Runtime-v4 settled action potion_id is invalid")?;
            if potion_id != action_potion_id {
                return Err("Runtime-v4 action transition potion_id does not match action");
            }
            require_null(root.get("error_code"), "action.error_code")?;
        }
        "rejected" | "unknown" | "cancelled" => {
            optional_action_reference(root.get("action"))?;
            require_null(root.get("observation"), "action.observation")?;
            require_null(root.get("transition"), "action.transition")?;
            validate_identity(root.get("error_code"), "action.error_code")?;
        }
        _ => unreachable!(),
    }
    Ok(body.clone())
}

fn optional_action_reference(value: Option<&JsonValue>) -> Result<(), &'static str> {
    match value {
        Some(JsonValue::Null) => Ok(()),
        Some(value) => validate_action_reference(Some(value)),
        None => Err("Runtime-v4 action reference is missing"),
    }
}

fn require_null(value: Option<&JsonValue>, _field: &str) -> Result<(), &'static str> {
    matches!(value, Some(JsonValue::Null))
        .then_some(())
        .ok_or("Runtime-v4 action nullable field is not null")
}

fn validate_action_reference(value: Option<&JsonValue>) -> Result<(), &'static str> {
    let action = exact_object(
        value,
        &["action_id", "action"],
        "Runtime-v4 action reference",
    )?;
    validate_identity(action.get("action_id"), "action_id")?;
    let payload = exact_object(
        action.get("action"),
        &["kind", "potion_id", "target_id"],
        "Runtime-v4 potion action",
    )?;
    if payload.get("kind").and_then(JsonValue::as_string) != Some("use_potion") {
        return Err("Runtime-v4 potion action kind is invalid");
    }
    validate_identity(payload.get("potion_id"), "potion_id")?;
    optional_identity(payload.get("target_id"), "target_id")
}
