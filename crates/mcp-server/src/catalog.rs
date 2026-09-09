// SPDX-License-Identifier: MIT

use crate::json::JsonValue;
use crate::transport::{LEGACY_MAX_FRAME_BYTES, MAX_FRAME_BYTES};

#[path = "catalog_coop_native.rs"]
mod coop_native;
#[path = "catalog_coop_receipt_query.rs"]
mod coop_receipt_query;
#[path = "catalog_coop_synchronization.rs"]
mod coop_synchronization;
#[path = "catalog_runtime.rs"]
mod runtime;
#[path = "catalog_runtime_map.rs"]
mod runtime_map;
#[path = "catalog_runtime_v2.rs"]
mod runtime_v2;
#[path = "catalog_runtime_v3_gameplay.rs"]
mod runtime_v3_gameplay;
#[path = "catalog_runtime_v4_expert.rs"]
mod runtime_v4_expert;
#[path = "catalog_runtime_v4_expert_rest_action.rs"]
mod runtime_v4_expert_rest_action;
#[path = "catalog_seeded_run.rs"]
mod seeded_run;

pub const GET_STATE_TOOL: &str = "get_state";
pub const SUBMIT_ACTION_TOOL: &str = "submit_action";
pub const RECONCILE_ACTION_TOOL: &str = "reconcile_action";
pub const OBSERVE_TOOL: &str = "sts2.observe";
pub const LEGAL_ACTIONS_TOOL: &str = "sts2.legal_actions";
pub const DISPATCH_ACTION_TOOL: &str = "sts2.dispatch_action";
pub const WAIT_FOR_TRANSITION_TOOL: &str = "sts2.wait_for_transition";
pub const REOBSERVE_TOOL: &str = "sts2.reobserve";
pub const RECOVER_TOOL: &str = "sts2.recover";
pub const MAP_SNAPSHOT_TOOL: &str = "sts2.map_snapshot";
pub const COOP_SYNCHRONIZATION_TOOL: &str = coop_synchronization::SYNC_TOOL;
pub const COOP_RECEIPT_QUERY_TOOL: &str = coop_receipt_query::COOP_RECEIPT_QUERY_TOOL;
pub const COOP_NATIVE_OBSERVATION_TOOL: &str = coop_native::OBSERVATION_TOOL;
pub const COOP_NATIVE_ACTION_TOOL: &str = coop_native::ACTION_TOOL;
pub const COOP_NATIVE_VOTE_TOOL: &str = coop_native::VOTE_TOOL;
pub const COOP_NATIVE_REJOIN_TOOL: &str = coop_native::REJOIN_TOOL;
pub const COOP_NATIVE_EFFECT_TOOL: &str = coop_native::EFFECT_TOOL;
pub const COOP_NATIVE_RECOVER_TOOL: &str = coop_native::RECOVER_TOOL;
pub const COOP_NATIVE_LEGAL_CATALOG_TOOL: &str = coop_native::LEGAL_CATALOG_TOOL;
pub const EXPERT_STATE_TOOL: &str = runtime_v4_expert::EXPERT_STATE_TOOL;
pub const EXPERT_ACTION_TOOL: &str = runtime_v4_expert::EXPERT_ACTION_TOOL;
pub const EXPERT_RECONCILE_TOOL: &str = runtime_v4_expert::EXPERT_RECONCILE_TOOL;
pub const EXPERT_REST_ACTION_TOOL: &str = runtime_v4_expert_rest_action::EXPERT_REST_ACTION_TOOL;
pub const EXPERT_REST_RECONCILE_TOOL: &str =
    runtime_v4_expert_rest_action::EXPERT_REST_RECONCILE_TOOL;
