// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::json::JsonValue;

use super::RecoveryOperation;
use super::scalars::{valid_digest, valid_uuid, valid_uuid_v4};

const MAX_WIRE_INTEGER: i64 = 9_007_199_254_740_991;
const LOOKUP_REQUEST_FIELDS: [&str; 2] = ["operation", "lookup_scope"];
const RECONCILE_REQUEST_FIELDS: [&str; 3] = ["operation", "strategy", "current_fence"];
const OPERATION_REF_FIELDS: [&str; 3] = ["operation_id", "payload_digest", "original_context"];
const ORIGINAL_CONTEXT_FIELDS: [&str; 7] = [
    "deployment_id",
    "instance_id",
    "instance_incarnation",
    "boot_id",
    "authority_generation",
    "lease_id",
    "lease_epoch",
];
const RESULT_FIELDS: [&str; 3] = ["status", "retryable", "retry_after_seconds"];
const RESULT_STATUSES: [&str; 30] = [
    "BOOT_AUTHORITY_CREATED",
    "BOOT_READY",
    "BOOT_BLOCKED",
    "FENCE_ACCEPTED",
    "FENCE_REJECTED",
    "LEASE_ACTIVE",
    "LEASE_RENEWED",
    "LEASE_REVOKED",
    "INTENT_RECORDED",
    "MAY_HAVE_BEEN_DISPATCHED",
    "ACCEPTED",
    "SETTLED",
    "REJECTED",
    "UNKNOWN",
    "RECONCILED",
    "DUPLICATE",
    "CONFLICT",
    "NOT_FOUND",
    "STALE_BOOT",
    "STALE_INCARNATION",
    "STALE_LEASE",
    "LEASE_EXPIRED",
    "AUTH_REQUIRED",
    "FORBIDDEN",
    "CONTRACT_MISMATCH",
    "PERSISTENCE_UNAVAILABLE",
    "HOST_NOT_READY",
    "BOUNDS_EXCEEDED",
    "INVALID",
    "BUSY",
];
const RECONCILE_STRATEGIES: [&str; 3] = ["reobserve", "receipt_lookup", "quarantine"];
const V4_IDENTITY_FIELDS: [&str; 3] = ["instance_incarnation", "boot_id", "lease_id"];
const POSITIVE_INTEGER_FIELDS: [&str; 2] = ["authority_generation", "lease_epoch"];
const LOOKUP_RESPONSE_FIELDS: [&str; 3] = ["result", "operation", "mutation_authorized"];
const RECONCILE_RESPONSE_FIELDS: [&str; 3] = ["result", "operation", "witness"];

/// Validates the closed request payload the gateway route accepts, so a
/// malformed reference never reaches the sideband.
pub(super) fn validate_request_payload(
    operation: RecoveryOperation,
    payload: Option<&JsonValue>,
) -> Result<(), String> {
    let object = payload
        .and_then(JsonValue::as_object)
        .ok_or("recovery request payload is missing")?;
    match operation {
        RecoveryOperation::Lookup => {
            if !exact(object, &LOOKUP_REQUEST_FIELDS) {
                return Err(String::from(
                    "recovery lookup payload has an unknown or missing field",
                ));
            }
            if object.get("lookup_scope").and_then(JsonValue::as_string) != Some("historical_read")
            {
                return Err(String::from(
                    "recovery lookup_scope is not a historical read",
                ));
            }
        }
        RecoveryOperation::Reconcile => {
            if !exact(object, &RECONCILE_REQUEST_FIELDS) {
                return Err(String::from(
                    "recovery reconcile payload has an unknown or missing field",
                ));
            }
            if !object
                .get("strategy")
                .and_then(JsonValue::as_string)
                .is_some_and(|strategy| RECONCILE_STRATEGIES.contains(&strategy))
            {
                return Err(String::from("recovery reconcile strategy is unsupported"));
            }
            if object
                .get("current_fence")
                .and_then(JsonValue::as_object)
                .is_none()
            {
                return Err(String::from("recovery current fence is missing"));
            }
        }
    }
    validate_operation_ref(object.get("operation"))
}

