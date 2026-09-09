// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::json::JsonValue;
use crate::protocol_artifact_runtime_v4_expert_rest_action::{
    RUNTIME_V4_EXPERT_REST_ACTION_ARTIFACT, RUNTIME_V4_EXPERT_REST_ACTION_GENERATOR,
    RUNTIME_V4_EXPERT_REST_ACTION_MAX_GENERATION, RUNTIME_V4_EXPERT_REST_ACTION_PROTOCOL_VERSION,
    RUNTIME_V4_EXPERT_REST_ACTION_SCHEMA_DIGEST, RUNTIME_V4_EXPERT_REST_ACTION_SCHEMA_SOURCE,
};

#[path = "projection_runtime_v4_expert_rest_action_shape.rs"]
mod shape;

const ROOT_FIELDS: [&str; 19] = [
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
    "effect_witness",
    "error_code",
];

pub(crate) fn project_runtime_v4_expert_rest_action_gateway_body(
    body: &JsonValue,
) -> Result<JsonValue, &'static str> {
    let root = exact_object(Some(body), &ROOT_FIELDS)?;
    validate_metadata(root)?;
    for field in [
        "correlation_id",
        "instance_id",
        "session_id",
        "lease_id",
        "state_id",
        "operation_id",
    ] {
        validate_identity(root.get(field))?;
    }
    bounded_number(root.get("lease_epoch"))?;
    let generation = bounded_number(root.get("generation"))?;
    if root.get("kind").and_then(JsonValue::as_string) != Some("action_response") {
        return Err("Runtime-v4 REST-action response kind is unsupported");
    }
    let status = root
        .get("status")
        .and_then(JsonValue::as_string)
        .ok_or("Runtime-v4 REST-action response status is invalid")?;
    if !matches!(
        status,
        "accepted" | "settled" | "rejected" | "unknown" | "cancelled"
    ) {
        return Err("Runtime-v4 REST-action response status is invalid");
    }
    match status {
        "accepted" => {
            shape::validate_action_reference(root.get("action"))?;
            require_null(root.get("observation"))?;
            require_null(root.get("transition"))?;
            require_null(root.get("effect_witness"))?;
            require_null(root.get("error_code"))?;
        }
        "settled" => validate_settled(root, generation)?,
        "rejected" | "unknown" | "cancelled" => {
            shape::validate_action_reference(root.get("action"))?;
            require_null(root.get("observation"))?;
            require_null(root.get("transition"))?;
            require_null(root.get("effect_witness"))?;
            validate_identity(root.get("error_code"))?;
        }
        _ => unreachable!(),
    }
    Ok(body.clone())
}

pub(crate) fn validate_runtime_v4_expert_rest_action_reference(
    value: &JsonValue,
) -> Result<(), &'static str> {
    shape::validate_action_reference(Some(value))
}

fn validate_metadata(root: &BTreeMap<String, JsonValue>) -> Result<(), &'static str> {
    if root.get("protocol_version").and_then(JsonValue::as_string)
        != Some(RUNTIME_V4_EXPERT_REST_ACTION_PROTOCOL_VERSION)
        || root.get("schema_digest").and_then(JsonValue::as_string)
            != Some(RUNTIME_V4_EXPERT_REST_ACTION_SCHEMA_DIGEST)
        || root.get("profile").and_then(JsonValue::as_string) != Some("expert-rest-action")
    {
        return Err("Runtime-v4 REST-action metadata is unsupported");
    }
    let provenance = exact_object(root.get("provenance"), &["artifact", "source", "generator"])?;
    if provenance.get("artifact").and_then(JsonValue::as_string)
        != Some(RUNTIME_V4_EXPERT_REST_ACTION_ARTIFACT)
        || provenance.get("source").and_then(JsonValue::as_string)
            != Some(RUNTIME_V4_EXPERT_REST_ACTION_SCHEMA_SOURCE)
        || provenance.get("generator").and_then(JsonValue::as_string)
            != Some(RUNTIME_V4_EXPERT_REST_ACTION_GENERATOR)
    {
        return Err("Runtime-v4 REST-action provenance is unsupported");
    }
    Ok(())
}

