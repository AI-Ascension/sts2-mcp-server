// SPDX-License-Identifier: MIT

//! Closed payload checks for watchdog-recovery-v1 request and response kinds.

use crate::json::JsonValue;

#[path = "recovery_payload_context.rs"]
mod context;
#[path = "recovery_payload_operations.rs"]
mod operations;
#[path = "recovery_payload_records.rs"]
mod records;

pub(super) fn validate_payload(
    value: &JsonValue,
    kind: &str,
    response: bool,
) -> Result<(), &'static str> {
    match (kind, response) {
        ("bootstrap", false) => context::bootstrap_request(value),
        ("bootstrap", true) => context::bootstrap_response(value),
        ("host_fence", false) => context::host_fence_request(value),
        ("host_fence", true) => context::host_fence_response(value),
        ("lease_acquire", false) => context::lease_acquire_request(value),
        ("lease_acquire", true) => context::lease_response(value),
        ("lease_renew", false) => context::lease_renew_request(value),
        ("lease_renew", true) => context::lease_response(value),
        ("lease_revoke", false) => context::lease_revoke_request(value),
        ("lease_revoke", true) => context::revoke_response(value),
        ("operation_intent", false) => operations::intent_request(value),
        ("operation_intent", true) => operations::operation_response(value),
        ("operation_dispatch", false) => operations::dispatch_request(value),
        ("operation_dispatch", true) => operations::operation_response(value),
        ("operation_lookup", false) => operations::lookup_request(value),
        ("operation_lookup", true) => operations::lookup_response(value),
        ("operation_reconcile", false) => operations::reconcile_request(value),
        ("operation_reconcile", true) => operations::reconcile_response(value),
        _ => Err("recovery kind is unsupported"),
    }
}
