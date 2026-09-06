// SPDX-License-Identifier: MIT

use super::{CapabilityCatalog, MAX_IDENTIFIER_BYTES, ToolDescriptor};
use crate::json::JsonValue;

pub(super) const REVISION: &str = "runtime-v4-expert-mcp";
pub(super) const EXPERT_STATE_TOOL: &str = "sts2.expert_state";
const INSTANCE_ID_PATTERN: &str = "^[A-Za-z0-9_-]{1,128}$";
const SESSION_ID_PATTERN: &str = "^[A-Za-z0-9_.:/-]{1,128}$";

pub(super) fn build() -> super::ToolCatalog {
    super::ToolCatalog {
        revision: String::from(REVISION),
        capabilities: CapabilityCatalog::default(),
        tools: vec![ToolDescriptor {
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
        }],
    }
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
