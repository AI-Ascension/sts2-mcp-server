// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::json::JsonValue;
use crate::protocol_artifact_game_information::{
    GAME_INFORMATION_ARTIFACT, GAME_INFORMATION_MAX_MESSAGE_BYTES,
    GAME_INFORMATION_PROTOCOL_VERSION, GAME_INFORMATION_SCHEMA_DIGEST,
    GAME_INFORMATION_SCHEMA_SOURCE,
};

use super::GameInformationContext;

#[path = "mapping_game_information_binding_validation.rs"]
pub(super) mod binding_validation;
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
    request: &JsonValue,
) -> Result<(JsonValue, bool), &'static str> {
    let is_error = binding_validation::validate(body, context, request)?;
    bounded(body)?;
    Ok((body.clone(), is_error))
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
    match message {
        RESPONSE_TOO_LARGE => ("game_information_response_too_large", "size"),
        binding_validation::UNSUPPORTED_VERSION | binding_validation::UNSUPPORTED_DIGEST => {
            ("game_information_unsupported_version", "unsupported")
        }
        _ => ("game_information_malformed_response", "malformed_response"),
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
        "unknown_kind" | "unsupported_filter" | "unsupported_projection" | "unsupported_field" => {
            "unsupported"
        }
        "unsupported_version" => "unsupported",
        "unknown_id" | "missing_capability" => "missing",
        "denied_scope" | "read_only_violation" => "denied",
        "stale_snapshot"
        | "stale_cursor"
        | "mixed_generation"
        | "mixed_binding"
        | "reobserve_unavailable" => "stale",
        "result_limit_exceeded" => "size",
        "invalid_identity" | "invalid_bounds" | "ambiguous_id" => "invalid_input",
        "malformed" => "malformed_response",
        _ => "malformed_response",
    }
}

pub(super) fn status_error_category(status: u16) -> &'static str {
    match status {
        400 | 422 => "invalid_input",
        401 | 403 => "denied",
        404 => "missing",
        409 => "stale",
        413 => "size",
        408 | 429 | 500..=599 => "transport",
        _ => "malformed_response",
    }
}
