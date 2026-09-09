// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::json::JsonValue;
use crate::protocol_artifact_coop_receipt_query::{
    COOP_RECEIPT_QUERY_ARTIFACT, COOP_RECEIPT_QUERY_GENERATOR, COOP_RECEIPT_QUERY_MAX_GENERATION,
    COOP_RECEIPT_QUERY_PROTOCOL_VERSION, COOP_RECEIPT_QUERY_SCHEMA_DIGEST,
    COOP_RECEIPT_QUERY_SCHEMA_SOURCE,
};

use super::Context;

const TOP_LEVEL_FIELDS: [&str; 24] = [
    "protocol_version",
    "schema_digest",
    "provenance",
    "correlation_id",
    "instance_id",
    "session_id",
    "lease_id",
    "lease_epoch",
    "kind",
    "operation_id",
    "action_kind",
    "action_fingerprint",
    "run_id",
    "location",
    "actor_id",
    "authority_id",
    "authority_epoch",
    "expected_host_generation",
    "before_host_generation",
    "participant_ids",
    "status",
    "evidence_scope",
    "receipt",
    "error_code",
];
const RECEIPT_FIELDS: [&str; 7] = [
    "status",
    "after_host_generation",
    "checkpoint_id",
    "state_digest",
    "effect_id",
    "effect_kind",
    "error_code",
];

pub(super) fn project_coop_receipt_query_response(
    body: &JsonValue,
    context: &Context,
    status_code: u16,
) -> Result<(JsonValue, bool), &'static str> {
    let object = exact_object(body, &TOP_LEVEL_FIELDS, "receipt-query response")?;
    validate_metadata(object, context)?;
    validate_identity(object, context)?;
    let status = object
        .get("status")
        .and_then(JsonValue::as_string)
        .ok_or("receipt-query response status is missing")?;
    if !status_code_matches(status, status_code) {
        return Err("receipt-query HTTP status does not match response status");
    }
    validate_semantics(object, context, status)?;
    let is_error = matches!(status, "rejected" | "unknown" | "recovery_required")
        || !(200..300).contains(&status_code);
    Ok((body.clone(), is_error))
}

fn validate_metadata(
    object: &BTreeMap<String, JsonValue>,
    context: &Context,
) -> Result<(), &'static str> {
    for (field, expected) in [
        ("protocol_version", COOP_RECEIPT_QUERY_PROTOCOL_VERSION),
        ("schema_digest", COOP_RECEIPT_QUERY_SCHEMA_DIGEST),
        ("correlation_id", context.correlation.as_str()),
        ("instance_id", context.instance.as_str()),
        ("session_id", context.session.as_str()),
        ("lease_id", context.lease.as_str()),
        ("kind", "receipt_query_response"),
        ("evidence_scope", "retained_receipt"),
    ] {
        if object.get(field).and_then(JsonValue::as_string) != Some(expected) {
            return Err("receipt-query response metadata or identity mismatched");
        }
    }
    if object.get("lease_epoch") != Some(&JsonValue::Number(context.epoch)) {
        return Err("receipt-query response lease epoch mismatched");
    }
    let provenance = exact_object(
        object
            .get("provenance")
            .ok_or("receipt-query response provenance is missing")?,
        &["artifact", "source", "generator"],
        "receipt-query response provenance",
    )?;
    for (field, expected) in [
        ("artifact", COOP_RECEIPT_QUERY_ARTIFACT),
        ("source", COOP_RECEIPT_QUERY_SCHEMA_SOURCE),
        ("generator", COOP_RECEIPT_QUERY_GENERATOR),
    ] {
        if provenance.get(field).and_then(JsonValue::as_string) != Some(expected) {
            return Err("receipt-query response provenance is unsupported");
        }
    }
    Ok(())
}

fn validate_identity(
    object: &BTreeMap<String, JsonValue>,
    context: &Context,
) -> Result<(), &'static str> {
    for (field, expected) in [
        ("operation_id", context.operation_id.as_str()),
        ("action_kind", context.action_kind.as_str()),
        ("action_fingerprint", context.action_fingerprint.as_str()),
        ("run_id", context.run_id.as_str()),
        ("actor_id", context.actor_id.as_str()),
        ("authority_id", context.authority_id.as_str()),
        ("authority_epoch", context.authority_epoch.as_str()),
    ] {
        if object.get(field).and_then(JsonValue::as_string) != Some(expected) {
            return Err("receipt-query response immutable identity mismatched");
        }
    }
    if object.get("location") != Some(&context.location)
        || object.get("expected_host_generation")
            != Some(&JsonValue::Number(context.expected_host_generation))
        || object.get("before_host_generation")
            != Some(&JsonValue::Number(context.before_host_generation))
        || object.get("participant_ids")
            != Some(&JsonValue::Array(
                context
                    .participant_ids
                    .iter()
                    .map(|value| JsonValue::string(value.as_str()))
                    .collect(),
            ))
    {
        return Err("receipt-query response immutable identity mismatched");
    }
    Ok(())
}

