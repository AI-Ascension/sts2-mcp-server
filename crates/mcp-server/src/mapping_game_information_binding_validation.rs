// SPDX-License-Identifier: MIT

use std::sync::OnceLock;

use crate::json::JsonValue;

use super::GameInformationContext;

const VERSION: &str = "game-information-lookup-binding-v1";
const DIGEST: &str = "f10f9af01d6be1de104069ba842e7971971e88f27553e782e81174ee7aa1cd58";
pub(super) const UNSUPPORTED_VERSION: &str = "lookup-binding protocol version is unsupported";
pub(super) const UNSUPPORTED_DIGEST: &str = "lookup-binding schema digest is unsupported";
const SCHEMA: &str =
    include_str!("../../../protocol-artifact/game-information-lookup-binding-v1/schema.json");
const ARTIFACT_SCHEMA: &str = include_str!(
    "../../../protocol-artifact/game-information-lookup-binding-v1/schemas/game-information-lookup-binding-v1.schema.json"
);

pub(super) fn validate(
    body: &JsonValue,
    context: &GameInformationContext,
    request: &JsonValue,
) -> Result<bool, &'static str> {
    let object = body
        .as_object()
        .ok_or("lookup-binding response is not an object")?;
    if object.get("protocol_version") != Some(&JsonValue::string(VERSION)) {
        return Err(UNSUPPORTED_VERSION);
    }
    if object.get("schema_digest") != Some(&JsonValue::string(DIGEST)) {
        return Err(UNSUPPORTED_DIGEST);
    }
    validate_schema(body)?;
    if object.get("correlation_id") != Some(&JsonValue::string(&context.correlation_id)) {
        return Err("lookup-binding response correlation does not match the request");
    }
    let kind = object
        .get("kind")
        .and_then(JsonValue::as_string)
        .ok_or("lookup-binding response kind is missing")?;
    match kind {
        "error_response" => {
            let error = object
                .get("error")
                .and_then(JsonValue::as_object)
                .ok_or("lookup-binding error is missing")?;
            let code = error
                .get("code")
                .and_then(JsonValue::as_string)
                .ok_or("lookup-binding error code is missing")?;
            if code == "reobserve_unavailable" {
                let attempts =
                    member_at(body, &["discovery", "reobserve", "attempts"]).and_then(|value| {
                        match value {
                            JsonValue::Number(value) => Some(*value),
                            _ => None,
                        }
                    });
                if request_value(request, "operation")? != "observe"
                    || object.get("binding") == Some(&JsonValue::Null)
                    || object.get("discovery") == Some(&JsonValue::Null)
                    || member_at(body, &["discovery", "observation_state"])
                        != Some(&JsonValue::string("reobserve_exhausted"))
                    || attempts.is_none_or(|attempts| attempts < 1)
                    || object.get("observation") != Some(&JsonValue::Null)
                {
                    return Err("reobserve-unavailable error disagrees with its terminal state");
                }
                validate_binding_identity(object, context, request)?;
            } else if object.get("binding") != Some(&JsonValue::Null)
                || object.get("discovery") != Some(&JsonValue::Null)
            {
                return Err("lookup-binding error carries an unrecognized result state");
            }
            Ok(true)
        }
        "lookup_binding_discovery_response" | "lookup_binding_observation_response" => {
            let expected_kind = match request_value(request, "operation")? {
                "discovery" => "lookup_binding_discovery_response",
                "observe" => "lookup_binding_observation_response",
                _ => return Err("lookup-binding operation is unsupported"),
            };
            if kind != expected_kind {
                return Err("lookup-binding response kind does not match the request");
            }
            if object.get("error") != Some(&JsonValue::Null) {
                return Err("lookup-binding success carries an error");
            }
            if member_at(body, &["discovery", "required_capabilities", "profile"])
                != Some(&JsonValue::string(VERSION))
                || member_at(
                    body,
                    &["discovery", "required_capabilities", "schema_digest"],
                ) != Some(&JsonValue::string(DIGEST))
            {
                return Err("lookup-binding required capability is unsupported");
            }
            validate_binding_identity(object, context, request)?;
            if let Some(JsonValue::Object(observation)) = object.get("observation")
                && observation.get("binding_id") != member_at(body, &["binding", "binding_id"])
            {
                return Err("lookup-binding observation crosses binding identity");
            }
            Ok(false)
        }
        _ => Err("lookup-binding response kind is not supported"),
    }
}

