// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::json::JsonValue;

use super::exact_object;
use super::observation;
use super::observation::{digest, generation, nullable_digest, safe_identity};

const EFFECT_FIELDS: [&str; 10] = [
    "effect_id",
    "operation_id",
    "kind",
    "from_generation",
    "to_generation",
    "state_digest",
    "authority_epoch",
    "checkpoint_id",
    "authority_id",
    "native_checksum",
];
pub(super) fn observation_response(
    object: &BTreeMap<String, JsonValue>,
) -> Result<(), &'static str> {
    for field in [
        "operation_id",
        "actor_peer",
        "expected_host_generation",
        "action",
        "vote",
        "status",
        "effect",
        "recovery",
    ] {
        if object.get(field) != Some(&JsonValue::Null) {
            return Err("native observation has an unexpected response member");
        }
    }
    observation::validate_observation(
        object
            .get("observation")
            .ok_or("native observation is missing")?,
    )
}

pub(super) fn effect_response(object: &BTreeMap<String, JsonValue>) -> Result<(), &'static str> {
    let operation_id = object
        .get("operation_id")
        .and_then(JsonValue::as_string)
        .filter(|value| safe_identity(value))
        .ok_or("native effect operation_id is missing or unsafe")?;
    for field in [
        "actor_peer",
        "expected_host_generation",
        "action",
        "vote",
        "recovery",
    ] {
        if object.get(field) != Some(&JsonValue::Null) {
            return Err("native effect has an unexpected response member");
        }
    }
    let status = object
        .get("status")
        .and_then(JsonValue::as_string)
        .filter(|value| matches!(*value, "accepted" | "settled" | "rejected" | "unknown"))
        .ok_or("native effect status is unsupported")?;
    observation::validate_observation(
        object
            .get("observation")
            .ok_or("native effect observation is missing")?,
    )?;
    match status {
        "settled" => {
            let effect = object
                .get("effect")
                .ok_or("settled native effect is missing")?;
            validate_effect(effect, operation_id)?;
        }
        "accepted" | "rejected" | "unknown" => {
            if object.get("effect") != Some(&JsonValue::Null) {
                return Err("non-settled native effect must not carry an effect");
            }
        }
        _ => unreachable!(),
    }
    Ok(())
}

pub(super) fn recovery_response(object: &BTreeMap<String, JsonValue>) -> Result<(), &'static str> {
    object
        .get("operation_id")
        .and_then(JsonValue::as_string)
        .filter(|value| safe_identity(value))
        .ok_or("native recovery operation_id is missing or unsafe")?;
    for field in [
        "actor_peer",
        "expected_host_generation",
        "action",
        "vote",
        "effect",
    ] {
        if object.get(field) != Some(&JsonValue::Null) {
            return Err("native recovery has an unexpected response member");
        }
    }
    let recovery = validate_recovery(
        object
            .get("recovery")
            .ok_or("native recovery member is missing")?,
    )?;
    let status = object.get("status").and_then(JsonValue::as_string);
    match status {
        None => {
            if recovery != "reconcile" || object.get("observation") != Some(&JsonValue::Null) {
                return Err("pending native recovery must be a reconcile request");
            }
        }
        Some("settled" | "rejected") => {
            if recovery != "reconcile" {
                return Err("settled or rejected native recovery must reconcile");
            }
            observation::validate_observation(
                object
                    .get("observation")
                    .ok_or("settled native recovery observation is missing")?,
            )?;
        }
        Some("unknown") => {
            observation::validate_observation(
                object
                    .get("observation")
                    .ok_or("unknown native recovery observation is missing")?,
            )?;
        }
        Some(_) => return Err("native recovery status is unsupported"),
    }
    Ok(())
}
fn validate_effect(value: &JsonValue, operation_id: &str) -> Result<(), &'static str> {
    let object = exact_object(value, &EFFECT_FIELDS, "native effect")?;
    let effect_operation = object
        .get("operation_id")
        .and_then(JsonValue::as_string)
        .filter(|value| safe_identity(value))
        .ok_or("native effect operation_id is invalid")?;
    if effect_operation != operation_id {
        return Err("native effect operation_id does not match its response");
    }
    if !object
        .get("effect_id")
        .and_then(JsonValue::as_string)
        .is_some_and(safe_identity)
        || !object
            .get("authority_epoch")
            .and_then(JsonValue::as_string)
            .is_some_and(safe_identity)
        || !object
            .get("checkpoint_id")
            .and_then(JsonValue::as_string)
            .is_some_and(safe_identity)
        || !object
            .get("authority_id")
            .and_then(JsonValue::as_string)
            .is_some_and(safe_identity)
    {
        return Err("native effect identity is invalid");
    }
    if !matches!(
        object.get("kind").and_then(JsonValue::as_string),
        Some(
            "turn_ended"
                | "shared_event_vote"
                | "native_end_turn_settled"
                | "native_play_card_settled"
                | "native_shared_event_vote_settled"
                | "native_treasure_relic_vote_settled"
        )
    ) {
        return Err("native effect kind is unsupported");
    }
    let from = generation(object.get("from_generation"))?;
    let to = generation(object.get("to_generation"))?;
    if to < from {
        return Err("native effect generation regressed");
    }
    digest(object.get("state_digest"))?;
    nullable_digest(object.get("native_checksum"))
}

fn validate_recovery(value: &JsonValue) -> Result<&'static str, &'static str> {
    let object = exact_object(value, &["kind", "rejoin_epoch"], "native recovery")?;
    let kind = object
        .get("kind")
        .and_then(JsonValue::as_string)
        .filter(|value| matches!(*value, "reconcile" | "rejoin"))
        .ok_or("native recovery kind is unsupported")?;
    generation(object.get("rejoin_epoch"))?;
    Ok(if kind == "reconcile" {
        "reconcile"
    } else {
        "rejoin"
    })
}
