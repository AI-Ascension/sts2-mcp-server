// SPDX-License-Identifier: MIT

use super::{
    CapabilityCatalog, GET_STATE_TOOL, INSTANCE_ID_PATTERN, MAX_IDENTIFIER_BYTES,
    SESSION_ID_PATTERN, SUBMIT_ACTION_TOOL, ToolDescriptor,
};
use crate::json::JsonValue;

pub(super) fn build() -> super::ToolCatalog {
    let state_schema = JsonValue::object([
        ("type".to_owned(), JsonValue::string("object")),
        ("additionalProperties".to_owned(), JsonValue::Bool(false)),
        (
            "required".to_owned(),
            JsonValue::Array(vec![
                JsonValue::string("instance_id"),
                JsonValue::string("mcp_session_id"),
            ]),
        ),
        (
            "properties".to_owned(),
            JsonValue::object([
                (
                    "instance_id".to_owned(),
                    JsonValue::object([
                        ("type".to_owned(), JsonValue::string("string")),
                        ("minLength".to_owned(), JsonValue::Number(1)),
                        (
                            "maxLength".to_owned(),
                            JsonValue::Number(MAX_IDENTIFIER_BYTES as i64),
                        ),
                        ("pattern".to_owned(), JsonValue::string(INSTANCE_ID_PATTERN)),
                    ]),
                ),
                (
                    "mcp_session_id".to_owned(),
                    JsonValue::object([
                        ("type".to_owned(), JsonValue::string("string")),
                        ("minLength".to_owned(), JsonValue::Number(1)),
                        (
                            "maxLength".to_owned(),
                            JsonValue::Number(MAX_IDENTIFIER_BYTES as i64),
                        ),
                        ("pattern".to_owned(), JsonValue::string(SESSION_ID_PATTERN)),
                    ]),
                ),
            ]),
        ),
    ]);
    let action_schema = JsonValue::object([
        ("type".to_owned(), JsonValue::string("object")),
        ("additionalProperties".to_owned(), JsonValue::Bool(false)),
        (
            "required".to_owned(),
            JsonValue::Array(vec![
                JsonValue::string("instance_id"),
                JsonValue::string("mcp_session_id"),
                JsonValue::string("generation"),
                JsonValue::string("action_id"),
            ]),
        ),
        (
            "properties".to_owned(),
            JsonValue::object([
                (
                    "instance_id".to_owned(),
                    JsonValue::object([
                        ("type".to_owned(), JsonValue::string("string")),
                        ("minLength".to_owned(), JsonValue::Number(1)),
                        (
                            "maxLength".to_owned(),
                            JsonValue::Number(MAX_IDENTIFIER_BYTES as i64),
                        ),
                        ("pattern".to_owned(), JsonValue::string(INSTANCE_ID_PATTERN)),
                    ]),
                ),
                (
                    "mcp_session_id".to_owned(),
                    JsonValue::object([
                        ("type".to_owned(), JsonValue::string("string")),
                        ("minLength".to_owned(), JsonValue::Number(1)),
                        (
                            "maxLength".to_owned(),
                            JsonValue::Number(MAX_IDENTIFIER_BYTES as i64),
                        ),
                        ("pattern".to_owned(), JsonValue::string(SESSION_ID_PATTERN)),
                    ]),
                ),
                (
                    "generation".to_owned(),
                    JsonValue::object([
                        ("type".to_owned(), JsonValue::string("integer")),
                        ("minimum".to_owned(), JsonValue::Number(0)),
                    ]),
                ),
                (
                    "action_id".to_owned(),
                    JsonValue::object([
                        ("type".to_owned(), JsonValue::string("string")),
                        ("const".to_owned(), JsonValue::string("show_runtime_probe")),
                    ]),
                ),
            ]),
        ),
    ]);
    super::ToolCatalog {
        revision: String::from("runtime-v1-mcp"),
        capabilities: CapabilityCatalog::default(),
        tools: vec![
            ToolDescriptor {
                name: String::from(GET_STATE_TOOL),
                description: String::from(
                    "Read one bounded host observation through the authenticated runtime slice.",
                ),
                input_schema: state_schema,
            },
            ToolDescriptor {
                name: String::from(SUBMIT_ACTION_TOOL),
                description: String::from(
                    "Submit the safe host-visible show_runtime_probe action.",
                ),
                input_schema: action_schema,
            },
        ],
    }
}
