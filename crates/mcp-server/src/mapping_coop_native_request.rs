// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::json::JsonValue;
use crate::mapping::{safe_header_value, safe_segment};
use crate::protocol_artifact_coop_native::COOP_NATIVE_MAX_GENERATION;

pub(super) fn identity<'a>(
    arguments: &'a BTreeMap<String, JsonValue>,
    key: &str,
    instance: bool,
) -> Result<&'a str, &'static str> {
    let value = arguments
        .get(key)
        .and_then(JsonValue::as_string)
        .filter(|value| safe_header_value(value))
        .ok_or("native co-op identity is missing, unsafe, or oversized")?;
    if instance && !safe_segment(value) {
        return Err("native instance_id is not a safe path segment");
    }
    Ok(value)
}

pub(super) fn operation_id<'a>(
    arguments: &'a BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<&'a str, &'static str> {
    let value = arguments
        .get(key)
        .and_then(JsonValue::as_string)
        .filter(|value| body_identity(value))
        .ok_or("native operation_id is missing, unsafe, or oversized")?;
    Ok(value)
}

pub(super) fn peer<'a>(
    arguments: &'a BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<&'a str, &'static str> {
    arguments
        .get(key)
        .and_then(JsonValue::as_string)
        .filter(|value| peer_identity(value))
        .ok_or("native peer identity is missing or unsafe")
}

pub(super) fn generation(
    arguments: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<i64, &'static str> {
    match arguments.get(key) {
        Some(JsonValue::Number(value)) if (0..=COOP_NATIVE_MAX_GENERATION).contains(value) => {
            Ok(*value)
        }
        _ => Err("native generation or lease_epoch is outside the protocol bound"),
    }
}

pub(super) fn action(value: Option<&JsonValue>) -> Result<JsonValue, &'static str> {
    let object = exact_object(
        value.ok_or("native action is missing")?,
        &["kind", "action_id", "target_peer"],
    )?;
    let kind = object
        .get("kind")
        .and_then(JsonValue::as_string)
        .filter(|value| {
            matches!(
                *value,
                "play_card" | "end_turn" | "select_card" | "choose_reward" | "confirm_selection"
            )
        })
        .ok_or("native action kind is unsupported")?;
    let action_id = object
        .get("action_id")
        .and_then(JsonValue::as_string)
        .filter(|value| body_identity(value))
        .ok_or("native action_id is missing or unsafe")?;
    let target_peer = match object.get("target_peer") {
        Some(JsonValue::Null) => JsonValue::Null,
        Some(JsonValue::String(value)) if peer_identity(value) => JsonValue::string(value),
        _ => return Err("native action target_peer is invalid"),
    };
    Ok(JsonValue::object([
        (String::from("kind"), JsonValue::string(kind)),
        (String::from("action_id"), JsonValue::string(action_id)),
        (String::from("target_peer"), target_peer),
    ]))
}

pub(super) fn vote(value: Option<&JsonValue>) -> Result<JsonValue, &'static str> {
    let object = exact_object(
        value.ok_or("native vote is missing")?,
        &["proposal_id", "voter_peer", "choice"],
    )?;
    let proposal_id = object
        .get("proposal_id")
        .and_then(JsonValue::as_string)
        .filter(|value| body_identity(value))
        .ok_or("native proposal_id is missing or unsafe")?;
    let voter_peer = object
        .get("voter_peer")
        .and_then(JsonValue::as_string)
        .filter(|value| peer_identity(value))
        .ok_or("native voter_peer is missing or unsafe")?;
    let choice = object
        .get("choice")
        .and_then(JsonValue::as_string)
        .filter(|value| body_identity(value))
        .ok_or("native vote choice is missing or unsafe")?;
    Ok(JsonValue::object([
        (String::from("proposal_id"), JsonValue::string(proposal_id)),
        (String::from("voter_peer"), JsonValue::string(voter_peer)),
        (String::from("choice"), JsonValue::string(choice)),
    ]))
}

pub(super) fn recovery(
    value: Option<&JsonValue>,
    expected: &str,
) -> Result<JsonValue, &'static str> {
    let object = exact_object(
        value.ok_or("native recovery is missing")?,
        &["kind", "rejoin_epoch"],
    )?;
    if object.get("kind").and_then(JsonValue::as_string) != Some(expected) {
        return Err("native recovery kind is not valid for this tool");
    }
    let epoch = match object.get("rejoin_epoch") {
        Some(JsonValue::Number(value)) if (0..=COOP_NATIVE_MAX_GENERATION).contains(value) => {
            *value
        }
        _ => return Err("native recovery epoch is outside the protocol bound"),
    };
    Ok(JsonValue::object([
        (String::from("kind"), JsonValue::string(expected)),
        (String::from("rejoin_epoch"), JsonValue::Number(epoch)),
    ]))
}

fn exact_object<'a>(
    value: &'a JsonValue,
    fields: &[&str],
) -> Result<&'a BTreeMap<String, JsonValue>, &'static str> {
    let object = value
        .as_object()
        .ok_or("native nested value must be an object")?;
    if object.len() != fields.len() || fields.iter().any(|field| !object.contains_key(*field)) {
        return Err("native nested value has unknown or missing fields");
    }
    Ok(object)
}

pub(super) fn peer_identity(value: &str) -> bool {
    value
        .strip_prefix("peer:")
        .is_some_and(|suffix| (5..=507).contains(&suffix.len()) && body_identity(value))
}

// These values are carried only in the closed native envelope. They must not
// inherit the shorter HTTP header limit used for configured instance, session,
// lease, and correlation identities.
fn body_identity(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 512
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':' | b'/')
        })
}
