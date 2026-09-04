// SPDX-License-Identifier: MIT

use super::{
    CapabilityCatalog, GET_STATE_TOOL, INSTANCE_ID_PATTERN, MAX_IDENTIFIER_BYTES,
    RECONCILE_ACTION_TOOL, SESSION_ID_PATTERN, SUBMIT_ACTION_TOOL, ToolDescriptor,
};
use crate::json::JsonValue;
use crate::protocol_artifact_runtime_v2::{RUNTIME_V2_ACTION_ID, RUNTIME_V2_MAX_GENERATION};

const OPERATION_ID_PATTERN: &str = "^[A-Za-z0-9_.:/-]{1,128}$";
const ROUTE_OPERATION_ID_PATTERN: &str = "^[A-Za-z0-9_.:-]{1,128}$";

pub(super) fn build() -> super::ToolCatalog {
    let state_schema = state_schema();
    let action_schema = action_schema();
    let reconcile_schema = reconcile_schema();
    super::ToolCatalog {
        revision: String::from("runtime-v2-mcp"),
        capabilities: CapabilityCatalog::default(),
        tools: vec![
            ToolDescriptor {
                name: String::from(GET_STATE_TOOL),
                description: String::from(
                    "Read one bounded Runtime-v2 state snapshot through the authenticated gateway.",
                ),
                input_schema: state_schema,
            },
            ToolDescriptor {
                name: String::from(SUBMIT_ACTION_TOOL),
                description: String::from(
                    "Submit exactly one end_turn operation; accepted is admission only and settled requires a fresh observation witness.",
                ),
                input_schema: action_schema,
            },
            ToolDescriptor {
                name: String::from(RECONCILE_ACTION_TOOL),
                description: String::from(
                    "Reconcile one previously submitted end_turn operation by its stable operation_id.",
                ),
                input_schema: reconcile_schema,
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
                [
                    "instance_id",
                    "mcp_session_id",
                    "lease_id",
                    "lease_epoch",
                    "generation",
                ]
                .into_iter()
                .map(JsonValue::string)
                .collect(),
            ),
        ),
        (String::from("properties"), context_properties(false, false)),
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
                    "operation_id",
                    "action_id",
                ]
                .into_iter()
                .map(JsonValue::string)
                .collect(),
            ),
        ),
        (String::from("properties"), context_properties(true, true)),
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
                    "generation",
                    "operation_id",
                ]
                .into_iter()
                .map(JsonValue::string)
                .collect(),
            ),
        ),
        (String::from("properties"), context_properties(true, false)),
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
        (
            String::from("lease_epoch"),
            bounded_counter(RUNTIME_V2_MAX_GENERATION),
        ),
        (
            String::from("generation"),
            bounded_counter(RUNTIME_V2_MAX_GENERATION),
        ),
    ];
    if include_operation {
        properties.push((
            String::from("operation_id"),
            bounded_string(ROUTE_OPERATION_ID_PATTERN),
        ));
    }
    if include_action {
        properties.push((
            String::from("action_id"),
            JsonValue::object([
                (String::from("type"), JsonValue::string("string")),
                (
                    String::from("const"),
                    JsonValue::string(RUNTIME_V2_ACTION_ID),
                ),
            ]),
        ));
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

fn bounded_counter(maximum: i64) -> JsonValue {
    JsonValue::object([
        (String::from("type"), JsonValue::string("integer")),
        (String::from("minimum"), JsonValue::Number(0)),
        (String::from("maximum"), JsonValue::Number(maximum)),
    ])
}
