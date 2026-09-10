// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::json::JsonValue;

use super::observation;

pub(super) fn effect_relations(
    object: &BTreeMap<String, JsonValue>,
    expected_generation: Option<i64>,
) -> Result<(), &'static str> {
    let observation = object
        .get("observation")
        .and_then(JsonValue::as_object)
        .ok_or("native effect observation is missing")?;
    let receipt = object
        .get("receipt")
        .and_then(JsonValue::as_object)
        .ok_or("native effect receipt is missing")?;
    let operation_id = object
        .get("operation_id")
        .and_then(JsonValue::as_string)
        .ok_or("native effect operation_id is missing")?;
    if receipt.get("operation_id").and_then(JsonValue::as_string) != Some(operation_id)
        || receipt.get("status") != object.get("status")
    {
        return Err("native effect receipt identity or status mismatched");
    }
    receipt_observation_relations(observation, receipt)?;
    let observation_generation = observation::generation(observation.get("host_generation"))?;
    let status = object
        .get("status")
        .and_then(JsonValue::as_string)
        .ok_or("native effect status is missing")?;
    let before = observation::generation(receipt.get("before_host_generation"))?;
    if expected_generation.is_some_and(|expected| expected != before) {
        return Err("native effect receipt generation mismatched the request");
    }
    match status {
        "settled" => {
            let after = observation::generation(receipt.get("after_host_generation"))?;
            if after != observation_generation || before >= after {
                return Err("settled native effect generations are inconsistent");
            }
            let effect = object
                .get("effect")
                .and_then(JsonValue::as_object)
                .ok_or("settled native effect is missing")?;
            if effect.get("operation_id").and_then(JsonValue::as_string) != Some(operation_id)
                || effect.get("from_generation") != Some(&JsonValue::Number(before))
                || effect.get("to_generation") != Some(&JsonValue::Number(after))
                || effect.get("state_digest") != observation.get("state_digest")
                || effect.get("authority_id") != observation.get("authority_id")
                || effect.get("authority_epoch") != observation.get("host_authority_epoch")
                || effect.get("checkpoint_id") != observation.get("checkpoint_id")
            {
                return Err("native effect witness does not match its observation");
            }
        }
        "accepted" | "rejected" | "unknown" => {
            if receipt.get("after_host_generation") != Some(&JsonValue::Null)
                || before != observation_generation
            {
                return Err("non-settled native effect generations are inconsistent");
            }
        }
        _ => return Err("native effect status is unsupported"),
    }
    Ok(())
}

pub(super) fn recovery_relations(
    object: &BTreeMap<String, JsonValue>,
    expected_generation: Option<i64>,
    expected_recovery_kind: Option<&str>,
) -> Result<(), &'static str> {
    let operation_id = object
        .get("operation_id")
        .and_then(JsonValue::as_string)
        .ok_or("native recovery operation_id is missing")?;
    let recovery = object
        .get("recovery")
        .and_then(JsonValue::as_object)
        .ok_or("native recovery member is missing")?;
    let recovery_kind = recovery
        .get("kind")
        .and_then(JsonValue::as_string)
        .ok_or("native recovery kind is missing")?;
    let Some(status) = object.get("status").and_then(JsonValue::as_string) else {
        return Err("native recovery response is missing its outcome status");
    };
    let observation = object
        .get("observation")
        .and_then(JsonValue::as_object)
        .ok_or("native recovery observation is missing")?;
    let receipt = object
        .get("receipt")
        .and_then(JsonValue::as_object)
        .ok_or("native recovery receipt is missing")?;
    if receipt.get("operation_id").and_then(JsonValue::as_string) != Some(operation_id)
        || receipt_observation_relations(observation, receipt).is_err()
    {
        return Err("native recovery receipt does not match its observation");
    }
    let observation_generation = observation::generation(observation.get("host_generation"))?;
    let before = observation::generation(receipt.get("before_host_generation"))?;
    if expected_generation.is_some_and(|expected| expected != before) {
        return Err("native recovery receipt generation mismatched the request");
    }
    match status {
        "unknown" => {
            if let Some(expected) = expected_recovery_kind {
                let valid = match expected {
                    "rejoin" => recovery_kind == "rejoin",
                    "reconcile" => recovery_kind == "reconcile",
                    _ => false,
                };
                if !valid {
                    return Err("unknown native recovery response has the wrong route kind");
                }
            }
            let receipt_status = receipt.get("status").and_then(JsonValue::as_string);
            let after_is_valid = match receipt_status {
                // The canonical pending-rejoin witness records admission at
                // the current host generation. An unresolved receipt remains
                // explicitly after-null, including an accepted recover
                // response on the reconcile route.
                Some("accepted") if expected_recovery_kind == Some("rejoin") => {
                    receipt.get("after_host_generation")
                        == Some(&JsonValue::Number(observation_generation))
                }
                Some("accepted" | "unknown") => {
                    receipt.get("after_host_generation") == Some(&JsonValue::Null)
                }
                _ => false,
            };
            if !after_is_valid || before != observation_generation {
                return Err("unknown native recovery generations are inconsistent");
            }
        }
        "settled" => {
            if expected_recovery_kind == Some("rejoin") && recovery_kind != "reconcile" {
                return Err("settled native rejoin response must reconcile");
            }
            if expected_recovery_kind == Some("reconcile") && recovery_kind != "reconcile" {
                return Err("settled native reconcile response has the wrong route kind");
            }
            if recovery_kind != "reconcile"
                || receipt.get("status").and_then(JsonValue::as_string) != Some("settled")
                || receipt.get("after_host_generation")
                    != Some(&JsonValue::Number(observation_generation))
            {
                return Err("settled native recovery is inconsistent");
            }
        }
        "rejected" => {
            if expected_recovery_kind == Some("rejoin") && recovery_kind != "reconcile" {
                return Err("rejected native rejoin response must reconcile");
            }
            if expected_recovery_kind == Some("reconcile") && recovery_kind != "reconcile" {
                return Err("rejected native reconcile response has the wrong route kind");
            }
            if recovery_kind != "reconcile"
                || receipt.get("status").and_then(JsonValue::as_string) != Some("rejected")
                || receipt.get("after_host_generation") != Some(&JsonValue::Null)
            {
                return Err("rejected native recovery is inconsistent");
            }
        }
        _ => return Err("native recovery status is unsupported"),
    }
    Ok(())
}

fn receipt_observation_relations(
    observation: &BTreeMap<String, JsonValue>,
    receipt: &BTreeMap<String, JsonValue>,
) -> Result<(), &'static str> {
    for (receipt_field, observation_field) in [
        ("state_digest", "state_digest"),
        ("authority_id", "authority_id"),
        ("authority_epoch", "host_authority_epoch"),
        ("checkpoint_id", "checkpoint_id"),
    ] {
        if receipt.get(receipt_field) != observation.get(observation_field) {
            return Err("native receipt identity does not match its observation");
        }
    }
    Ok(())
}
