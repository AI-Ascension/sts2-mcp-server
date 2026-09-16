// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::json::JsonValue;
use crate::protocol_artifact_game_information::{
    GAME_INFORMATION_ARTIFACT, GAME_INFORMATION_MAX_MESSAGE_BYTES,
    GAME_INFORMATION_PROTOCOL_VERSION, GAME_INFORMATION_SCHEMA_DIGEST,
    GAME_INFORMATION_SCHEMA_SOURCE,
};

use super::GameInformationContext;

#[path = "mapping_game_information_shapes.rs"]
mod shapes;

pub(super) const RESPONSE_TOO_LARGE: &str =
    "game-information response exceeds the message byte limit";

pub(super) fn project_capabilities(
    body: &JsonValue,
    context: &GameInformationContext,
) -> Result<(JsonValue, bool), &'static str> {
    let object = body
        .as_object()
        .ok_or("capabilities response is not an object")?;
    let kind = object
        .get("kind")
        .and_then(JsonValue::as_string)
        .ok_or("capabilities response kind is missing")?;
    if kind == "error_response" {
        validate_header(body, kind, &context.correlation_id)?;
        if object.get("query") != Some(&JsonValue::Null)
            || object.get("result") != Some(&JsonValue::Null)
            || object.get("capabilities") != Some(&JsonValue::Null)
        {
            return Err("capabilities error response carries result data");
        }
        shapes::validate_error(object.get("error").ok_or("capabilities error is missing")?)?;
        bounded(body)?;
        return Ok((body.clone(), true));
    }
    validate_header(body, "capabilities_response", &context.correlation_id)?;
    if object.get("query") != Some(&JsonValue::Null)
        || object.get("result") != Some(&JsonValue::Null)
        || object.get("error") != Some(&JsonValue::Null)
    {
        return Err("capabilities response carries query, result, or error data");
    }
    shapes::validate_capabilities(
        object
            .get("capabilities")
            .ok_or("capabilities are missing")?,
    )?;
    bounded(body)?;
    Ok((body.clone(), false))
}

pub(super) fn project_query(
    body: &JsonValue,
    context: &GameInformationContext,
    query: &JsonValue,
) -> Result<(JsonValue, bool), &'static str> {
    let object = body
        .as_object()
        .ok_or("game-information response is not an object")?;
    let kind = object
        .get("kind")
        .and_then(JsonValue::as_string)
        .ok_or("response kind is missing")?;
    validate_header(body, kind, &context.correlation_id)?;
    match kind {
        "query_response" => {
            if object.get("query") != Some(query)
                || object.get("capabilities") != Some(&JsonValue::Null)
                || object.get("error") != Some(&JsonValue::Null)
            {
                return Err("query response does not echo the request exactly");
            }
            shapes::validate_query_result(
                object.get("result").ok_or("query result is missing")?,
                query,
            )?;
            bounded(body)?;
            Ok((body.clone(), false))
        }
        "error_response" => {
            if object.get("query") != Some(&JsonValue::Null)
                || object.get("result") != Some(&JsonValue::Null)
                || object.get("capabilities") != Some(&JsonValue::Null)
            {
                return Err("error response carries result data");
            }
            shapes::validate_error(object.get("error").ok_or("error details are missing")?)?;
            bounded(body)?;
            Ok((body.clone(), true))
        }
        _ => Err("game-information response kind is not supported"),
    }
}

pub(super) fn project_binding(
    body: &JsonValue,
    context: &GameInformationContext,
) -> Result<(JsonValue, bool), &'static str> {
    const VERSION: &str = "game-information-lookup-binding-v1";
    const DIGEST: &str = "f10f9af01d6be1de104069ba842e7971971e88f27553e782e81174ee7aa1cd58";
    let object = body
        .as_object()
        .ok_or("lookup-binding response is not an object")?;
    let expected = [
        "protocol_version",
        "schema_digest",
        "provenance",
        "correlation_id",
        "kind",
        "binding",
        "discovery",
        "observation",
        "error",
    ];
    if object.len() != expected.len() || expected.iter().any(|key| !object.contains_key(*key)) {
        return Err("lookup-binding response has unknown or missing fields");
    }
    if object.get("protocol_version") != Some(&JsonValue::string(VERSION))
        || object.get("schema_digest") != Some(&JsonValue::string(DIGEST))
        || object.get("correlation_id") != Some(&JsonValue::string(&context.correlation_id))
    {
        return Err("lookup-binding response identity does not match the request");
    }
    let provenance = BTreeMap::from([
        (
            String::from("artifact"),
            JsonValue::string("sts2-protocol/game-information-lookup-binding-v1"),
        ),
        (
            String::from("source"),
            JsonValue::string("schemas/game-information-lookup-binding-v1.schema.json"),
        ),
        (
            String::from("generator"),
            JsonValue::string("hand-authored"),
        ),
    ]);
    if object.get("provenance").and_then(JsonValue::as_object) != Some(&provenance) {
        return Err("lookup-binding response provenance does not match the pinned artifact");
    }
    let kind = object.get("kind").and_then(JsonValue::as_string);
    let is_error = match kind {
        Some("error_response") => {
            if object.get("binding") != Some(&JsonValue::Null)
                || object.get("discovery") != Some(&JsonValue::Null)
                || object.get("observation") != Some(&JsonValue::Null)
            {
                return Err("lookup-binding error carries result data");
            }
            validate_binding_error(object.get("error"))?;
            true
        }
        Some("lookup_binding_discovery_response") => {
            if object.get("binding") == Some(&JsonValue::Null)
                || object.get("discovery") == Some(&JsonValue::Null)
                || object.get("observation") != Some(&JsonValue::Null)
                || object.get("error") != Some(&JsonValue::Null)
            {
                return Err("lookup-binding discovery response has an invalid shape");
            }
            false
        }
        Some("lookup_binding_observation_response") => {
            if object.get("binding") == Some(&JsonValue::Null)
                || object.get("discovery") == Some(&JsonValue::Null)
                || object.get("observation") == Some(&JsonValue::Null)
                || object.get("error") != Some(&JsonValue::Null)
            {
                return Err("lookup-binding observation response has an invalid shape");
            }
            false
        }
        _ => return Err("lookup-binding response kind is not supported"),
    };
    bounded(body)?;
    Ok((body.clone(), is_error))
}