/// Validates one closed response payload. The frame is surfaced to the caller
/// verbatim, so this is a shape guard rather than a projection.
pub(super) fn validate_response_payload(
    operation: RecoveryOperation,
    payload: Option<&JsonValue>,
) -> Result<(), String> {
    let object = payload
        .and_then(JsonValue::as_object)
        .ok_or("recovery response payload is missing")?;
    match operation {
        RecoveryOperation::Lookup => {
            if !exact(object, &LOOKUP_RESPONSE_FIELDS) {
                return Err(String::from(
                    "recovery lookup payload has an unknown or missing field",
                ));
            }
            if object.get("mutation_authorized") != Some(&JsonValue::Bool(false)) {
                return Err(String::from(
                    "recovery lookup must not authorize a mutation",
                ));
            }
        }
        RecoveryOperation::Reconcile => {
            if !exact(object, &RECONCILE_RESPONSE_FIELDS) {
                return Err(String::from(
                    "recovery reconcile payload has an unknown or missing field",
                ));
            }
        }
    }
    validate_result(object.get("result"))
}

fn validate_result(value: Option<&JsonValue>) -> Result<(), String> {
    let object = value
        .and_then(JsonValue::as_object)
        .ok_or("recovery result is missing")?;
    if !exact(object, &RESULT_FIELDS) {
        return Err(String::from(
            "recovery result has an unknown or missing field",
        ));
    }
    if !object
        .get("status")
        .and_then(JsonValue::as_string)
        .is_some_and(|status| RESULT_STATUSES.contains(&status))
    {
        return Err(String::from("recovery result status is unsupported"));
    }
    if !matches!(object.get("retryable"), Some(JsonValue::Bool(_))) {
        return Err(String::from("recovery result retryable is not boolean"));
    }
    match object.get("retry_after_seconds") {
        Some(JsonValue::Null) => Ok(()),
        Some(JsonValue::Number(value)) if (1..=MAX_WIRE_INTEGER).contains(value) => Ok(()),
        _ => Err(String::from("recovery retry_after_seconds is invalid")),
    }
}

fn validate_operation_ref(value: Option<&JsonValue>) -> Result<(), String> {
    let object = value
        .and_then(JsonValue::as_object)
        .ok_or("recovery operation reference is missing")?;
    if !exact(object, &OPERATION_REF_FIELDS) {
        return Err(String::from(
            "recovery operation reference has an unknown or missing field",
        ));
    }
    if !valid_uuid_v4(string_field(object, "operation_id")?) {
        return Err(String::from("recovery operation ID is not UUIDv4"));
    }
    if !valid_digest(string_field(object, "payload_digest")?) {
        return Err(String::from("recovery payload digest is invalid"));
    }
    validate_original_context(object.get("original_context"))
}

fn validate_original_context(value: Option<&JsonValue>) -> Result<(), String> {
    let object = value
        .and_then(JsonValue::as_object)
        .ok_or("recovery original context is missing")?;
    if !exact(object, &ORIGINAL_CONTEXT_FIELDS) {
        return Err(String::from(
            "recovery original context has an unknown or missing field",
        ));
    }
    let identity = |field: &str| {
        object
            .get(field)
            .and_then(JsonValue::as_string)
            .unwrap_or_default()
    };
    if !valid_uuid(identity("deployment_id")) || !valid_uuid(identity("instance_id")) {
        return Err(String::from(
            "recovery original context deployment or instance identity is invalid",
        ));
    }
    for field in V4_IDENTITY_FIELDS {
        if !valid_uuid_v4(identity(field)) {
            return Err(String::from(
                "recovery original context incarnation, boot, or lease identity is invalid",
            ));
        }
    }
    for field in POSITIVE_INTEGER_FIELDS {
        if !matches!(object.get(field), Some(JsonValue::Number(value))
            if (1..=MAX_WIRE_INTEGER).contains(value))
        {
            return Err(String::from(
                "recovery original context generation or lease epoch is invalid",
            ));
        }
    }
    Ok(())
}

pub(super) fn exact(object: &BTreeMap<String, JsonValue>, expected: &[&str]) -> bool {
    object.len() == expected.len() && expected.iter().all(|key| object.contains_key(*key))
}

pub(super) fn string_field<'a>(
    object: &'a BTreeMap<String, JsonValue>,
    field: &str,
) -> Result<&'a str, String> {
    object
        .get(field)
        .and_then(JsonValue::as_string)
        .ok_or_else(|| format!("recovery field {field} is not a string"))
}