pub const START_SEEDED_RUN_TOOL: &str = seeded_run::START_SEEDED_RUN_TOOL;
pub const RECONCILE_SEEDED_RUN_TOOL: &str = seeded_run::RECONCILE_SEEDED_RUN_TOOL;
pub(crate) const MAX_IDENTIFIER_BYTES: usize = 128;
const INSTANCE_ID_PATTERN: &str = "^[A-Za-z0-9_-]{1,128}$";
const SESSION_ID_PATTERN: &str = "^[A-Za-z0-9_.:/-]{1,128}$";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolDescriptor {
    pub name: String,
    pub description: String,
    pub input_schema: JsonValue,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilityCatalog {
    pub supports_tools: bool,
}

impl Default for CapabilityCatalog {
    fn default() -> Self {
        Self {
            supports_tools: true,
        }
    }
}

impl CapabilityCatalog {
    pub(crate) fn to_json(&self) -> JsonValue {
        let tools = if self.supports_tools {
            JsonValue::Object(Default::default())
        } else {
            JsonValue::Null
        };
        JsonValue::object([(String::from("tools"), tools)])
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolCatalog {
    pub revision: String,
    pub capabilities: CapabilityCatalog,
    tools: Vec<ToolDescriptor>,
}

#[path = "catalog_default.rs"]
mod default;

impl ToolCatalog {
    #[must_use]
    pub fn runtime_v1() -> Self {
        runtime::build()
    }

    #[must_use]
    pub fn runtime_v2() -> Self {
        runtime_v2::build()
    }

    #[must_use]
    pub fn runtime_v3_gameplay() -> Self {
        runtime_v3_gameplay::build()
    }

    #[must_use]
    pub fn runtime_v4_expert() -> Self {
        runtime_v4_expert::build()
    }

    #[must_use]
    pub fn runtime_v4_expert_rest_action() -> Self {
        runtime_v4_expert_rest_action::build()
    }

    #[must_use]
    pub fn runtime_map_v1() -> Self {
        runtime_map::build()
    }

    #[must_use]
    pub fn coop_synchronization() -> Self {
        coop_synchronization::build()
    }

    #[must_use]
    pub fn coop_receipt_query() -> Self {
        coop_receipt_query::build()
    }

    #[must_use]
    pub fn seeded_run_v1() -> Self {
        seeded_run::build()
    }

    #[must_use]
    pub fn seeded_run() -> Self {
        Self::seeded_run_v1()
    }

    #[must_use]
    pub fn coop_native() -> Self {
        coop_native::build()
    }

    /// Largest MCP frame this profile accepts. The poc, runtime-v1, and runtime-v2
    /// profiles keep their historical 16 KiB limit; only the Runtime-v3 semantic
    /// profile accepts frames up to [`MAX_FRAME_BYTES`].
    #[must_use]
    pub fn max_frame_bytes(&self) -> usize {
        if self.is_runtime_v3_gameplay()
            || self.is_runtime_v4_expert()
            || self.is_runtime_v4_expert_rest_action()
            || self.is_runtime_map_v1()
            || self.is_seeded_run()
            || self.is_coop_native()
        {
            MAX_FRAME_BYTES
        } else {
            LEGACY_MAX_FRAME_BYTES
        }
    }

    pub(crate) fn is_runtime_v1(&self) -> bool {
        self.revision == "runtime-v1-mcp"
    }

    pub(crate) fn is_runtime_v2(&self) -> bool {
        self.revision == "runtime-v2-mcp"
    }

    pub(crate) fn is_runtime_v3_gameplay(&self) -> bool {
        self.revision == "runtime-v3-gameplay-mcp"
    }

    pub(crate) fn is_runtime_v4_expert(&self) -> bool {
        self.revision == runtime_v4_expert::REVISION
    }

    pub(crate) fn is_runtime_v4_expert_rest_action(&self) -> bool {
        self.revision == runtime_v4_expert_rest_action::REVISION
    }

    pub(crate) fn is_runtime_map_v1(&self) -> bool {
        self.revision == runtime_map::REVISION
    }

    pub(crate) fn is_coop_synchronization(&self) -> bool {
        self.revision == coop_synchronization::REVISION
    }

    pub(crate) fn is_coop_receipt_query(&self) -> bool {
        self.revision == coop_receipt_query::REVISION
    }

    pub(crate) fn is_seeded_run(&self) -> bool {
        self.revision == seeded_run::REVISION
    }

    pub(crate) fn is_coop_native(&self) -> bool {
        self.revision == coop_native::REVISION
    }

    pub(crate) fn descriptor(&self, name: &str) -> Option<&ToolDescriptor> {
        self.tools.iter().find(|tool| tool.name == name)
    }

    pub(crate) fn to_json(&self) -> JsonValue {
        let tools = self
            .tools
            .iter()
            .map(|tool| {
                JsonValue::object([
                    ("name".to_owned(), JsonValue::string(tool.name.as_str())),
                    (
                        "description".to_owned(),
                        JsonValue::string(tool.description.as_str()),
                    ),
                    ("inputSchema".to_owned(), tool.input_schema.clone()),
                ])
            })
            .collect();
        JsonValue::object([
            ("tools".to_owned(), JsonValue::Array(tools)),
            (
                "revision".to_owned(),
                JsonValue::string(self.revision.as_str()),
            ),
        ])
    }
}
