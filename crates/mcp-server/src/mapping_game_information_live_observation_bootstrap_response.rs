// SPDX-License-Identifier: MIT

use std::collections::BTreeSet;

use crate::json::JsonValue;
use crate::{
    LIVE_BOOTSTRAP_ARTIFACT, LIVE_BOOTSTRAP_GENERATOR, LIVE_BOOTSTRAP_MAX_MESSAGE_BYTES,
    LIVE_BOOTSTRAP_MAX_VISIBLE_ENTITIES, LIVE_BOOTSTRAP_PROTOCOL_VERSION,
    LIVE_BOOTSTRAP_SCHEMA_DIGEST, LIVE_BOOTSTRAP_SCHEMA_SOURCE,
};

use super::{RequestContext, shapes};
pub(super) use shapes::{error_category, status_category};

pub(super) fn project_response(
    body: &JsonValue,
    context: &RequestContext,
    request: &JsonValue,
) -> Result<(JsonValue, bool), &'static str> {
    let object = body
        .as_object()
        .ok_or("bootstrap response is not an object")?;
    let expected_keys = [
        "protocol_version",
        "schema_digest",
        "provenance",
        "correlation_id",
        "kind",
        "selector",
        "scope",
        "limits",
        "parent_observation",
        "visible_entities",
        "owner_provenance",
        "error",
    ];
    if object.len() != expected_keys.len()
        || expected_keys.iter().any(|key| !object.contains_key(*key))
    {
        return Err("bootstrap envelope has unknown or missing fields");
    }
    if object.get("protocol_version") != Some(&JsonValue::string(LIVE_BOOTSTRAP_PROTOCOL_VERSION))
        || object.get("schema_digest") != Some(&JsonValue::string(LIVE_BOOTSTRAP_SCHEMA_DIGEST))
        || object.get("correlation_id") != Some(&JsonValue::string(context.correlation_id.clone()))
    {
        return Err("bootstrap response identity does not match the pinned artifact");
    }
    let provenance = object
        .get("provenance")
        .and_then(JsonValue::as_object)
        .ok_or("bootstrap provenance is missing")?;
    if provenance.len() != 3
        || provenance.get("artifact") != Some(&JsonValue::string(LIVE_BOOTSTRAP_ARTIFACT))
        || provenance.get("source") != Some(&JsonValue::string(LIVE_BOOTSTRAP_SCHEMA_SOURCE))
        || provenance.get("generator") != Some(&JsonValue::string(LIVE_BOOTSTRAP_GENERATOR))
    {
        return Err("bootstrap provenance does not match the pinned artifact");
    }
    let kind = object
        .get("kind")
        .and_then(JsonValue::as_string)
        .ok_or("bootstrap response kind is missing")?;
    let request_object = request
        .as_object()
        .ok_or("bootstrap request is not an object")?;
    match kind {
        "error_response" => {
            if object.get("limits") != Some(&JsonValue::Null)
                || object.get("parent_observation") != Some(&JsonValue::Null)
                || object.get("visible_entities") != Some(&JsonValue::Null)
                || object.get("owner_provenance") != Some(&JsonValue::Null)
            {
                return Err("bootstrap error response carries successful result data");
            }
            if object.get("selector") != request_object.get("selector")
                || object.get("scope") != request_object.get("scope")
            {
                return Err("bootstrap error selector does not echo the request");
            }
            shapes::validate_error(object.get("error").ok_or("bootstrap error is missing")?)?;
            shapes::bounded(body, LIVE_BOOTSTRAP_MAX_MESSAGE_BYTES)?;
            Ok((body.clone(), true))
        }
        "bootstrap_response" => {
            if object.get("selector") != request_object.get("selector")
                || object.get("scope") != request_object.get("scope")
                || object.get("error") != Some(&JsonValue::Null)
            {
                return Err("bootstrap response does not echo the request scope");
            }
            let response_limits = shapes::validate_limits(
                object.get("limits").ok_or("bootstrap limits are missing")?,
            )?;
            let requested_limits = shapes::validate_limits(
                request_object
                    .get("limits")
                    .ok_or("bootstrap request limits are missing")?,
            )?;
            if response_limits.0 > requested_limits.0
                || response_limits.1 > requested_limits.1
                || response_limits.2 > requested_limits.2
            {
                return Err("bootstrap response expands the requested limits");
            }
            shapes::validate_owner(
                object
                    .get("owner_provenance")
                    .ok_or("bootstrap owner provenance is missing")?,
            )?;
            let parent = object
                .get("parent_observation")
                .ok_or("bootstrap parent observation is missing")?;
            shapes::validate_parent(parent, context)?;
            let entities = object
                .get("visible_entities")
                .and_then(JsonValue::as_array)
                .ok_or("bootstrap visible entities are missing")?;
            if entities.is_empty()
                || entities.len() > LIVE_BOOTSTRAP_MAX_VISIBLE_ENTITIES as usize
                || entities.len() > response_limits.0 as usize
            {
                return Err("bootstrap visible entities exceed their bound");
            }
            let parent_object = parent
                .as_object()
                .ok_or("bootstrap parent observation is malformed")?;
            let parent_instance = parent_object
                .get("instance_ref")
                .ok_or("bootstrap parent instance_ref is missing")?;
            let parent_generation = parent_object
                .get("state_generation")
                .ok_or("bootstrap parent state_generation is missing")?;
            let parent_snapshot = parent_object
                .get("snapshot_ref")
                .and_then(JsonValue::as_object)
                .and_then(|snapshot| snapshot.get("snapshot_id"))
                .ok_or("bootstrap parent snapshot id is missing")?;
            let selector = request_object
                .get("selector")
                .and_then(JsonValue::as_object)
                .ok_or("bootstrap request selector is malformed")?;
            let selector_definition = selector
                .get("definition_ref")
                .ok_or("bootstrap selector definition_ref is missing")?;
            let selector_instance = selector
                .get("instance_ref")
                .ok_or("bootstrap selector instance_ref is missing")?;
            if selector_instance != &JsonValue::Null && selector_instance != parent_instance {
                return Err("bootstrap selector instance does not match the parent");
            }
            let mut identities = BTreeSet::new();
            let mut parent_seen = false;
            for entity in entities {
                shapes::validate_visible_entity(
                    entity,
                    parent_instance,
                    parent_generation,
                    parent_snapshot,
                    selector_definition,
                    selector_instance,
                    &mut identities,
                    context,
                    response_limits.1,
                )?;
                let instance = entity
                    .as_object()
                    .and_then(|object| object.get("instance_ref"))
                    .ok_or("bootstrap visible instance_ref is missing")?;
                if instance == parent_instance {
                    parent_seen = true;
                }
            }
            if !parent_seen {
                return Err("bootstrap visible entities omit the parent observation");
            }
            shapes::bounded(body, response_limits.2 as usize)?;
            Ok((body.clone(), false))
        }
        _ => Err("bootstrap response kind is unsupported"),
    }
}
