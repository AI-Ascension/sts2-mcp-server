// SPDX-License-Identifier: MIT

use super::{CapabilityCatalog, ToolDescriptor};
use crate::json::JsonValue;

pub(super) const REVISION: &str = "game-information-live-observation-bootstrap-v1-mcp";
pub(super) const TOOL: &str = "sts2.game_information.live_observation_bootstrap";

pub(super) fn is_tool(name: &str) -> bool {
    name == TOOL
}

pub(super) fn build() -> super::ToolCatalog {
    super::ToolCatalog {
        revision: String::from(REVISION),
        capabilities: CapabilityCatalog::default(),
        tools: vec![ToolDescriptor {
            name: String::from(TOOL),
            description: String::from(
                "Bootstrap one bounded live game-information observation through the negotiated Gateway route. This is read-only and never proxies arbitrary paths.",
            ),
            input_schema: input_schema(),
        }],
        composition: None,
    }
}

fn identity(pattern: &str) -> JsonValue {
    JsonValue::object([
        ("type".to_owned(), JsonValue::string("string")),
        ("minLength".to_owned(), JsonValue::Number(1)),
        ("maxLength".to_owned(), JsonValue::Number(128)),
        ("pattern".to_owned(), JsonValue::string(pattern)),
    ])
}

fn counter(minimum: i64, maximum: i64) -> JsonValue {
    JsonValue::object([
        ("type".to_owned(), JsonValue::string("integer")),
        ("minimum".to_owned(), JsonValue::Number(minimum)),
        ("maximum".to_owned(), JsonValue::Number(maximum)),
    ])
}

fn definition_ref() -> JsonValue {
    JsonValue::object([
        ("type".to_owned(), JsonValue::string("object")),
        ("additionalProperties".to_owned(), JsonValue::Bool(false)),
        (
            "required".to_owned(),
            JsonValue::Array(
                [
                    "content_manifest_id",
                    "entity_kind",
                    "namespaced_id",
                    "variant",
                ]
                .into_iter()
                .map(JsonValue::string)
                .collect(),
            ),
        ),
        (
            "properties".to_owned(),
            JsonValue::object([
                (
                    "content_manifest_id".to_owned(),
                    identity("^[A-Za-z0-9._:/-]+$"),
                ),
                (
                    "entity_kind".to_owned(),
                    JsonValue::object([(
                        "enum".to_owned(),
                        JsonValue::Array(
                            [
                                "card",
                                "character",
                                "enemy",
                                "event",
                                "map_node",
                                "potion",
                                "power",
                                "relic",
                                "room",
                                "status",
                            ]
                            .into_iter()
                            .map(JsonValue::string)
                            .collect(),
                        ),
                    )]),
                ),
                ("namespaced_id".to_owned(), identity("^[A-Za-z0-9._:/-]+$")),
                (
                    "variant".to_owned(),
                    JsonValue::object([(
                        "anyOf".to_owned(),
                        JsonValue::Array(vec![
                            identity("^[A-Za-z0-9._:/-]+$"),
                            JsonValue::object([("type".to_owned(), JsonValue::string("null"))]),
                        ]),
                    )]),
                ),
            ]),
        ),
    ])
}

fn instance_ref() -> JsonValue {
    JsonValue::object([
        ("type".to_owned(), JsonValue::string("object")),
        ("additionalProperties".to_owned(), JsonValue::Bool(false)),
        (
            "required".to_owned(),
            JsonValue::Array(
                ["instance_id", "run_id", "epoch", "entity_kind", "entity_id"]
                    .into_iter()
                    .map(JsonValue::string)
                    .collect(),
            ),
        ),
        (
            "properties".to_owned(),
            JsonValue::object([
                ("instance_id".to_owned(), identity("^[A-Za-z0-9._:/-]+$")),
                ("run_id".to_owned(), identity("^[A-Za-z0-9._:/-]+$")),
                ("epoch".to_owned(), counter(0, 9_007_199_254_740_991)),
                (
                    "entity_kind".to_owned(),
                    definition_ref()
                        .as_object()
                        .and_then(|o| o.get("properties"))
                        .and_then(JsonValue::as_object)
                        .and_then(|o| o.get("entity_kind"))
                        .cloned()
                        .unwrap_or(JsonValue::Null),
                ),
                ("entity_id".to_owned(), identity("^[A-Za-z0-9._:/-]+$")),
            ]),
        ),
    ])
}

fn nullable(value: JsonValue) -> JsonValue {
    JsonValue::object([(
        "anyOf".to_owned(),
        JsonValue::Array(vec![
            value,
            JsonValue::object([("type".to_owned(), JsonValue::string("null"))]),
        ]),
    )])
}

fn input_schema() -> JsonValue {
    let max = 9_007_199_254_740_991;
    let properties = vec![
        ("instance_id".to_owned(), identity("^[A-Za-z0-9_-]{1,128}$")),
        (
            "mcp_session_id".to_owned(),
            identity("^[A-Za-z0-9_.:/-]{1,128}$"),
        ),
        ("lease_id".to_owned(), identity("^[A-Za-z0-9_.:/-]{1,128}$")),
        ("lease_epoch".to_owned(), counter(0, max)),
        ("run_id".to_owned(), identity("^[A-Za-z0-9._:/-]+$")),
        ("authority_epoch".to_owned(), counter(1, max)),
        (
            "content_manifest_id".to_owned(),
            identity("^[A-Za-z0-9._:/-]+$"),
        ),
        (
            "locale".to_owned(),
            JsonValue::object([
                ("type".to_owned(), JsonValue::string("string")),
                ("minLength".to_owned(), JsonValue::Number(2)),
                ("maxLength".to_owned(), JsonValue::Number(35)),
                (
                    "pattern".to_owned(),
                    JsonValue::string("^[A-Za-z]{2,3}(?:-[A-Za-z0-9]{2,8})*$"),
                ),
            ]),
        ),
        ("definition_ref".to_owned(), definition_ref()),
        ("instance_ref".to_owned(), nullable(instance_ref())),
        ("max_visible_entities".to_owned(), counter(1, 64)),
        ("max_item_bytes".to_owned(), counter(1, 65_536)),
        ("max_message_bytes".to_owned(), counter(1, 262_144)),
    ];
    JsonValue::object([
        ("type".to_owned(), JsonValue::string("object")),
        ("additionalProperties".to_owned(), JsonValue::Bool(false)),
        (
            "required".to_owned(),
            JsonValue::Array(
                properties
                    .iter()
                    .map(|(key, _)| JsonValue::string(key))
                    .collect(),
            ),
        ),
        (
            "properties".to_owned(),
            JsonValue::Object(properties.into_iter().collect()),
        ),
    ])
}
