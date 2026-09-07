// SPDX-License-Identifier: MIT

use super::{CapabilityCatalog, MAX_IDENTIFIER_BYTES, ToolDescriptor};
use crate::json::JsonValue;

pub(super) const REVISION: &str = "watchdog-recovery-v1-mcp";
pub const BOOTSTRAP_TOOL: &str = "watchdog.bootstrap";
pub const HOST_FENCE_TOOL: &str = "watchdog.host_fence";
pub const LEASE_ACQUIRE_TOOL: &str = "watchdog.lease_acquire";
pub const LEASE_RENEW_TOOL: &str = "watchdog.lease_renew";
pub const LEASE_REVOKE_TOOL: &str = "watchdog.lease_revoke";
pub const OPERATION_INTENT_TOOL: &str = "watchdog.operation_intent";
pub const OPERATION_DISPATCH_TOOL: &str = "watchdog.operation_dispatch";
pub const OPERATION_LOOKUP_TOOL: &str = "watchdog.operation_lookup";
pub const OPERATION_RECONCILE_TOOL: &str = "watchdog.operation_reconcile";

const TOOLS: [(&str, &str, &[&str]); 9] = [
    (
        BOOTSTRAP_TOOL,
        "bootstrap authority through the authenticated recovery sideband",
        &[
            "deployment_id",
            "instance_id",
            "instance_incarnation",
            "release",
            "lease_policy",
        ],
    ),
    (
        HOST_FENCE_TOOL,
        "replace the host fence for the current boot context",
        &["boot"],
    ),
    (
        LEASE_ACQUIRE_TOOL,
        "acquire one fresh fenced gameplay lease",
        &["boot", "fence"],
    ),
    (
        LEASE_RENEW_TOOL,
        "renew one current lease with a monotonic sequence",
        &["lease", "renew_sequence"],
    ),
    (
        LEASE_REVOKE_TOOL,
        "durably revoke one lease before cleanup",
        &["lease", "reason"],
    ),
    (
        OPERATION_INTENT_TOOL,
        "record a mutation intent before dispatch",
        &["lease", "operation"],
    ),
    (
        OPERATION_DISPATCH_TOOL,
        "dispatch an existing intent without changing its identity",
        &["lease", "operation"],
    ),
    (
        OPERATION_LOOKUP_TOOL,
        "read one historical operation without mutation authority",
        &["operation", "lookup_scope"],
    ),
    (
        OPERATION_RECONCILE_TOOL,
        "reconcile one operation without resending its mutation",
        &["operation", "strategy", "current_fence"],
    ),
];

pub(super) fn build() -> super::ToolCatalog {
    let tools = TOOLS
        .iter()
        .map(|(name, description, fields)| ToolDescriptor {
            name: (*name).to_owned(),
            description: (*description).to_owned(),
            input_schema: schema(fields),
        })
        .collect();
    super::ToolCatalog {
        revision: REVISION.to_owned(),
        capabilities: CapabilityCatalog::default(),
        tools,
    }
}

fn schema(fields: &[&str]) -> JsonValue {
    let required = vec![
        JsonValue::string("mcp_session_id"),
        JsonValue::string("payload"),
    ];
    let properties = JsonValue::object([
        (
            "mcp_session_id".to_owned(),
            bounded_string("^[A-Za-z0-9_.:/-]{1,128}$"),
        ),
        (
            "payload".to_owned(),
            JsonValue::object([
                ("type".to_owned(), JsonValue::string("object")),
                ("additionalProperties".to_owned(), JsonValue::Bool(false)),
                (
                    "required".to_owned(),
                    JsonValue::Array(
                        fields
                            .iter()
                            .map(|field| JsonValue::string(*field))
                            .collect(),
                    ),
                ),
                (
                    "properties".to_owned(),
                    JsonValue::Object(
                        fields
                            .iter()
                            .map(|field| ((*field).to_owned(), JsonValue::object([])))
                            .collect(),
                    ),
                ),
            ]),
        ),
    ]);
    JsonValue::object([
        ("type".to_owned(), JsonValue::string("object")),
        ("additionalProperties".to_owned(), JsonValue::Bool(false)),
        ("required".to_owned(), JsonValue::Array(required)),
        ("properties".to_owned(), properties),
    ])
}

fn bounded_string(pattern: &str) -> JsonValue {
    JsonValue::object([
        ("type".to_owned(), JsonValue::string("string")),
        ("minLength".to_owned(), JsonValue::Number(1)),
        (
            "maxLength".to_owned(),
            JsonValue::Number(MAX_IDENTIFIER_BYTES as i64),
        ),
        ("pattern".to_owned(), JsonValue::string(pattern)),
    ])
}
