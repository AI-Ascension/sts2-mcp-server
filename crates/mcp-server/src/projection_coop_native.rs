// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::json::JsonValue;

#[path = "projection_coop_native_observation.rs"]
mod observation;
#[path = "projection_coop_native_shapes.rs"]
mod shapes;
use crate::protocol_artifact_coop_native::{
    COOP_NATIVE_ARTIFACT, COOP_NATIVE_GENERATOR, COOP_NATIVE_MAX_BODY_BYTES,
    COOP_NATIVE_PROTOCOL_VERSION, COOP_NATIVE_SCHEMA_DIGEST, COOP_NATIVE_SCHEMA_SOURCE,
};
use shapes::{effect_response, observation_response, recovery_response};

const TOP_LEVEL_FIELDS: [&str; 18] = [
    "protocol_version",
    "schema_digest",
    "provenance",
    "correlation_id",
    "instance_id",
    "session_id",
    "lease_id",
    "lease_epoch",
    "kind",
    "operation_id",
    "actor_peer",
    "expected_host_generation",
    "action",
    "vote",
    "status",
    "observation",
    "effect",
    "recovery",
];
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct NativeContext {
    pub(crate) correlation: String,
    pub(crate) instance: String,
    pub(crate) session: String,
    pub(crate) lease: String,
    pub(crate) epoch: i64,
}

pub(crate) fn project_coop_native_response(
    body: &JsonValue,
    context: &NativeContext,
    expected_kind: &str,
    status_code: u16,
    expected_operation: Option<&str>,
) -> Result<(JsonValue, bool), &'static str> {
    let object = exact_object(body, &TOP_LEVEL_FIELDS, "native co-op response")?;
    if body.to_json().len() > COOP_NATIVE_MAX_BODY_BYTES {
        return Err("native co-op response exceeds the protocol body limit");
    }
    validate_metadata(object, context, expected_kind)?;
    if let Some(expected_operation) = expected_operation
        && object.get("operation_id").and_then(JsonValue::as_string) != Some(expected_operation)
    {
        return Err("native co-op response operation identity mismatched");
    }
    match expected_kind {
        "observation" => observation_response(object)?,
        "effect_response" => effect_response(object)?,
        "recovery_response" => recovery_response(object)?,
        _ => return Err("native co-op response kind is unsupported"),
    }
    let status = object
        .get("status")
        .and_then(JsonValue::as_string)
        .unwrap_or("");
    Ok((
        body.clone(),
        !(200..300).contains(&status_code) || matches!(status, "rejected" | "unknown"),
    ))
}

pub(crate) fn project_coop_native_effect(
    body: &JsonValue,
    context: &NativeContext,
) -> Result<(JsonValue, bool), &'static str> {
    project_coop_native_response(body, context, "effect_response", 200, None)
}

fn validate_metadata(
    object: &BTreeMap<String, JsonValue>,
    context: &NativeContext,
    expected_kind: &str,
) -> Result<(), &'static str> {
    for (field, expected) in [
        ("protocol_version", COOP_NATIVE_PROTOCOL_VERSION),
        ("schema_digest", COOP_NATIVE_SCHEMA_DIGEST),
        ("correlation_id", context.correlation.as_str()),
        ("instance_id", context.instance.as_str()),
        ("session_id", context.session.as_str()),
        ("lease_id", context.lease.as_str()),
        ("kind", expected_kind),
    ] {
        if object.get(field).and_then(JsonValue::as_string) != Some(expected) {
            return Err("native co-op response metadata or identity mismatched");
        }
    }
    if object.get("lease_epoch") != Some(&JsonValue::Number(context.epoch)) {
        return Err("native co-op response lease epoch mismatched");
    }
    let provenance = exact_object(
        object
            .get("provenance")
            .ok_or("native co-op response provenance is missing")?,
        &["artifact", "source", "generator"],
        "native co-op response provenance",
    )?;
    for (field, expected) in [
        ("artifact", COOP_NATIVE_ARTIFACT),
        ("source", COOP_NATIVE_SCHEMA_SOURCE),
        ("generator", COOP_NATIVE_GENERATOR),
    ] {
        if provenance.get(field).and_then(JsonValue::as_string) != Some(expected) {
            return Err("native co-op response provenance is unsupported");
        }
    }
    Ok(())
}

fn exact_object<'a>(
    value: &'a JsonValue,
    fields: &[&str],
    label: &'static str,
) -> Result<&'a BTreeMap<String, JsonValue>, &'static str> {
    let object = value.as_object().ok_or(label)?;
    if object.len() != fields.len() || fields.iter().any(|field| !object.contains_key(*field)) {
        return Err(label);
    }
    Ok(object)
}
