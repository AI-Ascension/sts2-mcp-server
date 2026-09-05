// SPDX-License-Identifier: MIT

use super::{CapabilityCatalog, MAX_IDENTIFIER_BYTES, ToolDescriptor};
use crate::json::JsonValue;

const REVISION: &str = "runtime-v3-gameplay-mcp";
const MAX_GENERATION: i64 = 9_007_199_254_740_991;
const MAX_WAIT_MILLIS: i64 = 120_000;
const IDENTITY_PATTERN: &str = "^[A-Za-z0-9_.:/-]{1,512}$";
const SEGMENT_PATTERN: &str = "^[A-Za-z0-9_-]{1,128}$";

pub(super) fn build() -> super::ToolCatalog {
    super::ToolCatalog {
        revision: String::from(REVISION),
        capabilities: CapabilityCatalog::default(),
        tools: vec![
            ToolDescriptor {
                name: String::from("sts2.observe"),
                description: String::from(
                    "Read one bounded fair-play GameObservation through the authenticated gateway.",
                ),
                input_schema: context_schema(&[], &[]),
            },
            ToolDescriptor {
                name: String::from("sts2.legal_actions"),
                description: String::from(
                    "Read the complete host-generated LegalAction set for one observation generation.",
                ),
                input_schema: context_schema(&["state_id"], &[]),
            },
            ToolDescriptor {
                name: String::from("sts2.dispatch_action"),
                description: String::from(
                    "Dispatch exactly one current typed LegalAction with an idempotency identity.",
                ),
                input_schema: context_schema(&["state_id", "operation_id", "action"], &[]),
            },
            ToolDescriptor {
                name: String::from("sts2.wait_for_transition"),
                description: String::from(
                    "Wait for a semantic successor, same-state mutation, or bounded timeout.",
                ),
                input_schema: context_schema(&["operation_id", "wait_for_millis"], &[]),
            },
            ToolDescriptor {
                name: String::from("sts2.reobserve"),
                description: String::from(
                    "Obtain a fresh ordinary observation after a stale or contradictory result.",
                ),
                input_schema: context_schema(&[], &[]),
            },
            ToolDescriptor {
                name: String::from("sts2.recover"),
                description: String::from(
                    "Perform only an explicitly safe recovery operation; strategic actions are not accepted.",
                ),
                input_schema: context_schema(&["recovery_kind"], &["operation_id"]),
            },
        ],
    }
}

fn context_schema(required_extra: &[&str], optional_extra: &[&str]) -> JsonValue {
    let mut required = vec![
        JsonValue::string("instance_id"),
        JsonValue::string("mcp_session_id"),
        JsonValue::string("lease_id"),
        JsonValue::string("lease_epoch"),
        JsonValue::string("generation"),
    ];
    required.extend(required_extra.iter().map(|key| JsonValue::string(*key)));
    let mut properties = vec![
        (String::from("instance_id"), bounded_string(SEGMENT_PATTERN)),
        (
            String::from("mcp_session_id"),
            bounded_string(IDENTITY_PATTERN),
        ),
        (String::from("lease_id"), bounded_string(IDENTITY_PATTERN)),
        (String::from("lease_epoch"), bounded_counter(MAX_GENERATION)),
        (String::from("generation"), bounded_counter(MAX_GENERATION)),
    ];
    for key in required_extra {
        properties.push((String::from(*key), schema_for(key)));
    }
    for key in optional_extra {
        let schema = if *key == "operation_id" {
            JsonValue::object([(
                String::from("anyOf"),
                JsonValue::Array(vec![
                    payload_identity(),
                    JsonValue::object([(String::from("type"), JsonValue::string("null"))]),
                ]),
            )])
        } else {
            schema_for(key)
        };
        properties.push((String::from(*key), schema));
    }
    JsonValue::object([
        (String::from("type"), JsonValue::string("object")),
        (String::from("additionalProperties"), JsonValue::Bool(false)),
        (String::from("required"), JsonValue::Array(required)),
        (String::from("properties"), JsonValue::object(properties)),
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

fn payload_identity() -> JsonValue {
    JsonValue::object([
        (String::from("type"), JsonValue::string("string")),
        (String::from("minLength"), JsonValue::Number(1)),
        (String::from("maxLength"), JsonValue::Number(512)),
        (String::from("pattern"), JsonValue::string(IDENTITY_PATTERN)),
    ])
}

fn bounded_counter(maximum: i64) -> JsonValue {
    JsonValue::object([
        (String::from("type"), JsonValue::string("integer")),
        (String::from("minimum"), JsonValue::Number(0)),
        (String::from("maximum"), JsonValue::Number(maximum)),
    ])
}

fn legal_action_schema() -> JsonValue {
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
                (String::from("action_id"), payload_identity()),
                (String::from("action"), action_payload_schema()),
            ]),
        ),
    ])
}

