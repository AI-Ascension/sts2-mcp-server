// SPDX-License-Identifier: MIT

//! Admission tickets, effect witnesses, and operation-result consistency checks.

use crate::json::JsonValue;

use super::context::{expected_boundary, original_context};
use crate::recovery_canonical::{exact, string, validate_action};
use crate::recovery_fields::{
    digest, enum_value, nullable, number, positive, timestamp, uuid, uuid4, wire,
};

pub(super) fn operation_record(value: &JsonValue) -> Result<(), &'static str> {
    let object = obj(value)?;
    exact(
        object,
        &[
            "operation_id",
            "state",
            "payload_digest",
            "original_context",
            "expected_boundary",
            "action",
            "ticket",
            "witness",
            "uncertainty_reason",
            "created_at",
            "updated_at",
        ],
    )?;
    let operation_id = string(object, "operation_id")?;
    let state = string(object, "state")?;
    let digest_value = string(object, "payload_digest")?;
    uuid4(operation_id)?;
    enum_value(
        state,
        &[
            "INTENT_RECORDED",
            "MAY_HAVE_BEEN_DISPATCHED",
            "ACCEPTED",
            "SETTLED",
            "REJECTED",
            "UNKNOWN",
            "RECONCILED",
        ],
    )?;
    digest(digest_value)?;
    original_context(
        object
            .get("original_context")
            .ok_or("original context missing")?,
    )?;
    expected_boundary(
        object
            .get("expected_boundary")
            .ok_or("expected boundary missing")?,
    )?;
    validate_action(object.get("action").ok_or("action missing")?, digest_value)?;
    nullable(object.get("ticket"), |value| {
        ticket(value, operation_id, digest_value)
    })?;
    nullable(object.get("witness"), |value| {
        witness_for_operation(value, operation_id, digest_value)
    })?;
    let uncertainty = object.get("uncertainty_reason");
    match (state == "UNKNOWN", uncertainty) {
        (true, Some(JsonValue::String(_))) => nullable(uncertainty, |value| {
            enum_value(
                value.as_string().unwrap_or(""),
                &[
                    "transport_lost",
                    "timeout",
                    "gateway_crash",
                    "host_crash",
                    "receipt_missing",
                    "authority_rotated",
                ],
            )
        })?,
        (false, Some(JsonValue::Null)) => {}
        _ => return Err("recovery uncertainty reason does not match operation state"),
    }
    timestamp(string(object, "created_at")?)?;
    timestamp(string(object, "updated_at")?)
}

pub(super) fn ticket(
    value: &JsonValue,
    operation_id: &str,
    payload_digest: &str,
) -> Result<(), &'static str> {
    let object = obj(value)?;
    exact(
        object,
        &[
            "ticket_id",
            "operation_id",
            "payload_digest",
            "boot_id",
            "instance_incarnation",
            "lease_epoch",
            "host_fence_id",
            "state",
            "issued_at",
            "expires_at",
        ],
    )?;
    uuid4(string(object, "ticket_id")?)?;
    if string(object, "operation_id")? != operation_id
        || string(object, "payload_digest")? != payload_digest
    {
        return Err("recovery admission ticket does not match operation identity");
    }
    uuid4(string(object, "operation_id")?)?;
    digest(string(object, "payload_digest")?)?;
    uuid4(string(object, "boot_id")?)?;
    uuid4(string(object, "instance_incarnation")?)?;
    positive(number(object, "lease_epoch")?)?;
    uuid4(string(object, "host_fence_id")?)?;
    enum_value(
        string(object, "state")?,
        &[
            "ISSUED",
            "ADMITTED",
            "EXECUTING",
            "EFFECT_WITNESS_RECORDED",
            "SETTLED",
            "REJECTED",
            "UNKNOWN",
        ],
    )?;
    timestamp(string(object, "issued_at")?)?;
    timestamp(string(object, "expires_at")?)
}

pub(super) fn witness_for_operation(
    value: &JsonValue,
    operation_id: &str,
    payload_digest: &str,
) -> Result<(), &'static str> {
    witness(value)?;
    let object = obj(value)?;
    if string(object, "operation_id")? != operation_id
        || string(object, "payload_digest")? != payload_digest
    {
        Err("recovery witness does not match operation identity")
    } else {
        Ok(())
    }
}

pub(super) fn witness(value: &JsonValue) -> Result<(), &'static str> {
    let object = obj(value)?;
    exact(
        object,
        &[
            "witness_id",
            "operation_id",
            "payload_digest",
            "boot_id",
            "instance_incarnation",
            "host_fence_id",
            "source",
            "state_id",
            "generation",
            "effect_digest",
            "observed_at",
        ],
    )?;
    uuid4(string(object, "witness_id")?)?;
    uuid4(string(object, "operation_id")?)?;
    digest(string(object, "payload_digest")?)?;
    uuid4(string(object, "boot_id")?)?;
    uuid4(string(object, "instance_incarnation")?)?;
    uuid4(string(object, "host_fence_id")?)?;
    enum_value(
        string(object, "source")?,
        &[
            "host_game_thread",
            "host_receipt",
            "authoritative_reobserve",
        ],
    )?;
    uuid(string(object, "state_id")?)?;
    wire(number(object, "generation")?)?;
    digest(string(object, "effect_digest")?)?;
    timestamp(string(object, "observed_at")?)
}

pub(super) fn related_operation_fields(
    operation: &JsonValue,
    witness: &JsonValue,
) -> Result<(), &'static str> {
    let operation = obj(operation)?;
    let witness = obj(witness)?;
    if operation.get("operation_id") == witness.get("operation_id")
        && operation.get("payload_digest") == witness.get("payload_digest")
    {
        Ok(())
    } else {
        Err("recovery witness does not match operation identity")
    }
}

pub(super) fn operation_matches_result(
    operation: &JsonValue,
    status: &str,
) -> Result<(), &'static str> {
    let state = string(obj(operation)?, "state")?;
    let matches = match status {
        "INTENT_RECORDED"
        | "MAY_HAVE_BEEN_DISPATCHED"
        | "ACCEPTED"
        | "SETTLED"
        | "REJECTED"
        | "UNKNOWN"
        | "RECONCILED" => state == status,
        "DUPLICATE" => true,
        _ => true,
    };
    if matches {
        Ok(())
    } else {
        Err("recovery operation state does not match result status")
    }
}

fn obj(value: &JsonValue) -> Result<&std::collections::BTreeMap<String, JsonValue>, &'static str> {
    value.as_object().ok_or("recovery field must be an object")
}
