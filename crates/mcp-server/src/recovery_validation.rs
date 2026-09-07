// SPDX-License-Identifier: MIT

//! Closed-field, identity, and correlation checks for recovery frames.

use std::collections::BTreeMap;

use crate::json::JsonValue;
use crate::protocol_artifact_recovery::{
    RECOVERY_CONTRACT, RECOVERY_MAX_AUTH_PROOF_BYTES, RECOVERY_SCHEMA_DIGEST,
};

use super::recovery_canonical::{exact, string};
use super::recovery_fields::{enum_value, timestamp, uuid, uuid4};
use super::recovery_payload::validate_payload;

pub(crate) fn capability(kind: &str) -> Option<&'static str> {
    match kind {
        "bootstrap" => Some("bootstrap"),
        "host_fence" => Some("host_fence"),
        "lease_acquire" => Some("lease_acquire"),
        "lease_renew" => Some("lease_renew"),
        "lease_revoke" => Some("lease_revoke"),
        "operation_intent" | "operation_dispatch" => Some("operation_submit"),
        "operation_lookup" => Some("recovery_read"),
        "operation_reconcile" => Some("recovery_reconcile"),
        _ => None,
    }
}

pub(crate) fn validate_request(
    frame: &JsonValue,
    kind: &str,
    correlation: &str,
    configured_instance: Option<&str>,
) -> Result<(), &'static str> {
    let object = common(
        frame,
        &format!("{kind}_request"),
        correlation,
        capability(kind),
    )?;
    let payload = object
        .get("payload")
        .ok_or("recovery frame is missing payload")?;
    validate_payload(payload, kind, false)?;
    if let Some(instance) = configured_instance {
        // A recovery profile has UUID identities even though legacy runtime
        // profiles may use segment-shaped instance names.  Never silently
        // disable the configured-instance binding when the supplied setting
        // is malformed; reject it before a frame can reach the gateway.
        uuid(instance)?;
        validate_payload_instance(payload, instance)?;
    }
    Ok(())
}

pub(crate) fn validate_response(
    frame: &JsonValue,
    kind: &str,
    correlation: &str,
) -> Result<(), &'static str> {
    let object = common(
        frame,
        &format!("{kind}_response"),
        correlation,
        capability(kind),
    )?;
    validate_payload(
        object
            .get("payload")
            .ok_or("recovery frame is missing payload")?,
        kind,
        true,
    )
}

fn common<'a>(
    frame: &'a JsonValue,
    expected_kind: &str,
    correlation: &str,
    expected_capability: Option<&str>,
) -> Result<&'a BTreeMap<String, JsonValue>, &'static str> {
    let object = frame
        .as_object()
        .ok_or("recovery frame must be an object")?;
    exact(
        object,
        &[
            "contract",
            "schema_digest",
            "message_id",
            "correlation_id",
            "sent_at",
            "actor",
            "auth",
            "kind",
            "payload",
        ],
    )?;
    if string(object, "contract")? != RECOVERY_CONTRACT
        || string(object, "schema_digest")? != RECOVERY_SCHEMA_DIGEST
        || string(object, "kind")? != expected_kind
        || string(object, "correlation_id")? != correlation
    {
        return Err("recovery frame contract, digest, kind, or correlation mismatched");
    }
    uuid4(string(object, "message_id")?)?;
    uuid4(correlation)?;
    timestamp(string(object, "sent_at")?)?;
    let actor = object
        .get("actor")
        .and_then(JsonValue::as_object)
        .ok_or("recovery actor must be an object")?;
    exact(actor, &["principal_id", "role"])?;
    uuid(string(actor, "principal_id")?)?;
    enum_value(
        string(actor, "role")?,
        &["gateway", "watchdog", "harness", "host", "mod", "operator"],
    )?;
    let auth = object
        .get("auth")
        .and_then(JsonValue::as_object)
        .ok_or("recovery auth must be an object")?;
    exact(auth, &["principal_id", "capability", "proof"])?;
    uuid(string(auth, "principal_id")?)?;
    if string(auth, "principal_id")? != string(actor, "principal_id")? {
        return Err("recovery actor and auth principal differ");
    }
    if Some(string(auth, "capability")?) != expected_capability {
        return Err("recovery capability is not authorized for this route");
    }
    match auth.get("proof") {
        Some(JsonValue::Null) => {}
        Some(JsonValue::String(value))
            if !value.is_empty()
                && value.len() <= RECOVERY_MAX_AUTH_PROOF_BYTES
                && value.bytes().all(|byte| byte.is_ascii_graphic()) => {}
        _ => return Err("recovery auth proof is invalid"),
    }
    Ok(object)
}

fn validate_payload_instance(payload: &JsonValue, expected: &str) -> Result<(), &'static str> {
    let object = payload
        .as_object()
        .ok_or("recovery payload must be an object")?;
    let mut ids = Vec::new();
    collect_instance_ids(object, &mut ids);
    if ids.iter().any(|instance_id| *instance_id != expected) {
        return Err("recovery instance identity does not match configured target");
    }
    Ok(())
}

fn collect_instance_ids<'a>(object: &'a BTreeMap<String, JsonValue>, ids: &mut Vec<&'a str>) {
    if let Some(instance_id) = object.get("instance_id").and_then(JsonValue::as_string) {
        ids.push(instance_id);
    }
    for value in object.values() {
        if let Some(nested) = value.as_object() {
            collect_instance_ids(nested, ids);
        }
    }
}