fn validate_settled(
    root: &BTreeMap<String, JsonValue>,
    generation: i64,
) -> Result<(), &'static str> {
    shape::validate_action_reference(root.get("action"))?;
    let observation = root
        .get("observation")
        .filter(|value| !matches!(value, JsonValue::Null))
        .ok_or("Runtime-v4 settled REST observation is missing")?;
    let projected = crate::projection::project_runtime_v4_expert_gateway_body(observation)?;
    let projected_object = projected
        .as_object()
        .ok_or("Runtime-v4 settled REST observation is not an object")?;
    if projected_object.get("generation") != Some(&JsonValue::Number(generation))
        || projected_object.get("state_id") != root.get("state_id")
    {
        return Err("Runtime-v4 settled REST observation identity does not match response");
    }
    let transition = root
        .get("transition")
        .filter(|value| !matches!(value, JsonValue::Null))
        .ok_or("Runtime-v4 settled REST transition is missing")?;
    shape::validate_transition(
        root,
        transition,
        projected_object,
        generation,
        root.get("operation_id")
            .and_then(JsonValue::as_string)
            .ok_or("Runtime-v4 operation identity is missing")?,
    )
}

pub(super) fn exact_object<'a>(
    value: Option<&'a JsonValue>,
    fields: &[&str],
) -> Result<&'a BTreeMap<String, JsonValue>, &'static str> {
    let object = value
        .and_then(JsonValue::as_object)
        .ok_or("Runtime-v4 REST value must be an object")?;
    if object.len() != fields.len() || fields.iter().any(|field| !object.contains_key(*field)) {
        return Err("Runtime-v4 REST value contains unknown or missing fields");
    }
    Ok(object)
}

pub(super) fn validate_identity(value: Option<&JsonValue>) -> Result<(), &'static str> {
    match value.and_then(JsonValue::as_string) {
        Some(value)
            if !value.is_empty()
                && value.len() <= 512
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"._:/-".contains(&byte)) =>
        {
            Ok(())
        }
        _ => Err("Runtime-v4 REST identity is invalid"),
    }
}

pub(super) fn bounded_number(value: Option<&JsonValue>) -> Result<i64, &'static str> {
    match value {
        Some(JsonValue::Number(value))
            if (0..=RUNTIME_V4_EXPERT_REST_ACTION_MAX_GENERATION).contains(value) =>
        {
            Ok(*value)
        }
        _ => Err("Runtime-v4 REST number is outside its bound"),
    }
}

pub(super) fn require_null(value: Option<&JsonValue>) -> Result<(), &'static str> {
    matches!(value, Some(JsonValue::Null))
        .then_some(())
        .ok_or("Runtime-v4 REST nullable field is not null")
}

#[cfg(test)]
mod tests {
    use super::project_runtime_v4_expert_rest_action_gateway_body;
    use crate::json::parse_json;

    const GOLDENS: &[&str] = &[
        include_str!(
            "../../../protocol-artifact/runtime-v4-expert-rest-action/golden/action-accepted.json"
        ),
        include_str!(
            "../../../protocol-artifact/runtime-v4-expert-rest-action/golden/action-completed.json"
        ),
        include_str!(
            "../../../protocol-artifact/runtime-v4-expert-rest-action/golden/action-mend-selection-completed.json"
        ),
        include_str!(
            "../../../protocol-artifact/runtime-v4-expert-rest-action/golden/action-mend-selection-requested.json"
        ),
        include_str!(
            "../../../protocol-artifact/runtime-v4-expert-rest-action/golden/action-rejected.json"
        ),
        include_str!(
            "../../../protocol-artifact/runtime-v4-expert-rest-action/golden/action-selection-completed.json"
        ),
        include_str!(
            "../../../protocol-artifact/runtime-v4-expert-rest-action/golden/action-selection-early-confirm-rejected.json"
        ),
        include_str!(
            "../../../protocol-artifact/runtime-v4-expert-rest-action/golden/action-selection-progressed.json"
        ),
        include_str!(
            "../../../protocol-artifact/runtime-v4-expert-rest-action/golden/action-selection-requested.json"
        ),
        include_str!(
            "../../../protocol-artifact/runtime-v4-expert-rest-action/golden/action-selection-second-progressed.json"
        ),
        include_str!(
            "../../../protocol-artifact/runtime-v4-expert-rest-action/golden/action-unknown.json"
        ),
    ];

    #[test]
    fn packaged_rest_action_responses_project() {
        for (index, text) in GOLDENS.iter().enumerate() {
            let parsed = parse_json(text);
            assert!(parsed.is_ok(), "golden {index} is invalid JSON");
            if let Ok(value) = parsed {
                assert!(
                    project_runtime_v4_expert_rest_action_gateway_body(&value).is_ok(),
                    "golden {index} was rejected"
                );
            }
        }
    }
}
