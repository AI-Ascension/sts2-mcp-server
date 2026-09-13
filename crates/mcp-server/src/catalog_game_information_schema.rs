// SPDX-License-Identifier: MIT

use crate::json::JsonValue;

use super::ENTITY_KINDS;
use crate::catalog::MAX_IDENTIFIER_BYTES;

const FIELD_NAMES: [&str; 10] = [
    "amount",
    "cost",
    "description",
    "display_name",
    "flags",
    "owner",
    "position",
    "rarity",
    "source_id",
    "tags",
];

pub(super) fn query(required: &[&str]) -> JsonValue {
    let mut properties = common_properties();
    properties.extend([
        (String::from("content_manifest_id"), identity()),
        (String::from("locale"), locale()),
        (String::from("visibility_scope"), identity()),
        (String::from("entity_kind"), enum_values(&ENTITY_KINDS)),
        (
            String::from("projection"),
            enum_values(&["summary", "standard", "full"]),
        ),
        (
            String::from("detail_level"),
            enum_values(&["summary", "standard", "full"]),
        ),
        (
            String::from("fields"),
            array_schema(32, enum_values(&FIELD_NAMES)),
        ),
        (String::from("page_items"), bounded(1, 128)),
        (String::from("item_bytes"), bounded(1, 262_144)),
        (String::from("page_bytes"), bounded(1, 262_144)),
        (String::from("text_bytes"), bounded(1, 65_536)),
        (String::from("cursor"), nullable(cursor())),
        (String::from("display_name"), nullable(text())),
        (String::from("namespaced_ids"), array_schema(64, identity())),
        (
            String::from("definition_refs"),
            array_schema(64, definition_ref()),
        ),
        (String::from("instance_ids"), array_schema(64, identity())),
        (String::from("definition_ref"), nullable(definition_ref())),
        (String::from("instance_ref"), nullable(instance_ref())),
        (String::from("snapshot_ref"), nullable(snapshot_ref())),
        (
            String::from("parent_observation"),
            nullable(parent_observation()),
        ),
        (String::from("mode"), enum_values(&["static", "live"])),
    ]);
    let mut allowed = vec![
        "instance_id",
        "mcp_session_id",
        "lease_id",
        "lease_epoch",
        "content_manifest_id",
        "locale",
        "visibility_scope",
        "entity_kind",
        "projection",
        "detail_level",
        "fields",
        "page_items",
        "item_bytes",
        "page_bytes",
        "text_bytes",
        "cursor",
        "display_name",
        "namespaced_ids",
        "definition_refs",
        "instance_ids",
    ];
    if required.contains(&"definition_ref")
        || required.contains(&"instance_ref")
        || required.contains(&"mode")
    {
        allowed.push("definition_ref");
    }
    if required.contains(&"instance_ref") || required.contains(&"mode") {
        allowed.extend(["instance_ref", "snapshot_ref", "parent_observation"]);
    }
    if required.contains(&"mode") {
        allowed.push("mode");
    }
    properties.retain(|(key, _)| allowed.contains(&key.as_str()));
    object(required, properties)
}

pub(super) fn context(required: &[&str]) -> JsonValue {
    object(required, common_properties())
}

fn common_properties() -> Vec<(String, JsonValue)> {
    vec![
        (String::from("instance_id"), segment()),
        (String::from("mcp_session_id"), identity()),
        (String::from("lease_id"), identity()),
        (
            String::from("lease_epoch"),
            bounded(0, 9_007_199_254_740_991),
        ),
    ]
}

fn object<I, K>(required: &[&str], properties: I) -> JsonValue
where
    I: IntoIterator<Item = (K, JsonValue)>,
    K: Into<String>,
{
    JsonValue::object([
        (String::from("type"), JsonValue::string("object")),
        (String::from("additionalProperties"), JsonValue::Bool(false)),
        (
            String::from("required"),
            JsonValue::Array(required.iter().map(|key| JsonValue::string(*key)).collect()),
        ),
        (
            String::from("properties"),
            JsonValue::object(
                properties
                    .into_iter()
                    .map(|(key, value)| (key.into(), value)),
            ),
        ),
    ])
}

