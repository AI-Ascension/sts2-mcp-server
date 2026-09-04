// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::json::JsonValue;
use crate::protocol_artifact::{
    RUNTIME_ACTION_ID, RUNTIME_ARTIFACT, RUNTIME_GENERATOR, RUNTIME_MAX_GENERATION,
    RUNTIME_PROTOCOL_VERSION, RUNTIME_SCHEMA_DIGEST, RUNTIME_SCHEMA_SOURCE,
};

use super::{bounded_number, safe_identifier};

pub(crate) fn project_runtime_gateway_body(body: &JsonValue) -> Result<JsonValue, &'static str> {
    let Some(object) = body.as_object() else {
        return Err("runtime gateway response must be an object");
    };
    validate_runtime_envelope(object)?;
    validate_response_shape(object)?;
    let mut projection = Vec::new();
    if let Some(value) = object.get("kind") {
        let Some(kind) = value.as_string() else {
            return Err("runtime gateway kind must be a string");
        };
        if !matches!(kind, "state_response" | "action_response") {
            return Err("runtime gateway kind is not allowlisted");
        }
        projection.push((String::from("kind"), JsonValue::string(kind)));
    }
    if let Some(value) = object.get("generation") {
        projection.push((
            String::from("generation"),
            JsonValue::Number(bounded_number(value, RUNTIME_MAX_GENERATION)?),
        ));
    }
    if let Some(value) = object.get("observation") {
        projection.push((
            String::from("observation"),
            project_runtime_observation(value)?,
        ));
    }
    if let Some(value) = object.get("action")
        && !matches!(value, JsonValue::Null)
    {
        projection.push((String::from("action"), project_runtime_action(value)?));
    }
    if let Some(value) = object.get("status")
        && !matches!(value, JsonValue::Null)
    {
        let Some(status) = value.as_string() else {
            return Err("runtime gateway status must be a string");
        };
        if !matches!(status, "accepted" | "rejected") {
            return Err("runtime gateway status is not allowlisted");
        }
        projection.push((String::from("status"), JsonValue::string(status)));
    }
    if let Some(value) = object.get("error_code")
        && !matches!(value, JsonValue::Null)
    {
        let Some(error_code) = value.as_string() else {
            return Err("runtime gateway error_code must be a string");
        };
        if !safe_identifier(error_code) {
            return Err("runtime gateway error_code is unsafe or oversized");
        }
        projection.push((String::from("error_code"), JsonValue::string(error_code)));
    }
    if let Some(value) = object.get("effect_witness")
        && !matches!(value, JsonValue::Null)
    {
        projection.push((
            String::from("effect_witness"),
            project_runtime_effect_witness(value)?,
        ));
    }
    if !projection.iter().any(|(key, _)| {
        matches!(
            key.as_str(),
            "observation" | "action" | "status" | "error_code" | "effect_witness"
        )
    }) {
        return Err("runtime gateway response contains no allowlisted fields");
    }
    Ok(JsonValue::object(projection))
}

fn validate_runtime_envelope(object: &BTreeMap<String, JsonValue>) -> Result<(), &'static str> {
    const REQUIRED: [&str; 15] = [
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
        "observation",
        "action",
        "status",
        "error_code",
        "effect_witness",
    ];
    if object.len() != REQUIRED.len() || REQUIRED.iter().any(|key| !object.contains_key(*key)) {
        return Err("runtime gateway envelope contains unknown or missing fields");
    }
    if object
        .get("protocol_version")
        .and_then(JsonValue::as_string)
        != Some(RUNTIME_PROTOCOL_VERSION)
    {
        return Err("runtime gateway protocol version is unsupported");
    }
    if object.get("schema_digest").and_then(JsonValue::as_string) != Some(RUNTIME_SCHEMA_DIGEST) {
        return Err("runtime gateway schema digest is unsupported");
    }
    let Some(provenance) = object.get("provenance").and_then(JsonValue::as_object) else {
        return Err("runtime gateway provenance is missing");
    };
    if provenance.len() != 3
        || provenance.get("artifact").and_then(JsonValue::as_string) != Some(RUNTIME_ARTIFACT)
        || provenance.get("source").and_then(JsonValue::as_string) != Some(RUNTIME_SCHEMA_SOURCE)
        || provenance.get("generator").and_then(JsonValue::as_string) != Some(RUNTIME_GENERATOR)
    {
        return Err("runtime gateway provenance is unsupported");
    }
    for key in ["correlation_id", "instance_id", "session_id", "lease_id"] {
        let Some(value) = object.get(key).and_then(JsonValue::as_string) else {
            return Err("runtime gateway identity is missing");
        };
        if !safe_identifier(value) {
            return Err("runtime gateway identity is unsafe or oversized");
        }
    }
    bounded_number(
        object
            .get("lease_epoch")
            .ok_or("runtime gateway lease epoch is missing")?,
        RUNTIME_MAX_GENERATION,
    )?;
    bounded_number(
        object
            .get("generation")
            .ok_or("runtime gateway generation is missing")?,
        RUNTIME_MAX_GENERATION,
    )?;
    Ok(())
}

