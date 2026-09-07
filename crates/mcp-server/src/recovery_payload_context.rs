// SPDX-License-Identifier: MIT

//! Bootstrap, fence, lease, and shared recovery payload checks.

use crate::json::JsonValue;

use crate::recovery_canonical::{exact, string};
use crate::recovery_fields::{
    digest, enum_value, nullable, number, number_value, obj, positive, timestamp, token, uuid,
    uuid4, wire,
};

pub(super) fn bootstrap_request(value: &JsonValue) -> Result<(), &'static str> {
    let object = obj(value)?;
    exact(
        object,
        &[
            "deployment_id",
            "instance_id",
            "instance_incarnation",
            "release",
            "lease_policy",
        ],
    )?;
    uuid(string(object, "deployment_id")?)?;
    uuid(string(object, "instance_id")?)?;
    uuid4(string(object, "instance_incarnation")?)?;
    release(object.get("release").ok_or("release missing")?)?;
    let policy = obj(object.get("lease_policy").ok_or("lease policy missing")?)?;
    exact(policy, &["ttl_seconds", "renewal_interval_seconds"])?;
    crate::recovery_fields::ttl(
        number(policy, "ttl_seconds")?,
        number(policy, "renewal_interval_seconds")?,
    )
}

pub(super) fn bootstrap_response(value: &JsonValue) -> Result<(), &'static str> {
    let object = obj(value)?;
    exact(object, &["result", "boot", "fence"])?;
    result(object.get("result").ok_or("result missing")?)?;
    nullable(object.get("boot"), boot_context)?;
    nullable(object.get("fence"), fence_context)
}

pub(super) fn host_fence_request(value: &JsonValue) -> Result<(), &'static str> {
    let object = obj(value)?;
    exact(object, &["boot"])?;
    boot_context(object.get("boot").ok_or("boot missing")?)
}

pub(super) fn host_fence_response(value: &JsonValue) -> Result<(), &'static str> {
    let object = obj(value)?;
    exact(object, &["result", "fence"])?;
    result(object.get("result").ok_or("result missing")?)?;
    nullable(object.get("fence"), fence_context)
}

pub(super) fn lease_acquire_request(value: &JsonValue) -> Result<(), &'static str> {
    let object = obj(value)?;
    exact(object, &["boot", "fence"])?;
    boot_context(object.get("boot").ok_or("boot missing")?)?;
    fence_context(object.get("fence").ok_or("fence missing")?)
}

pub(super) fn lease_response(value: &JsonValue) -> Result<(), &'static str> {
    let object = obj(value)?;
    exact(object, &["result", "lease"])?;
    result(object.get("result").ok_or("result missing")?)?;
    nullable(object.get("lease"), lease_context)
}

pub(super) fn lease_renew_request(value: &JsonValue) -> Result<(), &'static str> {
    let object = obj(value)?;
    exact(object, &["lease", "renew_sequence"])?;
    lease_context(object.get("lease").ok_or("lease missing")?)?;
    positive(number(object, "renew_sequence")?)
}

pub(super) fn lease_revoke_request(value: &JsonValue) -> Result<(), &'static str> {
    let object = obj(value)?;
    exact(object, &["lease", "reason"])?;
    lease_context(object.get("lease").ok_or("lease missing")?)?;
    enum_value(
        string(object, "reason")?,
        &[
            "operator",
            "shutdown",
            "incarnation_replaced",
            "suspend_ambiguous",
            "rekey",
        ],
    )
}

pub(super) fn revoke_response(value: &JsonValue) -> Result<(), &'static str> {
    let object = obj(value)?;
    exact(object, &["result"])?;
    result(object.get("result").ok_or("result missing")?)
}
pub(super) fn result(value: &JsonValue) -> Result<(), &'static str> {
    result_status(value).map(|_| ())
}

pub(super) fn result_status(value: &JsonValue) -> Result<&str, &'static str> {
    let object = obj(value)?;
    exact(object, &["status", "retryable", "retry_after_seconds"])?;
    let status = string(object, "status")?;
    enum_value(
        status,
        &[
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
        ],
    )?;
    if !matches!(object.get("retryable"), Some(JsonValue::Bool(_))) {
        return Err("retryable must be a boolean");
    }
    nullable(object.get("retry_after_seconds"), |value| {
        positive(number_value(value)?)
    })?;
    Ok(status)
}

