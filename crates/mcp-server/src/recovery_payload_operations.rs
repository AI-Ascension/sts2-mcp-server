// SPDX-License-Identifier: MIT

//! Operation intent, dispatch, lookup, and reconciliation payload checks.

use crate::json::JsonValue;

use super::context::{
    expected_boundary, fence_context, lease_context, original_context, result_status,
};
use super::records::{
    operation_matches_result, operation_record, related_operation_fields, witness,
};
use crate::recovery_canonical::{exact, string, validate_action};
use crate::recovery_fields::{digest, enum_value, obj, uuid4};

pub(super) fn intent_request(value: &JsonValue) -> Result<(), &'static str> {
    let object = obj(value)?;
    exact(object, &["lease", "operation"])?;
    lease_context(object.get("lease").ok_or("lease missing")?)?;
    operation_intent(object.get("operation").ok_or("operation missing")?)
}

pub(super) fn dispatch_request(value: &JsonValue) -> Result<(), &'static str> {
    let object = obj(value)?;
    exact(object, &["lease", "operation"])?;
    lease_context(object.get("lease").ok_or("lease missing")?)?;
    operation_ref(object.get("operation").ok_or("operation missing")?)
}

pub(super) fn lookup_request(value: &JsonValue) -> Result<(), &'static str> {
    let object = obj(value)?;
    exact(object, &["operation", "lookup_scope"])?;
    operation_ref(object.get("operation").ok_or("operation missing")?)?;
    if string(object, "lookup_scope")? == "historical_read" {
        Ok(())
    } else {
        Err("lookup scope is not read-only")
    }
}

pub(super) fn reconcile_request(value: &JsonValue) -> Result<(), &'static str> {
    let object = obj(value)?;
    exact(object, &["operation", "strategy", "current_fence"])?;
    operation_ref(object.get("operation").ok_or("operation missing")?)?;
    enum_value(
        string(object, "strategy")?,
        &["reobserve", "receipt_lookup", "quarantine"],
    )?;
    fence_context(object.get("current_fence").ok_or("current fence missing")?)
}

pub(super) fn operation_response(value: &JsonValue) -> Result<(), &'static str> {
    let object = obj(value)?;
    let status = result_status(object.get("result").ok_or("result missing")?)?;
    exact(object, &["result", "operation"])?;
    if let Some(operation) = object.get("operation") {
        if !matches!(operation, JsonValue::Null) {
            operation_record(operation)?;
            operation_matches_result(operation, status)?;
        }
    } else {
        return Err("recovery nullable field is missing");
    }
    Ok(())
}

pub(super) fn lookup_response(value: &JsonValue) -> Result<(), &'static str> {
    let object = obj(value)?;
    let status = result_status(object.get("result").ok_or("result missing")?)?;
    exact(object, &["result", "operation", "mutation_authorized"])?;
    if let Some(operation) = object.get("operation") {
        if !matches!(operation, JsonValue::Null) {
            operation_record(operation)?;
            operation_matches_result(operation, status)?;
        }
    } else {
        return Err("recovery nullable field is missing");
    }
    if object.get("mutation_authorized") == Some(&JsonValue::Bool(false)) {
        Ok(())
    } else {
        Err("historical lookup cannot authorize mutation")
    }
}

pub(super) fn reconcile_response(value: &JsonValue) -> Result<(), &'static str> {
    let object = obj(value)?;
    let status = result_status(object.get("result").ok_or("result missing")?)?;
    exact(object, &["result", "operation", "witness"])?;
    let operation = object
        .get("operation")
        .ok_or("recovery operation is missing")?;
    let witness_value = object.get("witness").ok_or("recovery witness is missing")?;
    if !matches!(operation, JsonValue::Null) {
        operation_record(operation)?;
        operation_matches_result(operation, status)?;
    }
    if !matches!(witness_value, JsonValue::Null) {
        witness(witness_value)?;
        if !matches!(operation, JsonValue::Null) {
            related_operation_fields(operation, witness_value)?;
        }
    }
    Ok(())
}
pub(super) fn operation_intent(value: &JsonValue) -> Result<(), &'static str> {
    let object = obj(value)?;
    exact(
        object,
        &[
            "operation_id",
            "payload_digest",
            "original_context",
            "expected_boundary",
            "action",
        ],
    )?;
    let digest_value = string(object, "payload_digest")?;
    uuid4(string(object, "operation_id")?)?;
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
    validate_action(object.get("action").ok_or("action missing")?, digest_value)
}

pub(super) fn operation_ref(value: &JsonValue) -> Result<(), &'static str> {
    let object = obj(value)?;
    exact(
        object,
        &["operation_id", "payload_digest", "original_context"],
    )?;
    uuid4(string(object, "operation_id")?)?;
    digest(string(object, "payload_digest")?)?;
    original_context(
        object
            .get("original_context")
            .ok_or("original context missing")?,
    )
}