fn project_runtime_observation(value: &JsonValue) -> Result<JsonValue, &'static str> {
    let Some(object) = value.as_object() else {
        return Err("runtime gateway observation must be an object");
    };
    if object.len() != 4 {
        return Err("runtime observation contains unknown or missing fields");
    }
    let host_ready = match object.get("host_ready") {
        Some(JsonValue::Bool(value)) => *value,
        _ => return Err("runtime observation host_ready must be a boolean"),
    };
    let overlay_visible = match object.get("overlay_visible") {
        Some(JsonValue::Bool(value)) => *value,
        _ => return Err("runtime observation overlay_visible must be a boolean"),
    };
    let Some(screen) = object.get("screen").and_then(JsonValue::as_string) else {
        return Err("runtime observation screen must be a string");
    };
    if screen.is_empty()
        || screen.len() > 64
        || !screen.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'/' | b'-')
        })
    {
        return Err("runtime observation screen is unsafe or oversized");
    }
    let action_count = bounded_number(
        object
            .get("action_count")
            .ok_or("runtime observation is missing action_count")?,
        1024,
    )?;
    Ok(JsonValue::object([
        ("host_ready".to_owned(), JsonValue::Bool(host_ready)),
        (
            "overlay_visible".to_owned(),
            JsonValue::Bool(overlay_visible),
        ),
        ("screen".to_owned(), JsonValue::string(screen)),
        ("action_count".to_owned(), JsonValue::Number(action_count)),
    ]))
}

fn project_runtime_action(value: &JsonValue) -> Result<JsonValue, &'static str> {
    let Some(object) = value.as_object() else {
        return Err("runtime gateway action must be an object");
    };
    if object.len() != 1 {
        return Err("runtime gateway action contains unexpected fields");
    }
    let Some(action_id) = object.get("action_id").and_then(JsonValue::as_string) else {
        return Err("runtime gateway action is missing action_id");
    };
    if action_id != RUNTIME_ACTION_ID {
        return Err("runtime gateway action_id is not allowlisted");
    }
    Ok(JsonValue::object([(
        String::from("action_id"),
        JsonValue::string(action_id),
    )]))
}

fn project_runtime_effect_witness(value: &JsonValue) -> Result<JsonValue, &'static str> {
    let Some(object) = value.as_object() else {
        return Err("runtime gateway effect witness must be an object");
    };
    if object.len() != 2
        || object.get("kind").and_then(JsonValue::as_string) != Some("status_overlay_visible")
    {
        return Err("runtime gateway effect witness is not allowlisted");
    }
    let generation = bounded_number(
        object
            .get("generation")
            .ok_or("runtime gateway effect witness is missing generation")?,
        RUNTIME_MAX_GENERATION,
    )?;
    Ok(JsonValue::object([
        (
            String::from("kind"),
            JsonValue::string("status_overlay_visible"),
        ),
        (String::from("generation"), JsonValue::Number(generation)),
    ]))
}

fn validate_response_shape(object: &BTreeMap<String, JsonValue>) -> Result<(), &'static str> {
    // Required-field closure is checked before this function; missing is never null.
    let is_null = |key| object.get(key) == Some(&JsonValue::Null);
    match object.get("kind").and_then(JsonValue::as_string) {
        Some("state_response") => {
            if ["action", "status", "error_code", "effect_witness"]
                .iter()
                .any(|key| !is_null(*key))
            {
                return Err("runtime state response contains action result fields");
            }
        }
        Some("action_response") => {
            project_runtime_action(object.get("action").ok_or("missing runtime action")?)?;
            match object.get("status").and_then(JsonValue::as_string) {
                Some("accepted") => {
                    if !is_null("error_code") {
                        return Err("runtime accepted result contains an error");
                    }
                    let witness = object
                        .get("effect_witness")
                        .ok_or("missing runtime witness")?;
                    project_runtime_effect_witness(witness)?;
                    if witness
                        .as_object()
                        .and_then(|value| value.get("generation"))
                        != object.get("generation")
                    {
                        return Err("runtime witness generation does not match the envelope");
                    }
                }
                Some("rejected") => {
                    if !is_null("effect_witness")
                        || !object
                            .get("error_code")
                            .and_then(JsonValue::as_string)
                            .is_some_and(safe_identifier)
                    {
                        return Err("runtime rejected result requires an error and no witness");
                    }
                }
                _ => return Err("runtime action response status is unsupported"),
            }
        }
        _ => return Err("runtime gateway response kind is unsupported"),
    }
    Ok(())
}
