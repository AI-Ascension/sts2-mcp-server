// SPDX-License-Identifier: MIT

use super::{CapabilityCatalog, MAX_IDENTIFIER_BYTES, ToolDescriptor};
use crate::json::JsonValue;

pub(super) const REVISION: &str = "runtime-v4-expert-rest-action-mcp";
pub(super) const EXPERT_REST_ACTION_TOOL: &str = "sts2.expert_rest_action";
pub(super) const EXPERT_REST_RECONCILE_TOOL: &str = "sts2.expert_rest_reconcile";
const INSTANCE_ID_PATTERN: &str = "^[A-Za-z0-9_-]{1,128}$";
const SESSION_ID_PATTERN: &str = "^[A-Za-z0-9_.:/-]{1,128}$";

pub(super) fn build() -> super::ToolCatalog {
    super::ToolCatalog {
        revision: String::from(REVISION),
        capabilities: CapabilityCatalog::default(),
        tools: vec![
            ToolDescriptor {
                name: String::from(super::EXPERT_STATE_TOOL),
                description: String::from(
                    "Read one bounded host-owned expert-state observation through the authenticated gateway.",
                ),
                input_schema: state_schema(),
            },
            ToolDescriptor {
                name: String::from(EXPERT_REST_ACTION_TOOL),
                description: String::from(
                    "Dispatch one typed native rest-site option or selector follow-up through the authenticated gateway.",
                ),
                input_schema: action_schema(),
            },
            ToolDescriptor {
                name: String::from(EXPERT_REST_RECONCILE_TOOL),
                description: String::from(
                    "Read the authoritative result for one previously accepted native rest-site operation by its original operation identity.",
                ),
                input_schema: reconcile_schema(),
            },
        ],
    }
}

fn state_schema() -> JsonValue {
    JsonValue::object([
        (String::from("type"), JsonValue::string("object")),
        (String::from("additionalProperties"), JsonValue::Bool(false)),
        (
            String::from("required"),
            JsonValue::Array(
                ["instance_id", "mcp_session_id"]
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
            ]),
        ),
    ])
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
                (String::from("action"), action_reference_schema()),
            ]),
        ),
    ])
}

fn action_reference_schema() -> JsonValue {
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
                    JsonValue::object([(
                        String::from("oneOf"),
                        JsonValue::Array(vec![
                            rest_option_schema(),
                            select_card_schema(),
                            select_player_schema(),
                            selection_control_schema(),
                        ]),
                    )]),
                ),
            ]),
        ),
    ])
}

fn rest_option_schema() -> JsonValue {
    exact_payload(
        "rest_option",
        [("rest_option_id", bounded_string(SESSION_ID_PATTERN))],
    )
}

fn select_card_schema() -> JsonValue {
    exact_payload(
        "select_card",
        [
            ("selection_id", bounded_string(SESSION_ID_PATTERN)),
            ("rest_option_id", bounded_string(SESSION_ID_PATTERN)),
            ("card_id", bounded_string(SESSION_ID_PATTERN)),
        ],
    )
}

fn select_player_schema() -> JsonValue {
    exact_payload(
        "select_player",
        [
            ("selection_id", bounded_string(SESSION_ID_PATTERN)),
            ("rest_option_id", bounded_string(SESSION_ID_PATTERN)),
            ("player_id", bounded_string(SESSION_ID_PATTERN)),
        ],
    )
}

fn selection_control_schema() -> JsonValue {
    JsonValue::object([
        (String::from("type"), JsonValue::string("object")),
        (String::from("additionalProperties"), JsonValue::Bool(false)),
        (
            String::from("required"),
            JsonValue::Array(
                ["kind", "selection_id", "rest_option_id"]
                    .into_iter()
                    .map(JsonValue::string)
                    .collect(),
            ),
        ),
        (
            String::from("properties"),
            JsonValue::object([
                (
                    String::from("kind"),
                    JsonValue::object([(
                        String::from("enum"),
                        JsonValue::Array(
                            ["confirm_selection", "cancel_selection"]
                                .into_iter()
                                .map(JsonValue::string)
                                .collect(),
                        ),
                    )]),
                ),
                (
                    String::from("selection_id"),
                    bounded_string(SESSION_ID_PATTERN),
                ),
                (
                    String::from("rest_option_id"),
                    bounded_string(SESSION_ID_PATTERN),
                ),
            ]),
        ),
    ])
}

fn exact_payload(
    kind: &str,
    fields: impl IntoIterator<Item = (&'static str, JsonValue)>,
) -> JsonValue {
    let mut required = vec![JsonValue::string("kind")];
    let mut properties = vec![(
        String::from("kind"),
        JsonValue::object([(String::from("const"), JsonValue::string(kind))]),
    )];
    for (name, schema) in fields {
        required.push(JsonValue::string(name));
        properties.push((String::from(name), schema));
    }
    JsonValue::object([
        (String::from("type"), JsonValue::string("object")),
        (String::from("additionalProperties"), JsonValue::Bool(false)),
        (String::from("required"), JsonValue::Array(required)),
        (String::from("properties"), JsonValue::object(properties)),
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
