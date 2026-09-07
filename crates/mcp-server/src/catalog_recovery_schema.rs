// SPDX-License-Identifier: MIT

use crate::json::JsonValue;

#[path = "catalog_recovery_schema_parts.rs"]
mod parts;
use parts::{
    boot_context, bounded_string, closed, const_value, enum_value, host_fence_context,
    lease_context, lease_policy, operation_intent_context, operation_ref, positive_integer,
    release_set, uuid, uuid4,
};

const SESSION_PATTERN: &str = "^[A-Za-z0-9_.:/-]{1,128}$";

pub(super) fn tool(payload: JsonValue) -> JsonValue {
    closed(
        &["mcp_session_id", "payload"],
        [
            ("mcp_session_id", bounded_string(SESSION_PATTERN)),
            ("payload", payload),
        ],
    )
}

pub(super) fn bootstrap() -> JsonValue {
    closed(
        &[
            "deployment_id",
            "instance_id",
            "instance_incarnation",
            "release",
            "lease_policy",
        ],
        [
            ("deployment_id", uuid()),
            ("instance_id", uuid()),
            ("instance_incarnation", uuid4()),
            ("release", release_set()),
            ("lease_policy", lease_policy()),
        ],
    )
}

pub(super) fn host_fence() -> JsonValue {
    closed(&["boot"], [("boot", boot_context())])
}

pub(super) fn lease_acquire() -> JsonValue {
    closed(
        &["boot", "fence"],
        [("boot", boot_context()), ("fence", host_fence_context())],
    )
}

pub(super) fn lease_renew() -> JsonValue {
    closed(
        &["lease", "renew_sequence"],
        [
            ("lease", lease_context()),
            ("renew_sequence", positive_integer()),
        ],
    )
}

pub(super) fn lease_revoke() -> JsonValue {
    closed(
        &["lease", "reason"],
        [
            ("lease", lease_context()),
            (
                "reason",
                enum_value(&[
                    "operator",
                    "shutdown",
                    "incarnation_replaced",
                    "suspend_ambiguous",
                    "rekey",
                ]),
            ),
        ],
    )
}

pub(super) fn operation_intent() -> JsonValue {
    closed(
        &["lease", "operation"],
        [
            ("lease", lease_context()),
            ("operation", operation_intent_context()),
        ],
    )
}

pub(super) fn operation_dispatch() -> JsonValue {
    closed(
        &["lease", "operation"],
        [("lease", lease_context()), ("operation", operation_ref())],
    )
}

pub(super) fn operation_lookup() -> JsonValue {
    closed(
        &["operation", "lookup_scope"],
        [
            ("operation", operation_ref()),
            ("lookup_scope", const_value("historical_read")),
        ],
    )
}

pub(super) fn operation_reconcile() -> JsonValue {
    closed(
        &["operation", "strategy", "current_fence"],
        [
            ("operation", operation_ref()),
            (
                "strategy",
                enum_value(&["reobserve", "receipt_lookup", "quarantine"]),
            ),
            ("current_fence", host_fence_context()),
        ],
    )
}
