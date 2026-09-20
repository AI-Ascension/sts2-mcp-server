// SPDX-License-Identifier: MIT

//! The closed nine-field recovery frame envelope and its actor/auth pair.

use std::collections::BTreeMap;

use crate::json::JsonValue;

use super::scalars::{valid_timestamp, valid_uuid, valid_uuid_v4};
use super::validation::{exact, string_field};
use super::{RECOVERY_CONTRACT, RECOVERY_SCHEMA_DIGEST};

/// Borrows one nested object member of the frame.
pub(super) fn field_object<'a>(
    frame: &'a JsonValue,
    field: &str,
) -> Option<&'a BTreeMap<String, JsonValue>> {
    frame.as_object()?.get(field)?.as_object()
}

const MAX_AUTH_PROOF_BYTES: usize = 512;
const FRAME_FIELDS: [&str; 9] = [
    "contract",
    "schema_digest",
    "message_id",
    "correlation_id",
    "sent_at",
    "actor",
    "auth",
    "kind",
    "payload",
];
const ACTOR_FIELDS: [&str; 2] = ["principal_id", "role"];
const AUTH_FIELDS: [&str; 3] = ["principal_id", "capability", "proof"];
const ACTOR_ROLES: [&str; 6] = ["gateway", "watchdog", "harness", "host", "mod", "operator"];

/// Validates the nine closed frame fields, the actor/auth pair, the two closed
/// identifiers, and the timestamp. Returns the frame correlation.
pub(super) fn validate_envelope(
    frame: &JsonValue,
    kind: &str,
    capability: &str,
) -> Result<String, String> {
    let object = frame.as_object().ok_or("recovery frame is not an object")?;
    if !exact(object, &FRAME_FIELDS) {
        return Err(String::from(
            "recovery frame has an unknown or missing field",
        ));
    }
    if object.get("contract").and_then(JsonValue::as_string) != Some(RECOVERY_CONTRACT)
        || object.get("schema_digest").and_then(JsonValue::as_string)
            != Some(RECOVERY_SCHEMA_DIGEST)
        || object.get("kind").and_then(JsonValue::as_string) != Some(kind)
    {
        return Err(String::from(
            "recovery frame contract, digest, or kind is not the expected closed value",
        ));
    }
    if !valid_uuid_v4(string_field(object, "message_id")?) {
        return Err(String::from("recovery message_id is not UUIDv4"));
    }
    let correlation_id = string_field(object, "correlation_id")?;
    if !valid_uuid_v4(correlation_id) {
        return Err(String::from("recovery correlation_id is not UUIDv4"));
    }
    if !valid_timestamp(string_field(object, "sent_at")?) {
        return Err(String::from(
            "recovery sent_at is not a strict UTC timestamp",
        ));
    }
    validate_actor(object, capability)?;
    Ok(correlation_id.to_owned())
}

fn validate_actor(object: &BTreeMap<String, JsonValue>, capability: &str) -> Result<(), String> {
    let actor = object
        .get("actor")
        .and_then(JsonValue::as_object)
        .ok_or("recovery actor is missing")?;
    let auth = object
        .get("auth")
        .and_then(JsonValue::as_object)
        .ok_or("recovery auth is missing")?;
    if !exact(actor, &ACTOR_FIELDS) || !exact(auth, &AUTH_FIELDS) {
        return Err(String::from(
            "recovery actor or auth has an unknown or missing field",
        ));
    }
    let principal = actor
        .get("principal_id")
        .and_then(JsonValue::as_string)
        .ok_or("recovery actor principal is missing")?;
    if !valid_uuid(principal)
        || auth.get("principal_id").and_then(JsonValue::as_string) != Some(principal)
    {
        return Err(String::from(
            "recovery actor and auth are not one UUID principal",
        ));
    }
    let role = actor
        .get("role")
        .and_then(JsonValue::as_string)
        .ok_or("recovery actor role is missing")?;
    if !ACTOR_ROLES.contains(&role) {
        return Err(String::from("recovery actor role is unsupported"));
    }
    if auth.get("capability").and_then(JsonValue::as_string) != Some(capability) {
        return Err(String::from(
            "recovery auth capability does not match the operation",
        ));
    }
    match auth.get("proof") {
        Some(JsonValue::Null) => Ok(()),
        Some(JsonValue::String(proof))
            if !proof.is_empty() && proof.len() <= MAX_AUTH_PROOF_BYTES =>
        {
            Ok(())
        }
        _ => Err(String::from("recovery auth proof is invalid")),
    }
}
