// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use super::{MESSAGE_FIELDS, SeededContext, request, validation};
use crate::gateway::GatewayError;
use crate::json::JsonValue;
use crate::protocol::{RequestId, RpcResponse};
use crate::protocol_artifact_seeded_run::{
    SEEDED_RUN_ARTIFACT, SEEDED_RUN_GENERATOR, SEEDED_RUN_PROTOCOL_VERSION,
    SEEDED_RUN_SCHEMA_DIGEST, SEEDED_RUN_SCHEMA_SOURCE,
};

pub(super) fn gateway_success(
    id: RequestId,
    response: crate::gateway::GatewayResponse,
    context: &SeededContext,
    expected_kind: &str,
    requested_seed: Option<&str>,
    run_mode: Option<&str>,
) -> RpcResponse {
    if validate_response(
        &response.body,
        context,
        expected_kind,
        requested_seed,
        run_mode,
    )
    .is_err()
    {
        return super::super::tool_result(
            id,
            "gateway response is not a valid seeded-run envelope",
            true,
        );
    }
    let is_error = !(200..300).contains(&response.status)
        || matches!(
            status(&response.body),
            Some("rejected" | "unknown" | "cancelled")
        );
    super::super::tool_result(id, response.body.to_json(), is_error)
}

pub(super) fn unknown_result(
    id: RequestId,
    context: &SeededContext,
    requested_seed: Option<&str>,
    run_mode: Option<&str>,
    error: GatewayError,
) -> RpcResponse {
    let mut response = request::base_message(context, "start_response", &context.operation_id);
    if let JsonValue::Object(object) = &mut response {
        object.insert(
            String::from("requested_seed"),
            JsonValue::string(requested_seed.unwrap_or("")),
        );
        object.insert(
            String::from("run_mode"),
            JsonValue::string(run_mode.unwrap_or("diagnostic")),
        );
        object.insert(String::from("status"), JsonValue::string("unknown"));
        object.insert(
            String::from("error_code"),
            JsonValue::string(match error {
                GatewayError::Timeout => "sts2.seeded_run/unknown_after_disconnect",
                GatewayError::Unavailable => "sts2.seeded_run/downstream_unavailable",
                _ => "sts2.seeded_run/unknown",
            }),
        );
    }
    super::super::tool_result(id, response.to_json(), true)
}

fn status(body: &JsonValue) -> Option<&str> {
    body.as_object()?
        .get("status")
        .and_then(JsonValue::as_string)
}

fn validate_response(
    body: &JsonValue,
    context: &SeededContext,
    expected_kind: &str,
    requested_seed: Option<&str>,
    run_mode: Option<&str>,
) -> Result<(), &'static str> {
    let object = body
        .as_object()
        .ok_or("seeded-run response must be an object")?;
    if object.len() != MESSAGE_FIELDS.len()
        || object
            .keys()
            .any(|key| !MESSAGE_FIELDS.contains(&key.as_str()))
    {
        return Err("seeded-run response has unknown or missing fields");
    }
    validate_metadata(object)?;
    validate_identity(object, context)?;
    if validation::bounded_value(object.get("lease_epoch"))? != context.lease_epoch
        || validation::bounded_value(object.get("generation"))? != context.generation
    {
        return Err("seeded-run response lease or generation does not match the request");
    }
    if object.get("kind").and_then(JsonValue::as_string) != Some(expected_kind) {
        return Err("seeded-run response kind does not match the request");
    }
    let returned_seed = object
        .get("requested_seed")
        .and_then(JsonValue::as_string)
        .ok_or("seeded-run response requested_seed is missing")?;
    if !validation::validate_seed(returned_seed)
        || requested_seed.is_some_and(|value| value != returned_seed)
    {
        return Err("seeded-run response requested_seed does not match the request");
    }
    let returned_mode = object
        .get("run_mode")
        .and_then(JsonValue::as_string)
        .ok_or("seeded-run response run_mode is missing")?;
    if !validation::validate_mode(returned_mode)
        || run_mode.is_some_and(|value| value != returned_mode)
    {
        return Err("seeded-run response run_mode does not match the request");
    }
    let selected = object
        .get("selected_context")
        .ok_or("seeded-run response selected_context is missing")?;
    let selected_digest = validation::validate_selected_context(selected)?;
    if context
        .selected_context
        .as_ref()
        .is_some_and(|expected| expected != selected)
    {
        return Err("seeded-run response selected_context does not match the request");
    }
    if object.get("context_digest").and_then(JsonValue::as_string) != Some(selected_digest.as_str())
    {
        return Err("seeded-run response context_digest does not match selected_context");
    }
    validation::validate_optional_seed(object.get("canonical_seed"))?;
    validation::validate_observation(object.get("observation"))?;
    validation::validate_witness(object.get("effect_witness"))?;
    validation::validate_optional_error(object.get("error_code"))?;
    validate_status(object, context, selected_digest.as_str())
}