fn definition_ref() -> JsonValue {
    object(
        &[
            "content_manifest_id",
            "entity_kind",
            "namespaced_id",
            "variant",
        ],
        vec![
            ("content_manifest_id", identity()),
            ("entity_kind", enum_values(&ENTITY_KINDS)),
            ("namespaced_id", identity()),
            ("variant", nullable(identity())),
        ],
    )
}

fn instance_ref() -> JsonValue {
    object(
        &["instance_id", "run_id", "epoch", "entity_kind", "entity_id"],
        vec![
            ("instance_id", identity()),
            ("run_id", identity()),
            ("epoch", bounded(0, 9_007_199_254_740_991)),
            ("entity_kind", enum_values(&ENTITY_KINDS)),
            ("entity_id", identity()),
        ],
    )
}

fn snapshot_ref() -> JsonValue {
    object(
        &["snapshot_id", "instance_ref", "state_generation"],
        vec![
            ("snapshot_id", identity()),
            ("instance_ref", instance_ref()),
            ("state_generation", bounded(0, 9_007_199_254_740_991)),
        ],
    )
}

fn parent_observation() -> JsonValue {
    object(
        &["instance_ref", "snapshot_ref", "state_generation"],
        vec![
            ("instance_ref", instance_ref()),
            ("snapshot_ref", snapshot_ref()),
            ("state_generation", bounded(0, 9_007_199_254_740_991)),
        ],
    )
}

fn identity() -> JsonValue {
    bounded_string("^[A-Za-z0-9._:/-]+$", MAX_IDENTIFIER_BYTES)
}

fn segment() -> JsonValue {
    bounded_string("^[A-Za-z0-9_-]+$", MAX_IDENTIFIER_BYTES)
}

fn cursor() -> JsonValue {
    bounded_string("^[A-Za-z0-9._~:/+=-]+$", 512)
}

fn locale() -> JsonValue {
    bounded_string("^[A-Za-z]{2,3}(?:-[A-Za-z0-9]{2,8})*$", 35)
}

fn text() -> JsonValue {
    JsonValue::object([
        (String::from("type"), JsonValue::string("string")),
        (String::from("minLength"), JsonValue::Number(1)),
        (String::from("maxLength"), JsonValue::Number(1_024)),
        (
            String::from("not"),
            JsonValue::object([(
                String::from("pattern"),
                JsonValue::string("[\\u0000-\\u001F\\u007F-\\u009F]"),
            )]),
        ),
    ])
}

fn bounded(minimum: i64, maximum: i64) -> JsonValue {
    JsonValue::object([
        (String::from("type"), JsonValue::string("integer")),
        (String::from("minimum"), JsonValue::Number(minimum)),
        (String::from("maximum"), JsonValue::Number(maximum)),
    ])
}

fn bounded_string(pattern: &str, maximum: usize) -> JsonValue {
    JsonValue::object([
        (String::from("type"), JsonValue::string("string")),
        (String::from("minLength"), JsonValue::Number(1)),
        (String::from("maxLength"), JsonValue::Number(maximum as i64)),
        (String::from("pattern"), JsonValue::string(pattern)),
    ])
}

fn enum_values(values: &[&str]) -> JsonValue {
    JsonValue::object([(
        String::from("enum"),
        JsonValue::Array(
            values
                .iter()
                .map(|value| JsonValue::string(*value))
                .collect(),
        ),
    )])
}

fn nullable(value: JsonValue) -> JsonValue {
    JsonValue::object([(
        String::from("anyOf"),
        JsonValue::Array(vec![
            value,
            JsonValue::object([(String::from("type"), JsonValue::string("null"))]),
        ]),
    )])
}

fn array_schema(maximum: i64, item: JsonValue) -> JsonValue {
    JsonValue::object([
        (String::from("type"), JsonValue::string("array")),
        (String::from("minItems"), JsonValue::Number(0)),
        (String::from("maxItems"), JsonValue::Number(maximum)),
        (String::from("items"), item),
    ])
}
