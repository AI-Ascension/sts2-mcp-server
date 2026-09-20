// SPDX-License-Identifier: MIT

use super::{CapabilityCatalog, ToolDescriptor};
use crate::json::JsonValue;

pub(super) const REVISION: &str = "watchdog-recovery-v1-mcp";
pub(super) use super::ToolCatalog;

impl ToolCatalog {
    /// The recovery sideband: nine advertised tools, two with gateway routes.
    #[must_use]
    pub fn watchdog_recovery() -> Self {
        build()
    }

    /// Whether this catalog is the watchdog recovery sideband surface.
    pub(crate) fn is_watchdog_recovery(&self) -> bool {
        self.revision == REVISION
    }
}

/// The exact sideband tool surface the recovery profile advertises, in order.
const TOOLS: [(&str, &str, bool); 9] = [
    (
        "watchdog.bootstrap",
        "Create the durable recovery boot authority. This sideband profile does not carry the boot route.",
        false,
    ),
    (
        "watchdog.host_fence",
        "Submit a host fence for the durable recovery boot. This sideband profile does not carry the fence route.",
        false,
    ),
    (
        "watchdog.lease_acquire",
        "Acquire the durable recovery lease for a fenced boot. This sideband profile does not carry the lease route.",
        false,
    ),
    (
        "watchdog.lease_renew",
        "Renew the durable recovery lease. This sideband profile does not carry the lease route.",
        false,
    ),
    (
        "watchdog.lease_revoke",
        "Revoke the durable recovery lease. This sideband profile does not carry the lease route.",
        false,
    ),
    (
        "watchdog.operation_intent",
        "Record one operation intent before dispatch. This sideband profile does not carry the intent route.",
        false,
    ),
    (
        "watchdog.operation_dispatch",
        "Submit one recorded operation for dispatch. This sideband profile does not carry the dispatch route.",
        false,
    ),
    (
        "watchdog.operation_lookup",
        "Read the historical gateway record of one operation reference. This read never authorizes a mutation and never replays a host effect.",
        true,
    ),
    (
        "watchdog.operation_reconcile",
        "Resolve one uncertain operation against the authoritative gateway record without replaying the original action.",
        true,
    ),
];

pub(super) fn build() -> super::ToolCatalog {
    let input_schema = request_schema();
    ToolCatalog {
        revision: String::from(REVISION),
        capabilities: CapabilityCatalog::default(),
        tools: TOOLS
            .into_iter()
            .map(|(name, description, _)| ToolDescriptor {
                name: String::from(name),
                description: String::from(description),
                input_schema: input_schema.clone(),
            })
            .collect(),
        composition: None,
    }
}

fn request_schema() -> JsonValue {
    JsonValue::object([
        ("type".to_owned(), JsonValue::string("object")),
        (
            "properties".to_owned(),
            JsonValue::object([
                (
                    "mcp_session_id".to_owned(),
                    JsonValue::object([
                        ("type".to_owned(), JsonValue::string("string")),
                        ("minLength".to_owned(), JsonValue::Number(1)),
                        (
                            "maxLength".to_owned(),
                            JsonValue::Number(super::MAX_IDENTIFIER_BYTES as i64),
                        ),
                        (
                            "pattern".to_owned(),
                            JsonValue::string("^[A-Za-z0-9_.:/-]{1,128}$"),
                        ),
                    ]),
                ),
                (
                    "payload".to_owned(),
                    JsonValue::object([
                        ("type".to_owned(), JsonValue::string("object")),
                        (
                            "description".to_owned(),
                            JsonValue::string(
                                "Closed watchdog recovery payload forwarded verbatim to the authenticated gateway sideband route.",
                            ),
                        ),
                    ]),
                ),
            ]),
        ),
        (
            "required".to_owned(),
            JsonValue::Array(vec![
                JsonValue::string("mcp_session_id"),
                JsonValue::string("payload"),
            ]),
        ),
        ("additionalProperties".to_owned(), JsonValue::Bool(false)),
    ])
}

/// Advertised-but-unwired tools are listed for contract shape only; a call to
/// one is refused instead of being forwarded to a route this profile lacks.
pub(crate) fn is_wired(name: &str) -> bool {
    TOOLS
        .into_iter()
        .any(|(tool, _, wired)| tool == name && wired)
}
