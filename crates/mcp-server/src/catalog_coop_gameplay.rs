// SPDX-License-Identifier: MIT

use super::{CapabilityCatalog, MAX_IDENTIFIER_BYTES, ToolDescriptor};
use crate::json::JsonValue;

pub(super) const REVISION: &str = "coop-gameplay-v1-mcp";
pub(super) const SYNC_TOOL: &str = "sts2.coop_synchronization";

pub(super) fn build() -> super::ToolCatalog {
    let identity = |pattern: &str| JsonValue::object([
        (String::from("type"), JsonValue::string("string")),
        (String::from("minLength"), JsonValue::Number(1)),
        (String::from("maxLength"), JsonValue::Number(MAX_IDENTIFIER_BYTES as i64)),
        (String::from("pattern"), JsonValue::string(pattern)),
    ]);
    let properties = [
        (String::from("instance_id"), identity("^[A-Za-z0-9_-]{1,128}$")),
        (String::from("mcp_session_id"), identity("^[A-Za-z0-9_.:/-]{1,128}$")),
        (String::from("lease_id"), identity("^[A-Za-z0-9_.:/-]{1,128}$")),
        (String::from("lease_epoch"), JsonValue::object([
            (String::from("type"), JsonValue::string("integer")),
            (String::from("minimum"), JsonValue::Number(0)),
            (String::from("maximum"), JsonValue::Number(9_007_199_254_740_991)),
        ])),
        (String::from("generation"), JsonValue::object([
            (String::from("type"), JsonValue::string("integer")),
            (String::from("minimum"), JsonValue::Number(0)),
            (String::from("maximum"), JsonValue::Number(9_007_199_254_740_991)),
        ])),
    ];
    let required = properties
        .iter()
        .map(|(key, _)| JsonValue::string(key.as_str()))
        .collect();
    super::ToolCatalog {
        revision: String::from(REVISION),
        capabilities: CapabilityCatalog::default(),
        tools: vec![ToolDescriptor {
            name: String::from(SYNC_TOOL),
            description: String::from("Read bounded co-op peer synchronization; disagreement suspends mutation."),
            input_schema: JsonValue::object([
                (String::from("type"), JsonValue::string("object")),
                (String::from("additionalProperties"), JsonValue::Bool(false)),
                (String::from("required"), JsonValue::Array(required)),
                (String::from("properties"), JsonValue::object(properties)),
            ]),
        }],
    }
}
