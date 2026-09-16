// SPDX-License-Identifier: MIT

use crate::json::JsonValue;
use crate::protocol_artifact_hash::sha256_hex;
use crate::{
    EXACT_RESTORE_GATEWAY_CONTRACT, EXACT_RESTORE_GATEWAY_SCHEMA_DIGEST,
    EXACT_RESTORE_MAX_FRAME_BYTES,
};

use super::request::{ExactRestorePhase, ExactRestoreRequestBinding};
use super::schema;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExactRestoreResponse {
    pub frame: JsonValue,
    pub may_have_started: bool,
    pub lookup_only: bool,
}

pub fn validate_exact_restore_response(
    envelope: &JsonValue,
    request: &ExactRestoreRequestBinding,
    principal_id: &str,
) -> Result<ExactRestoreResponse, &'static str> {
    if envelope.to_json().len() > EXACT_RESTORE_MAX_FRAME_BYTES {
        return Err("exact-restore response wrapper exceeds 16384 bytes");
    }
    schema::validate_gateway(envelope)?;
    if super::string_member(envelope, "contract") != Some(EXACT_RESTORE_GATEWAY_CONTRACT)
        || super::string_member(envelope, "schema_digest")
            != Some(EXACT_RESTORE_GATEWAY_SCHEMA_DIGEST)
        || super::string_member(envelope, "kind") != Some("exact_restore_response")
    {
        return Err("exact-restore response wrapper contract is unsupported");
    }
    let actor = super::object_member(envelope, "actor").ok_or("response actor is missing")?;
    let auth = super::object_member(envelope, "auth").ok_or("response auth is missing")?;
    if super::string_member(actor, "principal_id") != Some(principal_id)
        || super::string_member(actor, "role") != Some("gateway")
        || super::string_member(auth, "principal_id") != Some(principal_id)
        || super::string_member(auth, "capability") != Some("exact_restore")
        || super::object_member(auth, "proof") != Some(&JsonValue::Null)
    {
        return Err("exact-restore response identity is not configured");
    }
    let frame = super::object_member(
        super::object_member(envelope, "payload").ok_or("response payload is missing")?,
        "frame",
    )
    .ok_or("response neutral frame is missing")?;
    if frame.to_json().len() > EXACT_RESTORE_MAX_FRAME_BYTES {
        return Err("exact-restore neutral response exceeds 16384 bytes");
    }
    schema::validate_neutral(frame)?;
    let kind = super::string_member(frame, "kind").ok_or("response kind is missing")?;
    let message_id = require_string(envelope, "message_id")?;
    let correlation_id = require_string(envelope, "correlation_id")?;
    if message_id != require_string(frame, "message_id")?
        || correlation_id != require_string(frame, "correlation_id")?
        || correlation_id != request.message_id
    {
        return Err("exact-restore response correlation does not match its request");
    }
    let payload = super::object_member(frame, "payload").ok_or("response payload is missing")?;
    if require_string(payload, "operation_id")? != request.operation_id
        || super::object_member(payload, "expected_owner") != Some(&request.expected_owner)
        || require_string(payload, "request_digest")? != request.request_digest
    {
        return Err("exact-restore response crosses operation, owner, or request digest");
    }
    let error_response = kind == "exact_restore_error_response";
    if !error_response && kind != request.phase.response_kind() {
        return Err("exact-restore response kind does not match the request phase");
    }
    let request_payload =
        super::object_member(&request.frame, "payload").ok_or("request payload is missing")?;
    validate_receipt(
        payload,
        &request.expected_owner,
        &request.operation_id,
        request_payload,
    )?;
    let state = super::string_member(payload, "state").unwrap_or_default();
    let may_have_started = state == "COMMIT_INTENT"
        || state == "UNKNOWN"
        || (error_response
            && super::string_member(payload, "outcome") == Some("UNAVAILABLE")
            && super::string_member(payload, "host_effect") == Some("may_have_started"));
    Ok(ExactRestoreResponse {
        frame: frame.clone(),
        may_have_started,
        lookup_only: may_have_started && request.phase == ExactRestorePhase::Commit,
    })
}

fn validate_receipt(
    payload: &JsonValue,
    expected_owner: &JsonValue,
    operation_id: &str,
    request_payload: &JsonValue,
) -> Result<(), &'static str> {
    let Some(receipt) = super::object_member(payload, "receipt") else {
        return Ok(());
    };
    let supplied = require_string(receipt, "receipt_digest")?;
    let mut unsigned = receipt.clone();
    let object = match &mut unsigned {
        JsonValue::Object(object) => object,
        _ => return Err("exact-restore receipt is not an object"),
    };
    object.remove("receipt_digest");
    let actual = format!("sha256:{}", sha256_hex(unsigned.to_json().as_bytes()));
    if supplied != actual
        || super::string_member(receipt, "operation_id") != Some(operation_id)
        || super::object_member(receipt, "destination_owner") != Some(expected_owner)
        || super::string_member(receipt, "exact_state_digest")
            != super::string_member(receipt, "recaptured_exact_state_digest")
    {
        return Err("exact-restore receipt digest or authority binding is invalid");
    }
    for field in [
        "branch",
        "checkpoint_id",
        "exact_state_digest",
        "manifest_digest",
        "closure_digest",
        "compatibility_digest",
        "coverage_contract_digest",
        "aggregate_closure_bytes",
        "artifact_reference_count",
        "distinct_blob_count",
        "boundary",
    ] {
        if let Some(request_value) = super::object_member(request_payload, field)
            && super::object_member(receipt, field) != Some(request_value)
        {
            return Err("exact-restore receipt differs from its request binding");
        }
    }
    Ok(())
}

fn require_string<'a>(value: &'a JsonValue, key: &str) -> Result<&'a str, &'static str> {
    super::string_member(value, key).ok_or("exact-restore response string field is missing")
}
