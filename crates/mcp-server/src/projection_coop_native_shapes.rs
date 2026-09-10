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
const RECEIPT_FIELDS: [&str; 10] = [
    "operation_id",
    "status",
    "before_host_generation",
    "after_host_generation",
    "authority_id",
    "authority_epoch",
    "checkpoint_id",
    "state_digest",
    "native_checksum",
    "error_code",
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
        "catalog",
        "receipt",
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
        "catalog",
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
    let receipt = object
        .get("receipt")
        .ok_or("native effect receipt is missing")?;
    let receipt_status = validate_receipt(receipt, operation_id)?;
    if receipt_status != status {
        return Err("native effect receipt status does not match its response");
    }
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
        "catalog",
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
    if status.is_some() {
        let receipt = object
            .get("receipt")
            .ok_or("native recovery receipt is missing")?;
        let receipt_status = validate_receipt(
            receipt,
            object
                .get("operation_id")
                .and_then(JsonValue::as_string)
                .ok_or("native recovery operation_id is missing or unsafe")?,
        )?;
        if let Some(expected_status) = status {
            match expected_status {
                "settled" | "rejected" if receipt_status != expected_status => {
                    return Err("native recovery receipt status does not match its response");
                }
                "unknown" if !matches!(receipt_status, "accepted" | "unknown") => {
                    return Err("native recovery receipt status is unsupported");
                }
                _ => {}
            }
        }
    } else if object.get("receipt") != Some(&JsonValue::Null) {
        return Err("pending native recovery must not carry a receipt");
    }
    Ok(())
}

fn validate_receipt<'a>(value: &'a JsonValue, operation_id: &str) -> Result<&'a str, &'static str> {
    let object = exact_object(value, &RECEIPT_FIELDS, "native receipt")?;
    if object
        .get("operation_id")
        .and_then(JsonValue::as_string)
        .filter(|value| safe_identity(value))
        != Some(operation_id)
    {
        return Err("native receipt operation_id is invalid or mismatched");
    }
    let status = object
        .get("status")
        .and_then(JsonValue::as_string)
        .filter(|value| matches!(*value, "accepted" | "settled" | "rejected" | "unknown"))
        .ok_or("native receipt status is unsupported")?;
    generation(object.get("before_host_generation"))?;
    nullable_generation(object.get("after_host_generation"))?;
    for field in ["authority_id", "authority_epoch", "checkpoint_id"] {
        if !object
            .get(field)
            .and_then(JsonValue::as_string)
            .is_some_and(safe_identity)
        {
            return Err("native receipt identity is invalid");
        }
    }
    digest(object.get("state_digest"))?;
    nullable_digest(object.get("native_checksum"))?;
    nullable_identity(object.get("error_code"))?;
    Ok(status)
}

fn nullable_generation(value: Option<&JsonValue>) -> Result<(), &'static str> {
    match value {
        Some(JsonValue::Null) => Ok(()),
        Some(value) => generation(Some(value)).map(|_| ()),
        None => Err("native nullable generation is missing"),
    }
}

fn nullable_identity(value: Option<&JsonValue>) -> Result<(), &'static str> {
    match value {
        Some(JsonValue::Null) => Ok(()),
        Some(JsonValue::String(value)) if safe_identity(value) => Ok(()),
        _ => Err("native nullable identity is invalid"),
    }
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
