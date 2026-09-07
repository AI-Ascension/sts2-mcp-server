// SPDX-License-Identifier: MIT

use super::{CapabilityCatalog, MAX_IDENTIFIER_BYTES, ToolDescriptor};
use crate::json::JsonValue;

pub(super) const REVISION: &str = "runtime-v4-expert-mcp";
pub(super) const EXPERT_STATE_TOOL: &str = "sts2.expert_state";
pub(super) const EXPERT_ACTION_TOOL: &str = "sts2.expert_action";
pub(super) const EXPERT_RECONCILE_TOOL: &str = "sts2.expert_reconcile";
const INSTANCE_ID_PATTERN: &str = "^[A-Za-z0-9_-]{1,128}$";
const SESSION_ID_PATTERN: &str = "^[A-Za-z0-9_.:/-]{1,128}$";

pub(super) fn build() -> super::ToolCatalog {
    super::ToolCatalog {
        revision: String::from(REVISION),
        capabilities: CapabilityCatalog::default(),
        tools: vec![
            ToolDescriptor {
                name: String::from(EXPERT_STATE_TOOL),
                description: String::from(
                    "Read one bounded host-owned expert-state observation through the authenticated gateway.",
                ),
                input_schema: JsonValue::object([
                    (String::from("type"), JsonValue::string("object")),
                    (String::from("additionalProperties"), JsonValue::Bool(false)),
                    (
                        String::from("required"),
                        JsonValue::Array(vec![
                            JsonValue::string("instance_id"),
                            JsonValue::string("mcp_session_id"),
                        ]),
                    ),
                    (
                        String::from("properties"),
                        JsonValue::object([
                            (
                                String::from("instance_id"),
                                bounded_string(INSTANCE_ID_PATTERN),
                            ),
                            (
                                String::from("mcp_session_id"),
                                bounded_string(SESSION_ID_PATTERN),
                            ),
                        ]),
                    ),
                ]),
            },
            ToolDescriptor {
                name: String::from(EXPERT_ACTION_TOOL),
                description: String::from(
                    "Dispatch one current host-generated use_potion action and reconcile by operation identity.",
                ),
                input_schema: action_schema(),
            },
            ToolDescriptor {
                name: String::from(EXPERT_RECONCILE_TOOL),
                description: String::from(
                    "Read the authoritative result for one previously accepted use_potion operation.",
                ),
                input_schema: reconcile_schema(),
            },
        ],
    }
}

fn reconcile_schema() -> JsonValue {
    JsonValue::object([
        (String::from("type"), JsonValue::string("object")),
        (String::from("additionalProperties"), JsonValue::Bool(false)),
        (
            String::from("required"),
            JsonValue::Array(
                [
                    "instance_id",
                    "mcp_session_id",
                    "lease_id",
                    "lease_epoch",
                    "operation_id",
                ]
                .into_iter()
                .map(JsonValue::string)
                .collect(),
            ),
        ),
        (
            String::from("properties"),
            JsonValue::object([
                (
                    String::from("instance_id"),
                    bounded_string(INSTANCE_ID_PATTERN),
                ),
                (
                    String::from("mcp_session_id"),
                    bounded_string(SESSION_ID_PATTERN),
                ),
                (String::from("lease_id"), bounded_string(SESSION_ID_PATTERN)),
                (String::from("lease_epoch"), bounded_counter()),
                (
                    String::from("operation_id"),
                    bounded_string(SESSION_ID_PATTERN),
                ),
            ]),
        ),
    ])
}

fn action_schema() -> JsonValue {
    JsonValue::object([
        (String::from("type"), JsonValue::string("object")),
        (String::from("additionalProperties"), JsonValue::Bool(false)),
        (
            String::from("required"),
            JsonValue::Array(
                [
                    "instance_id",
                    "mcp_session_id",
                    "lease_id",
                    "lease_epoch",
                    "generation",
                    "state_id",
                    "operation_id",
                    "action",
                ]
                .into_iter()
                .map(JsonValue::string)
                .collect(),
            ),
        ),
        (
            String::from("properties"),
            JsonValue::object([
                (
                    String::from("instance_id"),
                    bounded_string(INSTANCE_ID_PATTERN),
                ),
                (
                    String::from("mcp_session_id"),
                    bounded_string(SESSION_ID_PATTERN),
                ),
                (String::from("lease_id"), bounded_string(SESSION_ID_PATTERN)),
                (String::from("lease_epoch"), bounded_counter()),
                (String::from("generation"), bounded_counter()),
                (String::from("state_id"), bounded_string(SESSION_ID_PATTERN)),
                (
                    String::from("operation_id"),
                    bounded_string(SESSION_ID_PATTERN),
                ),
                (String::from("action"), potion_action_schema()),
            ]),
        ),
    ])
}

fn bounded_counter() -> JsonValue {
    JsonValue::object([
        (String::from("type"), JsonValue::string("integer")),
        (String::from("minimum"), JsonValue::Number(0)),
        (
            String::from("maximum"),
            JsonValue::Number(9_007_199_254_740_991),
        ),
    ])
}

fn potion_action_schema() -> JsonValue {
    JsonValue::object([
        (String::from("type"), JsonValue::string("object")),
        (String::from("additionalProperties"), JsonValue::Bool(false)),
        (
            String::from("required"),
            JsonValue::Array(vec![
                JsonValue::string("action_id"),
                JsonValue::string("action"),
            ]),
        ),
        (
            String::from("properties"),
            JsonValue::object([
                (
                    String::from("action_id"),
                    bounded_string(SESSION_ID_PATTERN),
                ),
                (
                    String::from("action"),
                    JsonValue::object([
                        (String::from("type"), JsonValue::string("object")),
                        (String::from("additionalProperties"), JsonValue::Bool(false)),
                        (
                            String::from("required"),
                            JsonValue::Array(vec![
                                JsonValue::string("kind"),
                                JsonValue::string("potion_id"),
                                JsonValue::string("target_id"),
                            ]),
                        ),
                        (
                            String::from("properties"),
                            JsonValue::object([
                                (
                                    String::from("kind"),
                                    JsonValue::object([(
                                        String::from("const"),
                                        JsonValue::string("use_potion"),
                                    )]),
                                ),
                                (
                                    String::from("potion_id"),
                                    bounded_string(SESSION_ID_PATTERN),
                                ),
                                (String::from("target_id"), nullable_identity()),
                            ]),
                        ),
                    ]),
                ),
            ]),
        ),
    ])
}

fn nullable_identity() -> JsonValue {
    JsonValue::object([(
        String::from("anyOf"),
        JsonValue::Array(vec![
            bounded_string(SESSION_ID_PATTERN),
            JsonValue::object([(String::from("type"), JsonValue::string("null"))]),
        ]),
    )])
}

fn bounded_string(pattern: &str) -> JsonValue {
    JsonValue::object([
        (String::from("type"), JsonValue::string("string")),
        (String::from("minLength"), JsonValue::Number(1)),
        (
            String::from("maxLength"),
            JsonValue::Number(MAX_IDENTIFIER_BYTES as i64),
        ),
        (String::from("pattern"), JsonValue::string(pattern)),
    ])
}