fn action_payload_schema() -> JsonValue {
    let mut variants = vec![
        simple_action_schema("end_turn"),
        simple_action_schema("skip_reward"),
        simple_action_schema("rest"),
        simple_action_schema("confirm_victory"),
        simple_action_schema("save_quit"),
    ];
    for (kind, field) in [
        ("start_run", "character_id"),
        ("select_map_node", "node_id"),
        ("choose_reward", "reward_id"),
        ("shop_purchase", "item_id"),
        ("shop_remove", "card_id"),
        ("smith", "card_id"),
        ("select_card", "card_id"),
        ("event_choice", "choice_id"),
    ] {
        variants.push(one_argument_action_schema(kind, field));
    }
    variants.push(JsonValue::object([
        (String::from("type"), JsonValue::string("object")),
        (String::from("additionalProperties"), JsonValue::Bool(false)),
        (
            String::from("required"),
            JsonValue::Array(
                ["kind", "card_id", "target_id"]
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
                    JsonValue::object([(String::from("const"), JsonValue::string("play_card"))]),
                ),
                (String::from("card_id"), payload_identity()),
                (
                    String::from("target_id"),
                    JsonValue::object([(
                        String::from("anyOf"),
                        JsonValue::Array(vec![
                            payload_identity(),
                            JsonValue::object([(String::from("type"), JsonValue::string("null"))]),
                        ]),
                    )]),
                ),
            ]),
        ),
    ]));
    JsonValue::object([(String::from("oneOf"), JsonValue::Array(variants))])
}

fn simple_action_schema(kind: &str) -> JsonValue {
    JsonValue::object([
        (String::from("type"), JsonValue::string("object")),
        (String::from("additionalProperties"), JsonValue::Bool(false)),
        (
            String::from("required"),
            JsonValue::Array(vec![JsonValue::string("kind")]),
        ),
        (
            String::from("properties"),
            JsonValue::object([(
                String::from("kind"),
                JsonValue::object([(String::from("const"), JsonValue::string(kind))]),
            )]),
        ),
    ])
}

fn one_argument_action_schema(kind: &str, field: &str) -> JsonValue {
    JsonValue::object([
        (String::from("type"), JsonValue::string("object")),
        (String::from("additionalProperties"), JsonValue::Bool(false)),
        (
            String::from("required"),
            JsonValue::Array(vec![JsonValue::string("kind"), JsonValue::string(field)]),
        ),
        (
            String::from("properties"),
            JsonValue::object([
                (
                    String::from("kind"),
                    JsonValue::object([(String::from("const"), JsonValue::string(kind))]),
                ),
                (String::from(field), payload_identity()),
            ]),
        ),
    ])
}

fn schema_for(key: &str) -> JsonValue {
    match key {
        "state_id" | "operation_id" => payload_identity(),
        "action" => legal_action_schema(),
        "wait_for_millis" => JsonValue::object([
            (String::from("type"), JsonValue::string("integer")),
            (String::from("minimum"), JsonValue::Number(1)),
            (String::from("maximum"), JsonValue::Number(MAX_WAIT_MILLIS)),
        ]),
        "recovery_kind" => JsonValue::object([
            (String::from("type"), JsonValue::string("string")),
            (
                String::from("enum"),
                JsonValue::Array(
                    ["reobserve", "reconcile", "release_lease", "stop_episode"]
                        .into_iter()
                        .map(JsonValue::string)
                        .collect(),
                ),
            ),
        ]),
        _ => JsonValue::Null,
    }
}