fn validate_schema(body: &JsonValue) -> Result<(), &'static str> {
    if crate::protocol_artifact_hash::sha256_hex(SCHEMA.as_bytes()) != DIGEST
        || ARTIFACT_SCHEMA != SCHEMA
    {
        return Err("lookup-binding schema artifact is not pinned");
    }
    static VALIDATOR: OnceLock<Result<jsonschema::Validator, ()>> = OnceLock::new();
    let validator = VALIDATOR
        .get_or_init(|| {
            let schema = serde_json::from_str(SCHEMA).map_err(|_| ())?;
            jsonschema::draft202012::options()
                .build(&schema)
                .map_err(|_| ())
        })
        .as_ref()
        .map_err(|_| "lookup-binding schema is unavailable")?;
    let value: serde_json::Value =
        serde_json::from_str(&body.to_json()).map_err(|_| "lookup-binding JSON is invalid")?;
    if validator.is_valid(&value) {
        Ok(())
    } else {
        Err("lookup-binding response does not match its pinned schema")
    }
}

fn validate_binding_identity(
    object: &std::collections::BTreeMap<String, JsonValue>,
    context: &GameInformationContext,
    request: &JsonValue,
) -> Result<(), &'static str> {
    let request_object = request
        .as_object()
        .ok_or("lookup-binding request is not an object")?;
    let binding = object
        .get("binding")
        .and_then(JsonValue::as_object)
        .ok_or("lookup-binding response binding is missing")?;
    let scope = JsonValue::object([
        (
            String::from("project_id"),
            request_member(request, "project_id")?.clone(),
        ),
        (
            String::from("run_id"),
            request_member(request, "run_id")?.clone(),
        ),
        (
            String::from("episode_id"),
            request_member(request, "episode_id")?.clone(),
        ),
        (
            String::from("agent_id"),
            request_member(request, "agent_id")?.clone(),
        ),
    ]);
    if binding.get("scope") != Some(&scope)
        || binding.get("instance_id") != Some(&JsonValue::string(&context.instance_id))
        || binding.get("authority_epoch") != request_object.get("authority_epoch")
    {
        return Err("lookup-binding owner identity does not match the request");
    }
    let supplied = binding
        .get("binding_id")
        .and_then(JsonValue::as_string)
        .ok_or("lookup-binding identifier is missing")?;
    if supplied != canonical_binding_id(binding)? {
        return Err("lookup-binding identifier does not match its canonical identity");
    }
    Ok(())
}

pub(crate) fn canonical_binding_id(
    binding: &std::collections::BTreeMap<String, JsonValue>,
) -> Result<String, &'static str> {
    let scope = binding
        .get("scope")
        .and_then(JsonValue::as_object)
        .ok_or("lookup-binding scope is missing")?;
    let identity = JsonValue::object([
        (
            String::from("agent_id"),
            scope
                .get("agent_id")
                .ok_or("binding agent is missing")?
                .clone(),
        ),
        (
            String::from("authority_epoch"),
            binding
                .get("authority_epoch")
                .ok_or("binding epoch is missing")?
                .clone(),
        ),
        (
            String::from("content_manifest_id"),
            binding
                .get("content_manifest_id")
                .ok_or("binding manifest is missing")?
                .clone(),
        ),
        (
            String::from("episode_id"),
            scope
                .get("episode_id")
                .ok_or("binding episode is missing")?
                .clone(),
        ),
        (
            String::from("game_profile"),
            binding
                .get("game_profile")
                .ok_or("binding profile is missing")?
                .clone(),
        ),
        (
            String::from("locale"),
            binding
                .get("locale")
                .ok_or("binding locale is missing")?
                .clone(),
        ),
        (
            String::from("project_id"),
            scope
                .get("project_id")
                .ok_or("binding project is missing")?
                .clone(),
        ),
        (
            String::from("run_id"),
            scope.get("run_id").ok_or("binding run is missing")?.clone(),
        ),
    ]);
    Ok(crate::protocol_artifact_hash::sha256_hex(
        identity.to_json().as_bytes(),
    ))
}

fn request_member<'a>(request: &'a JsonValue, name: &str) -> Result<&'a JsonValue, &'static str> {
    request
        .as_object()
        .and_then(|object| object.get(name))
        .ok_or("lookup-binding request identity is incomplete")
}

fn member_at<'a>(value: &'a JsonValue, path: &[&str]) -> Option<&'a JsonValue> {
    let mut current = value;
    for member in path {
        current = current.as_object()?.get(*member)?;
    }
    Some(current)
}

fn request_value<'a>(request: &'a JsonValue, name: &str) -> Result<&'a str, &'static str> {
    request_member(request, name)?
        .as_string()
        .ok_or("lookup-binding request operation is invalid")
}