pub(super) fn release(value: &JsonValue) -> Result<(), &'static str> {
    let object = obj(value)?;
    exact(
        object,
        &[
            "release_digest",
            "config_digest",
            "profile_digest",
            "runtime_v3_schema_digest",
        ],
    )?;
    for key in [
        "release_digest",
        "config_digest",
        "profile_digest",
        "runtime_v3_schema_digest",
    ] {
        digest(string(object, key)?)?;
    }
    Ok(())
}

pub(super) fn boot_context(value: &JsonValue) -> Result<(), &'static str> {
    let object = obj(value)?;
    exact(
        object,
        &[
            "deployment_id",
            "instance_id",
            "instance_incarnation",
            "boot_id",
            "authority_generation",
            "release",
            "created_at",
            "state",
        ],
    )?;
    uuid(string(object, "deployment_id")?)?;
    uuid(string(object, "instance_id")?)?;
    uuid4(string(object, "instance_incarnation")?)?;
    uuid4(string(object, "boot_id")?)?;
    positive(number(object, "authority_generation")?)?;
    release(object.get("release").ok_or("release missing")?)?;
    timestamp(string(object, "created_at")?)?;
    enum_value(
        string(object, "state")?,
        &["FENCE_REQUIRED", "READY", "BLOCKED", "REVOKED"],
    )
}

pub(super) fn fence_context(value: &JsonValue) -> Result<(), &'static str> {
    let object = obj(value)?;
    exact(
        object,
        &[
            "host_fence_id",
            "deployment_id",
            "instance_id",
            "instance_incarnation",
            "boot_id",
            "authority_generation",
            "fence_generation",
            "created_at",
        ],
    )?;
    uuid4(string(object, "host_fence_id")?)?;
    uuid(string(object, "deployment_id")?)?;
    uuid(string(object, "instance_id")?)?;
    uuid4(string(object, "instance_incarnation")?)?;
    uuid4(string(object, "boot_id")?)?;
    positive(number(object, "authority_generation")?)?;
    positive(number(object, "fence_generation")?)?;
    timestamp(string(object, "created_at")?)
}

pub(super) fn lease_context(value: &JsonValue) -> Result<(), &'static str> {
    let object = obj(value)?;
    exact(
        object,
        &[
            "deployment_id",
            "instance_id",
            "instance_incarnation",
            "boot_id",
            "authority_generation",
            "lease_id",
            "lease_epoch",
            "fence_token",
            "issued_at",
            "expires_at",
            "ttl_seconds",
            "renewal_interval_seconds",
        ],
    )?;
    uuid(string(object, "deployment_id")?)?;
    uuid(string(object, "instance_id")?)?;
    uuid4(string(object, "instance_incarnation")?)?;
    uuid4(string(object, "boot_id")?)?;
    positive(number(object, "authority_generation")?)?;
    uuid4(string(object, "lease_id")?)?;
    positive(number(object, "lease_epoch")?)?;
    token(string(object, "fence_token")?)?;
    timestamp(string(object, "issued_at")?)?;
    timestamp(string(object, "expires_at")?)?;
    crate::recovery_fields::ttl(
        number(object, "ttl_seconds")?,
        number(object, "renewal_interval_seconds")?,
    )
}

pub(super) fn original_context(value: &JsonValue) -> Result<(), &'static str> {
    let object = obj(value)?;
    exact(
        object,
        &[
            "deployment_id",
            "instance_id",
            "instance_incarnation",
            "boot_id",
            "authority_generation",
            "lease_id",
            "lease_epoch",
        ],
    )?;
    uuid(string(object, "deployment_id")?)?;
    uuid(string(object, "instance_id")?)?;
    uuid4(string(object, "instance_incarnation")?)?;
    uuid4(string(object, "boot_id")?)?;
    positive(number(object, "authority_generation")?)?;
    uuid4(string(object, "lease_id")?)?;
    positive(number(object, "lease_epoch")?)
}

pub(super) fn expected_boundary(value: &JsonValue) -> Result<(), &'static str> {
    let object = obj(value)?;
    exact(object, &["state_id", "generation", "catalog_digest"])?;
    uuid(string(object, "state_id")?)?;
    wire(number(object, "generation")?)?;
    digest(string(object, "catalog_digest")?)
}
