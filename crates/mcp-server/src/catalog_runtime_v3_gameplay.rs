// SPDX-License-Identifier: MIT

use super::{
    CapabilityCatalog, GET_STATE_TOOL, INSTANCE_ID_PATTERN, MAX_IDENTIFIER_BYTES,
    RECONCILE_ACTION_TOOL, SESSION_ID_PATTERN, SUBMIT_ACTION_TOOL, ToolDescriptor,
};
use crate::json::JsonValue;

const OPERATION_ID_PATTERN: &str = "^[A-Za-z0-9_.:/-]{1,128}$";
const ROUTE_OPERATION_ID_PATTERN: &str = "^[A-Za-z0-9_.:-]{1,128}$";
const PROFILE_REVISION: &str = "runtime-v3-gameplay-mcp";
const ACTION_ID: &str = "play_card";
const MAX_CARD_INDEX: i64 = 64;

pub(super) fn build() -> super::ToolCatalog {
    super::ToolCatalog {
        revision: String::from(PROFILE_REVISION),
        capabilities: CapabilityCatalog::default(),
        tools: vec![
            ToolDescriptor {
                name: String::from(GET_STATE_TOOL),
                description: String::from(
                    "Read one bounded Runtime-v3 gameplay state snapshot through the authenticated gateway.",
                ),
                input_schema: state_schema(),
            },
            ToolDescriptor {
                name: String::from(SUBMIT_ACTION_TOOL),
                description: String::from(
                    "Submit exactly one bounded play_card operation with an optional target; settlement requires a fresh host observation witness.",
                ),
                input_schema: action_schema(),
            },
            ToolDescriptor {
                name: String::from(RECONCILE_ACTION_TOOL),
                description: String::from(
                    "Reconcile one previously submitted play_card operation by its stable operation_id.",
                ),
                input_schema: reconcile_schema(),
            },
        ],
    }
}

fn state_schema() -> JsonValue {
    schema(
        [
            "instance_id",
            "mcp_session_id",
            "lease_id",
            "lease_epoch",
            "generation",
        ],
        context_properties(false, false),
    )
}

fn action_schema() -> JsonValue {
    schema(
        [
            "instance_id",
            "mcp_session_id",
            "lease_id",
            "lease_epoch",
            "generation",
            "operation_id",
            "action_id",
            "card_index",
            "target_id",
        ],
        context_properties(true, true),
    )
}

fn reconcile_schema() -> JsonValue {
    schema(
        [
            "instance_id",
            "mcp_session_id",
            "lease_id",
            "lease_epoch",
            "generation",
            "operation_id",
        ],
        context_properties(true, false),
    )
}

fn schema<const N: usize>(required: [&str; N], properties: JsonValue) -> JsonValue {
    JsonValue::object([
        (String::from("type"), JsonValue::string("object")),
        (String::from("additionalProperties"), JsonValue::Bool(false)),
        (
            String::from("required"),
            JsonValue::Array(
                required
                    .into_iter()
                    .filter(|value| !value.is_empty())
                    .map(JsonValue::string)
                    .collect(),
            ),
        ),
        (String::from("properties"), properties),
    ])
}

fn context_properties(include_operation: bool, include_action: bool) -> JsonValue {
    let mut properties = vec![
        (
            String::from("instance_id"),
            bounded_string(INSTANCE_ID_PATTERN),
        ),
        (
            String::from("mcp_session_id"),
            bounded_string(SESSION_ID_PATTERN),
        ),
        (
            String::from("lease_id"),
            bounded_string(OPERATION_ID_PATTERN),
        ),
        (String::from("lease_epoch"), bounded_counter()),
        (String::from("generation"), bounded_counter()),
    ];
    if include_operation {
        properties.push((
            String::from("operation_id"),
            bounded_string(ROUTE_OPERATION_ID_PATTERN),
        ));
    }
    if include_action {
        properties.extend([
            (
                String::from("action_id"),
                JsonValue::object([
                    (String::from("type"), JsonValue::string("string")),
                    (String::from("const"), JsonValue::string(ACTION_ID)),
                ]),
            ),
            (
                String::from("card_index"),
                JsonValue::object([
                    (String::from("type"), JsonValue::string("integer")),
                    (String::from("minimum"), JsonValue::Number(0)),
                    (String::from("maximum"), JsonValue::Number(MAX_CARD_INDEX)),
                ]),
            ),
            (
                String::from("target_id"),
                JsonValue::object([(
                    String::from("anyOf"),
                    JsonValue::Array(vec![
                        bounded_string(OPERATION_ID_PATTERN),
                        JsonValue::object([("type".to_owned(), JsonValue::string("null"))]),
                    ]),
                )]),
            ),
        ]);
    }
    JsonValue::object(properties)
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
