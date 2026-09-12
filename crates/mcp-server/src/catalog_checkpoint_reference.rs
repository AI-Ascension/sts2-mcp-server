// SPDX-License-Identifier: MIT

use super::{CapabilityCatalog, ToolDescriptor};
use crate::json::JsonValue;

pub(super) const REVISION: &str = "checkpoint-reference-v1-mcp";

pub(super) fn build() -> super::ToolCatalog {
    let mut catalog = super::runtime_v3_gameplay::build();
    catalog.revision = String::from(REVISION);
    catalog.tools.clear();
    catalog.tools.push(ToolDescriptor {
        name: String::from(super::CHECKPOINT_REFERENCE_TOOL),
        description: String::from(
            "Read the current public checkpoint reference; unavailable unless the selected producer supports it.",
        ),
        input_schema: context_schema(),
    });
    catalog.capabilities = CapabilityCatalog::default();
    catalog
}

fn context_schema() -> JsonValue {
    const MAX_IDENTIFIER: i64 = 128;
    const MAX_GENERATION: i64 = 9_007_199_254_740_991;
    const IDENTITY_PATTERN: &str = "^[A-Za-z0-9_.:/-]{1,512}$";
    const SEGMENT_PATTERN: &str = "^[A-Za-z0-9_-]{1,128}$";
    let bounded_string = |pattern: &str, maximum: i64| {
        JsonValue::object([
            (String::from("type"), JsonValue::string("string")),
            (String::from("minLength"), JsonValue::Number(1)),
            (String::from("maxLength"), JsonValue::Number(maximum)),
            (String::from("pattern"), JsonValue::string(pattern)),
        ])
    };
    let bounded_counter = JsonValue::object([
        (String::from("type"), JsonValue::string("integer")),
        (String::from("minimum"), JsonValue::Number(0)),
        (String::from("maximum"), JsonValue::Number(MAX_GENERATION)),
    ]);
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
                    "caller_id",
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
                    bounded_string(SEGMENT_PATTERN, MAX_IDENTIFIER),
                ),
                (
                    String::from("mcp_session_id"),
                    bounded_string(IDENTITY_PATTERN, MAX_IDENTIFIER),
                ),
                (
                    String::from("lease_id"),
                    bounded_string(IDENTITY_PATTERN, MAX_IDENTIFIER),
                ),
                (String::from("lease_epoch"), bounded_counter.clone()),
                (
                    String::from("caller_id"),
                    bounded_string(IDENTITY_PATTERN, MAX_IDENTIFIER),
                ),
            ]),
        ),
    ])
}
