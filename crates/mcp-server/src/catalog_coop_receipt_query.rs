// SPDX-License-Identifier: MIT

use super::{CapabilityCatalog, ToolDescriptor};
use crate::catalog::MAX_IDENTIFIER_BYTES;
use crate::json::JsonValue;

pub const REVISION: &str = "coop-receipt-query-v1-mcp";
pub const COOP_RECEIPT_QUERY_TOOL: &str = "sts2.coop_receipt_query";

pub(super) fn build() -> super::ToolCatalog {
    super::ToolCatalog {
        revision: String::from(REVISION),
        capabilities: CapabilityCatalog::default(),
        tools: vec![ToolDescriptor {
            name: String::from(COOP_RECEIPT_QUERY_TOOL),
            description: String::from(
                "Read one immutable retained co-op receipt by its original operation identity; this never observes, reconciles, retries, queues, or mutates the game.",
            ),
            input_schema: input_schema(),
        }],
    }
}

fn input_schema() -> JsonValue {
    const MAX_GENERATION: i64 = 9_007_199_254_740_991;
    let identity = |pattern: &str| {
        JsonValue::object([
            (String::from("type"), JsonValue::string("string")),
            (String::from("minLength"), JsonValue::Number(1)),
            (
                String::from("maxLength"),
                JsonValue::Number(MAX_IDENTIFIER_BYTES as i64),
            ),
            (String::from("pattern"), JsonValue::string(pattern)),
        ])
    };
    let digest = JsonValue::object([
        (String::from("type"), JsonValue::string("string")),
        (String::from("minLength"), JsonValue::Number(64)),
        (String::from("maxLength"), JsonValue::Number(64)),
        (String::from("pattern"), JsonValue::string("^[0-9a-f]{64}$")),
    ]);
    let counter = JsonValue::object([
        (String::from("type"), JsonValue::string("integer")),
        (String::from("minimum"), JsonValue::Number(0)),
        (String::from("maximum"), JsonValue::Number(MAX_GENERATION)),
    ]);
    let location = JsonValue::object([
        (String::from("type"), JsonValue::string("object")),
        (String::from("additionalProperties"), JsonValue::Bool(false)),
        (
            String::from("required"),
            JsonValue::Array(
                ["act_index", "room_id", "coord"]
                    .into_iter()
                    .map(JsonValue::string)
                    .collect(),
            ),
        ),
        (
            String::from("properties"),
            JsonValue::object([
                (
                    String::from("act_index"),
                    JsonValue::object([
                        (String::from("type"), JsonValue::string("integer")),
                        (
                            String::from("minimum"),
                            JsonValue::Number(i64::from(i32::MIN)),
                        ),
                        (
                            String::from("maximum"),
                            JsonValue::Number(i64::from(i32::MAX)),
                        ),
                    ]),
                ),
                (
                    String::from("room_id"),
                    JsonValue::object([(
                        String::from("anyOf"),
                        JsonValue::Array(vec![
                            JsonValue::object([
                                (String::from("type"), JsonValue::string("integer")),
                                (
                                    String::from("minimum"),
                                    JsonValue::Number(i64::from(i32::MIN)),
                                ),
                                (
                                    String::from("maximum"),
                                    JsonValue::Number(i64::from(i32::MAX)),
                                ),
                            ]),
                            JsonValue::object([(String::from("type"), JsonValue::string("null"))]),
                        ]),
                    )]),
                ),
                (
                    String::from("coord"),
                    JsonValue::object([(
                        String::from("anyOf"),
                        JsonValue::Array(vec![
                            JsonValue::object([
                                (String::from("type"), JsonValue::string("object")),
                                (String::from("additionalProperties"), JsonValue::Bool(false)),
                                (
                                    String::from("required"),
                                    JsonValue::Array(
                                        ["col", "row"].into_iter().map(JsonValue::string).collect(),
                                    ),
                                ),
                                (
                                    String::from("properties"),
                                    JsonValue::object([
                                        (
                                            String::from("col"),
                                            JsonValue::object([
                                                (
                                                    String::from("type"),
                                                    JsonValue::string("integer"),
                                                ),
                                                (
                                                    String::from("minimum"),
                                                    JsonValue::Number(i64::from(i32::MIN)),
                                                ),
                                                (
                                                    String::from("maximum"),
                                                    JsonValue::Number(i64::from(i32::MAX)),
                                                ),
                                            ]),
                                        ),
                                        (
                                            String::from("row"),
                                            JsonValue::object([
                                                (
                                                    String::from("type"),
                                                    JsonValue::string("integer"),
                                                ),
                                                (
                                                    String::from("minimum"),
                                                    JsonValue::Number(i64::from(i32::MIN)),
                                                ),
                                                (
                                                    String::from("maximum"),
                                                    JsonValue::Number(i64::from(i32::MAX)),
                                                ),
                                            ]),
                                        ),
                                    ]),
                                ),
                            ]),
                            JsonValue::object([(String::from("type"), JsonValue::string("null"))]),
                        ]),
                    )]),
                ),
            ]),
        ),
    ]);
    let properties = vec![
        (
            String::from("instance_id"),
            identity("^[A-Za-z0-9_-]{1,128}$"),
        ),
        (
            String::from("mcp_session_id"),
            identity("^[A-Za-z0-9_.:/-]{1,128}$"),
        ),
        (
            String::from("lease_id"),
            identity("^[A-Za-z0-9_.:/-]{1,128}$"),
        ),
        (String::from("lease_epoch"), counter.clone()),
        (
            String::from("operation_id"),
            identity("^[A-Za-z0-9_.:/-]{1,128}$"),
        ),
        (
            String::from("action_kind"),
            JsonValue::object([(
                String::from("enum"),
                JsonValue::Array(
                    ["end_turn", "play_card"]
                        .into_iter()
                        .map(JsonValue::string)
                        .collect(),
                ),
            )]),
        ),
        (String::from("action_fingerprint"), digest),
        (
            String::from("run_id"),
            identity("^[A-Za-z0-9_.:/-]{1,128}$"),
        ),
        (String::from("location"), location),
        (
            String::from("actor_id"),
            identity("^[A-Za-z0-9_.:/-]{1,128}$"),
        ),
        (
            String::from("authority_id"),
            identity("^[A-Za-z0-9_.:/-]{1,128}$"),
        ),
        (
            String::from("authority_epoch"),
            identity("^[A-Za-z0-9_.:/-]{1,128}$"),
        ),
        (String::from("expected_host_generation"), counter.clone()),
        (String::from("before_host_generation"), counter),
        (
            String::from("participant_ids"),
            JsonValue::object([
                (String::from("type"), JsonValue::string("array")),
                (String::from("minItems"), JsonValue::Number(2)),
                (String::from("maxItems"), JsonValue::Number(4)),
                (String::from("uniqueItems"), JsonValue::Bool(true)),
                (String::from("items"), identity("^[A-Za-z0-9_.:/-]{1,128}$")),
            ]),
        ),
    ];
    JsonValue::object([
        (String::from("type"), JsonValue::string("object")),
        (String::from("additionalProperties"), JsonValue::Bool(false)),
        (
            String::from("required"),
            JsonValue::Array(
                properties
                    .iter()
                    .map(|(key, _)| JsonValue::string(key))
                    .collect(),
            ),
        ),
        (String::from("properties"), JsonValue::object(properties)),
    ])
}
