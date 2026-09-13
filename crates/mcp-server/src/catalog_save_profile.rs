// SPDX-License-Identifier: MIT

use super::{CapabilityCatalog, MAX_IDENTIFIER_BYTES, ToolDescriptor};
use crate::json::JsonValue;

pub(super) const REVISION: &str = "save-profile-v1-mcp";
pub(super) const CONTRACT: &str = "gateway-save-profile-v1";
pub(super) const LAUNCH_PROFILE_CONTRACT: &str = "gateway-launch-profile-v1";
pub(super) const LIST_TOOL: &str = "sts2.save_profile_list";
pub(super) const CURRENT_TOOL: &str = "sts2.save_profile_current";
pub(super) const SELECT_TOOL: &str = "sts2.save_profile_select";
pub(super) const CREATE_DISPOSABLE_TOOL: &str = "sts2.save_profile_create_disposable";
pub(super) const STATUS_TOOL: &str = "sts2.save_profile_status";

const IDENTITY_PATTERN: &str = "^(?!.*\\.\\.)(?!.*://)[A-Za-z0-9_.:-]{1,128}$";
const PATH_ID_PATTERN: &str = "^[A-Za-z0-9_-]{1,128}$";
const OPERATION_ID_PATTERN: &str = "^(?!.*\\.\\.)[A-Za-z0-9_.:-]{1,128}$";
const PROFILE_ID_PATTERN: &str = "^(?!.*\\.\\.)(?!.*://)[A-Za-z0-9_.:-]{1,128}$";
const DIGEST_PATTERN: &str = "^[0-9a-f]{64}$";

pub(super) fn build(read: bool, mutate: bool) -> super::ToolCatalog {
    let mut tools = Vec::new();
    if read {
        tools.extend([
            descriptor(
                LIST_TOOL,
                "List bounded save-profile summaries through the authenticated gateway. This read-only discovery never provisions or selects a profile.",
                context_schema(),
            ),
            descriptor(
                CURRENT_TOOL,
                "Read the current save-profile summary through the authenticated gateway. This read-only query never provisions or selects a profile.",
                context_schema(),
            ),
            descriptor(
                STATUS_TOOL,
                "Read one retained save-profile operation receipt by its path-safe operation identity. Reconciliation never replays a mutation.",
                status_schema(),
            ),
        ]);
    }
    if mutate {
        tools.extend([
            descriptor(
                SELECT_TOOL,
                "Select one bounded save profile with an explicit baseline fence. Selection is a mutation and is never exposed as passive discovery.",
                select_schema(),
            ),
            descriptor(
                CREATE_DISPOSABLE_TOOL,
                "Request one isolated disposable automation profile. The gateway owns allocation and authoritative baseline readback; this is a mutation.",
                context_schema(),
            ),
        ]);
    }
    super::ToolCatalog {
        revision: String::from(REVISION),
        capabilities: CapabilityCatalog::default(),
        tools,
        composition: None,
    }
}

pub(super) fn is_tool(name: &str) -> bool {
    matches!(
        name,
        LIST_TOOL | CURRENT_TOOL | SELECT_TOOL | CREATE_DISPOSABLE_TOOL | STATUS_TOOL
    )
}

pub(super) fn is_read_tool(name: &str) -> bool {
    matches!(name, LIST_TOOL | CURRENT_TOOL | STATUS_TOOL)
}

pub(super) fn is_mutation_tool(name: &str) -> bool {
    matches!(name, SELECT_TOOL | CREATE_DISPOSABLE_TOOL)
}

impl super::ToolCatalog {
    #[must_use]
    pub fn save_profile_v1() -> Self {
        build(true, true)
    }

    #[must_use]
    pub fn save_profile_v1_with_capabilities(read: bool, mutate: bool) -> Self {
        build(read, mutate)
    }

    #[must_use]
    pub fn save_profile_v1_read_only() -> Self {
        build(true, false)
    }

    #[must_use]
    pub fn save_profile_v1_unsupported() -> Self {
        build(false, false)
    }

    #[must_use]
    pub fn save_profile() -> Self {
        Self::save_profile_v1()
    }

    pub(crate) fn is_save_profile(&self) -> bool {
        self.revision == REVISION
    }

