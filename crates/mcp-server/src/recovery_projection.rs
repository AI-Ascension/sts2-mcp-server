// SPDX-License-Identifier: MIT

//! Safe MCP projection for recovery responses and transport-loss uncertainty.

use crate::json::JsonValue;
use crate::protocol_artifact_recovery::RECOVERY_SCHEMA_DIGEST;

use super::recovery_canonical::string;

pub(crate) fn project_response(
    body: &JsonValue,
    kind: &str,
    correlation: &str,
) -> Result<JsonValue, &'static str> {
    super::recovery_validation::validate_response(body, kind, correlation)?;
    let object = body
        .as_object()
        .ok_or("recovery response must be an object")?;
    let mut payload = object
        .get("payload")
        .cloned()
        .ok_or("recovery payload missing")?;
    redact_secrets(&mut payload);
    Ok(JsonValue::object([
        (
            "contract".to_owned(),
            JsonValue::string("watchdog-recovery-v1"),
        ),
        (
            "schema_digest".to_owned(),
            JsonValue::string(RECOVERY_SCHEMA_DIGEST),
        ),
        (
            "message_id".to_owned(),
            JsonValue::string(string(object, "message_id")?),
        ),
        ("correlation_id".to_owned(), JsonValue::string(correlation)),
        (
            "sent_at".to_owned(),
            JsonValue::string(string(object, "sent_at")?),
        ),
        (
            "kind".to_owned(),
            JsonValue::string(format!("{kind}_response")),
        ),
        ("payload".to_owned(), payload),
    ]))
}

pub(crate) fn is_error(body: &JsonValue, status: u16) -> bool {
    if !(200..300).contains(&status) {
        return true;
    }
    let Some(payload) = body
        .as_object()
        .and_then(|o| o.get("payload"))
        .and_then(JsonValue::as_object)
    else {
        return true;
    };
    let Some(result) = payload.get("result").and_then(JsonValue::as_object) else {
        return true;
    };
    !matches!(
        result.get("status").and_then(JsonValue::as_string),
        Some(
            "BOOT_AUTHORITY_CREATED"
                | "BOOT_READY"
                | "FENCE_ACCEPTED"
                | "LEASE_ACTIVE"
                | "LEASE_RENEWED"
                | "LEASE_REVOKED"
                | "INTENT_RECORDED"
                | "ACCEPTED"
                | "SETTLED"
                | "RECONCILED"
                | "DUPLICATE"
        )
    )
}

pub(crate) fn unknown(kind: &str, request: &JsonValue, correlation: &str) -> JsonValue {
    let mut fields = vec![
        (
            "contract".to_owned(),
            JsonValue::string("watchdog-recovery-v1"),
        ),
        (
            "schema_digest".to_owned(),
            JsonValue::string(RECOVERY_SCHEMA_DIGEST),
        ),
        (
            "kind".to_owned(),
            JsonValue::string(format!("{kind}_response")),
        ),
        ("correlation_id".to_owned(), JsonValue::string(correlation)),
        ("status".to_owned(), JsonValue::string("UNKNOWN")),
        ("retryable".to_owned(), JsonValue::Bool(false)),
        ("mutation_resubmitted".to_owned(), JsonValue::Bool(false)),
    ];
    if let Some(payload) = request
        .as_object()
        .and_then(|o| o.get("payload"))
        .and_then(JsonValue::as_object)
    {
        for key in ["operation_id", "payload_digest"] {
            if let Some(value) = payload.get(key).filter(|value| value.as_string().is_some()) {
                fields.push((key.to_owned(), value.clone()));
            }
        }
        if let Some(operation) = payload.get("operation").and_then(JsonValue::as_object) {
            for key in ["operation_id", "payload_digest"] {
                if let Some(value) = operation
                    .get(key)
                    .filter(|value| value.as_string().is_some())
                {
                    fields.push((key.to_owned(), value.clone()));
                }
            }
        }
    }
    JsonValue::object(fields)
}

fn redact_secrets(value: &mut JsonValue) {
    let Some(object) = value.as_object_mut() else {
        return;
    };
    object.remove("fence_token");
    for nested in object.values_mut() {
        redact_secrets(nested);
    }
}
