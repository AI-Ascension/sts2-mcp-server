// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::json::JsonValue;
use crate::protocol_artifact_runtime_v2::{
    RUNTIME_V2_ARTIFACT, RUNTIME_V2_GENERATOR, RUNTIME_V2_MAX_GENERATION,
    RUNTIME_V2_PROTOCOL_VERSION, RUNTIME_V2_SCHEMA_DIGEST, RUNTIME_V2_SCHEMA_SOURCE,
};

#[path = "projection_runtime_v2_shape.rs"]
mod shape;

/// Request identity used to fence and correlate a Runtime-v2 gateway result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RuntimeV2Context {
    pub(crate) correlation_id: String,
    pub(crate) instance_id: String,
    pub(crate) session_id: String,
    pub(crate) lease_id: String,
    pub(crate) lease_epoch: i64,
    pub(crate) generation: i64,
    pub(crate) operation_id: String,
}

pub(crate) fn validate_runtime_v2_gateway_body(
    body: &JsonValue,
    context: &RuntimeV2Context,
    expected_kind: &str,
) -> Result<(), &'static str> {
    let Some(object) = body.as_object() else {
        return Err("runtime-v2 gateway response must be an object");
    };
    let kind = validate_envelope(object)?;
    if kind != expected_kind {
        return Err("runtime-v2 gateway response kind does not match the request");
    }
    validate_context(object, context, kind)?;
    shape::validate_kind_shape(object, kind, context.generation)
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
        return Err("runtime-v2 gateway envelope contains unknown or missing fields");
    }
    if object
        .get("protocol_version")
        .and_then(JsonValue::as_string)
        != Some(RUNTIME_V2_PROTOCOL_VERSION)
    {
        return Err("runtime-v2 gateway protocol version is unsupported");
    }
    if object.get("schema_digest").and_then(JsonValue::as_string) != Some(RUNTIME_V2_SCHEMA_DIGEST)
    {
        return Err("runtime-v2 gateway schema digest is unsupported");
    }
    validate_provenance(object.get("provenance"))?;
    for key in ["correlation_id", "instance_id", "session_id", "lease_id"] {
        let Some(value) = object.get(key).and_then(JsonValue::as_string) else {
            return Err("runtime-v2 gateway identity is missing or not a string");
        };
        if !shape::safe_identity(value) {
            return Err("runtime-v2 gateway identity is unsafe or oversized");
        }
    }
    shape::bounded_number(
        object
            .get("lease_epoch")
            .ok_or("runtime-v2 gateway lease_epoch is missing")?,
        RUNTIME_V2_MAX_GENERATION,
    )?;
    shape::bounded_number(
        object
            .get("generation")
            .ok_or("runtime-v2 gateway generation is missing")?,
        RUNTIME_V2_MAX_GENERATION,
    )?;
    let Some(kind) = object.get("kind").and_then(JsonValue::as_string) else {
        return Err("runtime-v2 gateway kind must be a string");
    };
    if !matches!(
        kind,
        "state_request"
            | "state_response"
            | "action_request"
            | "action_response"
            | "reconcile_request"
            | "reconcile_response"
    ) {
        return Err("runtime-v2 gateway kind is not allowlisted");
    }
    let operation_id = object
        .get("operation_id")
        .ok_or("runtime-v2 gateway operation_id is missing")?;
    if matches!(kind, "state_request" | "state_response") {
        if !matches!(operation_id, JsonValue::Null) {
            return Err("runtime-v2 state messages must not carry an operation_id");
        }
    } else {
        let Some(operation_id) = operation_id.as_string() else {
            return Err("runtime-v2 operation_id must be a string");
        };
        if !shape::safe_identity(operation_id) {
            return Err("runtime-v2 operation_id is unsafe or oversized");
        }
    }
    Ok(kind)
}

fn validate_provenance(value: Option<&JsonValue>) -> Result<(), &'static str> {
    let Some(provenance) = value.and_then(JsonValue::as_object) else {
        return Err("runtime-v2 gateway provenance is missing");
    };
    if provenance.len() != 3
        || provenance.get("artifact").and_then(JsonValue::as_string) != Some(RUNTIME_V2_ARTIFACT)
        || provenance.get("source").and_then(JsonValue::as_string) != Some(RUNTIME_V2_SCHEMA_SOURCE)
        || provenance.get("generator").and_then(JsonValue::as_string) != Some(RUNTIME_V2_GENERATOR)
    {
        return Err("runtime-v2 gateway provenance is unsupported");
    }
    Ok(())
}

fn validate_context(
    object: &BTreeMap<String, JsonValue>,
    context: &RuntimeV2Context,
    kind: &str,
) -> Result<(), &'static str> {
    for (key, expected) in [
        ("correlation_id", context.correlation_id.as_str()),
        ("instance_id", context.instance_id.as_str()),
        ("session_id", context.session_id.as_str()),
        ("lease_id", context.lease_id.as_str()),
    ] {
        if object.get(key).and_then(JsonValue::as_string) != Some(expected) {
            return Err("runtime-v2 gateway response identity does not match the request");
        }
    }
    if shape::bounded_number(
        object
            .get("lease_epoch")
            .ok_or("runtime-v2 gateway lease_epoch is missing")?,
        RUNTIME_V2_MAX_GENERATION,
    )? != context.lease_epoch
    {
        return Err("runtime-v2 gateway response lease fence does not match the request");
    }
    if kind != "state_response"
        && object.get("operation_id").and_then(JsonValue::as_string)
            != Some(context.operation_id.as_str())
    {
        return Err("runtime-v2 gateway response operation_id does not match the request");
    }
    Ok(())
}