fn validate_semantics(
    object: &BTreeMap<String, JsonValue>,
    context: &Context,
    status: &str,
) -> Result<(), &'static str> {
    match status {
        "accepted" => accepted(object),
        "settled" => settled(object, context),
        "rejected" => rejected(object),
        "unknown" | "recovery_required" => {
            if object.get("receipt") != Some(&JsonValue::Null)
                || !object
                    .get("error_code")
                    .and_then(JsonValue::as_string)
                    .is_some_and(safe_identity)
            {
                Err("receipt-query uncertainty must not carry a receipt")
            } else {
                Ok(())
            }
        }
        _ => Err("receipt-query response status is unsupported"),
    }
}

fn accepted(object: &BTreeMap<String, JsonValue>) -> Result<(), &'static str> {
    if object.get("error_code") != Some(&JsonValue::Null) {
        return Err("accepted receipt-query response must not carry an error");
    }
    let receipt = exact_object(
        object
            .get("receipt")
            .ok_or("accepted receipt-query response receipt is missing")?,
        &RECEIPT_FIELDS,
        "accepted receipt-query receipt",
    )?;
    if receipt.get("status").and_then(JsonValue::as_string) != Some("accepted")
        || receipt.get("after_host_generation") != Some(&JsonValue::Null)
        || receipt.get("checkpoint_id") != Some(&JsonValue::Null)
        || receipt.get("state_digest") != Some(&JsonValue::Null)
        || receipt.get("effect_id") != Some(&JsonValue::Null)
        || receipt.get("effect_kind") != Some(&JsonValue::Null)
        || receipt.get("error_code") != Some(&JsonValue::Null)
    {
        return Err("accepted receipt-query receipt is malformed");
    }
    Ok(())
}

fn settled(object: &BTreeMap<String, JsonValue>, context: &Context) -> Result<(), &'static str> {
    if object.get("error_code") != Some(&JsonValue::Null) {
        return Err("settled receipt-query response must not carry an error");
    }
    let receipt = exact_object(
        object
            .get("receipt")
            .ok_or("settled receipt-query response receipt is missing")?,
        &RECEIPT_FIELDS,
        "settled receipt-query receipt",
    )?;
    let after = match receipt.get("after_host_generation") {
        Some(JsonValue::Number(value))
            if (0..=COOP_RECEIPT_QUERY_MAX_GENERATION).contains(value) =>
        {
            *value
        }
        _ => return Err("settled receipt-query generation is invalid"),
    };
    if receipt.get("status").and_then(JsonValue::as_string) != Some("settled")
        || after <= context.before_host_generation
        || receipt
            .get("checkpoint_id")
            .and_then(JsonValue::as_string)
            .is_none_or(|value| !safe_identity(value))
        || receipt
            .get("state_digest")
            .and_then(JsonValue::as_string)
            .is_none_or(|value| !lower_hex_digest(value))
        || receipt.get("effect_id").and_then(JsonValue::as_string)
            != Some(format!("effect:{}", context.operation_id).as_str())
        || receipt.get("effect_kind").and_then(JsonValue::as_string)
            != Some(format!("{}_settled", context.action_kind).as_str())
        || receipt.get("error_code") != Some(&JsonValue::Null)
    {
        return Err("settled receipt-query receipt is inconsistent with its identity");
    }
    Ok(())
}

fn rejected(object: &BTreeMap<String, JsonValue>) -> Result<(), &'static str> {
    let error_code = object
        .get("error_code")
        .and_then(JsonValue::as_string)
        .filter(|value| safe_identity(value))
        .ok_or("rejected receipt-query response requires an error code")?;
    let receipt = exact_object(
        object
            .get("receipt")
            .ok_or("rejected receipt-query response receipt is missing")?,
        &RECEIPT_FIELDS,
        "rejected receipt-query receipt",
    )?;
    if receipt.get("status").and_then(JsonValue::as_string) != Some("rejected")
        || receipt.get("after_host_generation") != Some(&JsonValue::Null)
        || receipt.get("checkpoint_id") != Some(&JsonValue::Null)
        || receipt.get("state_digest") != Some(&JsonValue::Null)
        || receipt.get("effect_id") != Some(&JsonValue::Null)
        || receipt.get("effect_kind") != Some(&JsonValue::Null)
        || receipt.get("error_code").and_then(JsonValue::as_string) != Some(error_code)
    {
        return Err("rejected receipt-query receipt is malformed");
    }
    Ok(())
}

fn status_code_matches(status: &str, status_code: u16) -> bool {
    match status {
        "accepted" | "settled" => status_code == 200,
        "rejected" => status_code == 409,
        "unknown" => status_code == 503,
        "recovery_required" => matches!(status_code, 409 | 503),
        _ => false,
    }
}

fn exact_object<'a>(
    value: &'a JsonValue,
    fields: &[&str],
    label: &'static str,
) -> Result<&'a BTreeMap<String, JsonValue>, &'static str> {
    let object = value.as_object().ok_or(label)?;
    if object.len() != fields.len() || fields.iter().any(|field| !object.contains_key(*field)) {
        return Err(label);
    }
    Ok(object)
}

fn safe_identity(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && !value.contains("..")
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':' | b'/')
        })
}

fn lower_hex_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