fn validate_binding_error(value: Option<&JsonValue>) -> Result<(), &'static str> {
    let Some(error) = value.and_then(JsonValue::as_object) else {
        return Err("lookup-binding error is missing");
    };
    let expected = ["code", "field", "reason"];
    if error.len() != expected.len() || expected.iter().any(|key| !error.contains_key(*key)) {
        return Err("lookup-binding error has unknown or missing fields");
    }
    match error.get("code").and_then(JsonValue::as_string) {
        Some(
            "unsupported_version"
            | "invalid_identity"
            | "denied_scope"
            | "missing_capability"
            | "stale_snapshot"
            | "mixed_binding"
            | "reobserve_unavailable"
            | "malformed",
        ) => Ok(()),
        _ => Err("lookup-binding error code is invalid"),
    }
}

fn validate_header(
    value: &JsonValue,
    expected_kind: &str,
    correlation: &str,
) -> Result<(), &'static str> {
    let object = value
        .as_object()
        .ok_or("game-information envelope is not an object")?;
    let expected = [
        "protocol_version",
        "schema_digest",
        "provenance",
        "correlation_id",
        "kind",
        "query",
        "result",
        "capabilities",
        "error",
    ];
    if object.len() != expected.len() || expected.iter().any(|key| !object.contains_key(*key)) {
        return Err("game-information envelope has unknown or missing fields");
    }
    if object.get("protocol_version")
        != Some(&JsonValue::String(String::from(
            GAME_INFORMATION_PROTOCOL_VERSION,
        )))
        || object.get("schema_digest")
            != Some(&JsonValue::String(String::from(
                GAME_INFORMATION_SCHEMA_DIGEST,
            )))
        || object.get("correlation_id") != Some(&JsonValue::String(String::from(correlation)))
        || object.get("kind") != Some(&JsonValue::String(String::from(expected_kind)))
    {
        return Err("game-information envelope identity does not match the request");
    }
    let provenance = object
        .get("provenance")
        .and_then(JsonValue::as_object)
        .ok_or("provenance is missing")?;
    let expected_provenance = BTreeMap::from([
        (
            String::from("artifact"),
            JsonValue::string(GAME_INFORMATION_ARTIFACT),
        ),
        (
            String::from("source"),
            JsonValue::string(GAME_INFORMATION_SCHEMA_SOURCE),
        ),
        (
            String::from("generator"),
            JsonValue::string("hand-authored"),
        ),
    ]);
    if provenance != &expected_provenance {
        return Err("game-information provenance does not match the pinned artifact");
    }
    Ok(())
}

fn bounded(value: &JsonValue) -> Result<(), &'static str> {
    if value.to_json().len() <= GAME_INFORMATION_MAX_MESSAGE_BYTES {
        Ok(())
    } else {
        Err(RESPONSE_TOO_LARGE)
    }
}

pub(super) fn projection_error_code(message: &str) -> (&'static str, &'static str) {
    if message == RESPONSE_TOO_LARGE {
        ("game_information_response_too_large", "size")
    } else {
        ("game_information_malformed_response", "malformed_response")
    }
}

pub(super) fn protocol_error_code(body: &JsonValue) -> Option<&str> {
    body.as_object()?
        .get("error")?
        .as_object()?
        .get("code")?
        .as_string()
}

pub(super) fn protocol_error_category(code: &str) -> &'static str {
    match code {
        "unknown_kind"
        | "unsupported_filter"
        | "unsupported_projection"
        | "unsupported_version"
        | "unsupported_field" => "unsupported",
        "unknown_id" | "missing_capability" => "missing",
        "denied_scope" | "read_only_violation" => "denied",
        "stale_snapshot" | "stale_cursor" | "mixed_generation" => "stale",
        "result_limit_exceeded" => "size",
        "invalid_identity" | "invalid_bounds" | "ambiguous_id" => "invalid_input",
        "malformed" => "malformed_response",
        _ => "malformed_response",
    }
}