    pub(crate) fn save_profile_read_capability(&self) -> bool {
        self.is_save_profile() && self.tools.iter().any(|tool| is_read_tool(&tool.name))
    }

    pub(crate) fn save_profile_mutate_capability(&self) -> bool {
        self.is_save_profile() && self.tools.iter().any(|tool| is_mutation_tool(&tool.name))
    }

    pub(crate) fn capabilities_json(&self) -> JsonValue {
        let mut capabilities = self.capabilities.to_json();
        if self.is_save_profile()
            && let JsonValue::Object(object) = &mut capabilities
        {
            let read = self.save_profile_read_capability();
            let mutate = self.save_profile_mutate_capability();
            if !read && !mutate {
                object.insert(String::from("tools"), JsonValue::Null);
            }
            object.insert(
                String::from("save_profile"),
                JsonValue::object([
                    (String::from("contract"), JsonValue::string(CONTRACT)),
                    (String::from("revision"), JsonValue::string(REVISION)),
                    (String::from("supported"), JsonValue::Bool(read || mutate)),
                    (String::from("read"), JsonValue::Bool(read)),
                    (String::from("mutate"), JsonValue::Bool(mutate)),
                ]),
            );
        }
        capabilities
    }
}

fn descriptor(name: &str, description: &str, input_schema: JsonValue) -> ToolDescriptor {
    ToolDescriptor {
        name: String::from(name),
        description: String::from(description),
        input_schema,
    }
}

fn context_schema() -> JsonValue {
    object_schema(
        ["instance_id", "mcp_session_id", "lease_id", "lease_epoch"],
        [
            ("instance_id", bounded(PATH_ID_PATTERN)),
            ("mcp_session_id", bounded(IDENTITY_PATTERN)),
            ("lease_id", bounded(IDENTITY_PATTERN)),
            ("lease_epoch", epoch()),
        ],
    )
}

fn status_schema() -> JsonValue {
    object_schema(
        [
            "instance_id",
            "mcp_session_id",
            "lease_id",
            "lease_epoch",
            "operation_id",
        ],
        [
            ("instance_id", bounded(PATH_ID_PATTERN)),
            ("mcp_session_id", bounded(IDENTITY_PATTERN)),
            ("lease_id", bounded(IDENTITY_PATTERN)),
            ("lease_epoch", epoch()),
            ("operation_id", bounded(OPERATION_ID_PATTERN)),
        ],
    )
}

fn select_schema() -> JsonValue {
    object_schema(
        [
            "instance_id",
            "mcp_session_id",
            "lease_id",
            "lease_epoch",
            "profile_id",
            "baseline",
        ],
        [
            ("instance_id", bounded(PATH_ID_PATTERN)),
            ("mcp_session_id", bounded(IDENTITY_PATTERN)),
            ("lease_id", bounded(IDENTITY_PATTERN)),
            ("lease_epoch", epoch()),
            ("profile_id", bounded(PROFILE_ID_PATTERN)),
            ("baseline", baseline()),
        ],
    )
}

fn baseline() -> JsonValue {
    object_schema(
        ["identity", "digest"],
        [
            ("identity", bounded(IDENTITY_PATTERN)),
            ("digest", bounded(DIGEST_PATTERN)),
        ],
    )
}

fn object_schema(
    required: impl IntoIterator<Item = &'static str>,
    properties: impl IntoIterator<Item = (&'static str, JsonValue)>,
) -> JsonValue {
    JsonValue::object([
        ("type".to_owned(), JsonValue::string("object")),
        ("additionalProperties".to_owned(), JsonValue::Bool(false)),
        (
            "required".to_owned(),
            JsonValue::Array(required.into_iter().map(JsonValue::string).collect()),
        ),
        (
            "properties".to_owned(),
            JsonValue::object(
                properties
                    .into_iter()
                    .map(|(name, value)| (String::from(name), value)),
            ),
        ),
    ])
}

fn bounded(pattern: &str) -> JsonValue {
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

fn epoch() -> JsonValue {
    JsonValue::object([
        ("type".to_owned(), JsonValue::string("integer")),
        ("minimum".to_owned(), JsonValue::Number(0)),
        (
            "maximum".to_owned(),
            JsonValue::Number(9_007_199_254_740_991),
        ),
    ])
}