fn validate_metadata(object: &BTreeMap<String, JsonValue>) -> Result<(), &'static str> {
    if object
        .get("protocol_version")
        .and_then(JsonValue::as_string)
        != Some(SEEDED_RUN_PROTOCOL_VERSION)
        || object.get("schema_digest").and_then(JsonValue::as_string)
            != Some(SEEDED_RUN_SCHEMA_DIGEST)
    {
        return Err("seeded-run response metadata is unsupported");
    }
    let provenance = object
        .get("provenance")
        .and_then(JsonValue::as_object)
        .ok_or("seeded-run response provenance is missing")?;
    if provenance.len() != 3
        || provenance.get("artifact").and_then(JsonValue::as_string) != Some(SEEDED_RUN_ARTIFACT)
        || provenance.get("source").and_then(JsonValue::as_string) != Some(SEEDED_RUN_SCHEMA_SOURCE)
        || provenance.get("generator").and_then(JsonValue::as_string) != Some(SEEDED_RUN_GENERATOR)
    {
        return Err("seeded-run response provenance is unsupported");
    }
    Ok(())
}

fn validate_identity(
    object: &BTreeMap<String, JsonValue>,
    context: &SeededContext,
) -> Result<(), &'static str> {
    for (key, expected) in [
        ("correlation_id", context.correlation_id.as_str()),
        ("instance_id", context.instance_id.as_str()),
        ("session_id", context.session_id.as_str()),
        ("lease_id", context.lease_id.as_str()),
        ("operation_id", context.operation_id.as_str()),
    ] {
        if object.get(key).and_then(JsonValue::as_string) != Some(expected)
            || !validation::validate_identity(expected)
        {
            return Err("seeded-run response identity does not match the request");
        }
    }
    Ok(())
}

fn validate_status(
    object: &BTreeMap<String, JsonValue>,
    context: &SeededContext,
    selected_digest: &str,
) -> Result<(), &'static str> {
    let status = object
        .get("status")
        .and_then(JsonValue::as_string)
        .ok_or("seeded-run response status is missing")?;
    match status {
        "accepted" => {
            validation::require_null(object, "canonical_seed")?;
            validation::require_null(object, "observation")?;
            validation::require_null(object, "effect_witness")?;
            validation::require_null(object, "error_code")?;
        }
        "settled" => validate_settled(object, context, selected_digest)?,
        "rejected" | "cancelled" => {
            validation::require_string(object, "error_code")?;
            validation::require_null(object, "canonical_seed")?;
            validation::require_null(object, "observation")?;
            validation::require_null(object, "effect_witness")?;
        }
        "unknown" => {
            validation::require_string(object, "error_code")?;
            validation::require_null(object, "canonical_seed")?;
            validation::require_null(object, "observation")?;
            validation::require_null(object, "effect_witness")?;
        }
        _ => return Err("seeded-run response status is not allowlisted"),
    }
    Ok(())
}

fn validate_settled(
    object: &BTreeMap<String, JsonValue>,
    context: &SeededContext,
    selected_digest: &str,
) -> Result<(), &'static str> {
    let canonical_seed = validation::required_string(object, "canonical_seed")?;
    let observation = validation::required_object(object, "observation")?;
    let witness = validation::required_object(object, "effect_witness")?;
    if observation.get("run_started") != Some(&JsonValue::Bool(true))
        || observation.get("host_ready") != Some(&JsonValue::Bool(true))
        || validation::bounded_value(observation.get("generation"))? <= context.generation
        || observation
            .get("canonical_seed")
            .and_then(JsonValue::as_string)
            != Some(canonical_seed)
        || observation
            .get("selected_context_digest")
            .and_then(JsonValue::as_string)
            != Some(selected_digest)
        || witness.get("generation") != observation.get("generation")
        || witness.get("canonical_seed").and_then(JsonValue::as_string) != Some(canonical_seed)
    {
        return Err("seeded-run settlement lacks a fresh matching host witness");
    }
    validation::require_null(object, "error_code")
}
