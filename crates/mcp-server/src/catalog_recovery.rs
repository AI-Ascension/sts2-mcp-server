// SPDX-License-Identifier: MIT

use super::{CapabilityCatalog, ToolDescriptor, recovery_schema};
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

type ToolFactory = fn() -> JsonValue;

const TOOLS: [(&str, &str, ToolFactory); 9] = [
    (
        BOOTSTRAP_TOOL,
        "bootstrap authority through the authenticated recovery sideband",
        recovery_schema::bootstrap,
    ),
    (
        HOST_FENCE_TOOL,
        "replace the host fence for the current boot context",
        recovery_schema::host_fence,
    ),
    (
        LEASE_ACQUIRE_TOOL,
        "acquire one fresh fenced gameplay lease",
        recovery_schema::lease_acquire,
    ),
    (
        LEASE_RENEW_TOOL,
        "renew one current lease with a monotonic sequence",
        recovery_schema::lease_renew,
    ),
    (
        LEASE_REVOKE_TOOL,
        "durably revoke one lease before cleanup",
        recovery_schema::lease_revoke,
    ),
    (
        OPERATION_INTENT_TOOL,
        "record a mutation intent before dispatch",
        recovery_schema::operation_intent,
    ),
    (
        OPERATION_DISPATCH_TOOL,
        "dispatch an existing intent without changing its identity",
        recovery_schema::operation_dispatch,
    ),
    (
        OPERATION_LOOKUP_TOOL,
        "read one historical operation without mutation authority",
        recovery_schema::operation_lookup,
    ),
    (
        OPERATION_RECONCILE_TOOL,
        "reconcile one operation without resending its mutation",
        recovery_schema::operation_reconcile,
    ),
];

pub(super) fn build() -> super::ToolCatalog {
    let tools = TOOLS
        .iter()
        .map(|(name, description, payload)| ToolDescriptor {
            name: (*name).to_owned(),
            description: (*description).to_owned(),
            input_schema: recovery_schema::tool(payload()),
        })
        .collect();
    super::ToolCatalog {
        revision: REVISION.to_owned(),
        capabilities: CapabilityCatalog::default(),
        tools,
    }
}
