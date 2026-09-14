// SPDX-License-Identifier: MIT
//! Pinned `game-information-query-v1` envelope and capability-shape builders
//! owned by the issue #51 acceptance producer.

use serde_json::{Map, Value, json};
use sts2_mcp_server::{
    GAME_INFORMATION_ARTIFACT, GAME_INFORMATION_GENERATOR, GAME_INFORMATION_PROTOCOL_VERSION,
    GAME_INFORMATION_SCHEMA_DIGEST, GAME_INFORMATION_SCHEMA_SOURCE,
};

use super::content::CARD_FIELDS;

pub(crate) fn provenance() -> Value {
    json!({
        "artifact": GAME_INFORMATION_ARTIFACT,
        "source": GAME_INFORMATION_SCHEMA_SOURCE,
        "generator": GAME_INFORMATION_GENERATOR,
    })
}

pub(crate) fn envelope(correlation: &str, kind: &str) -> Map<String, Value> {
    let mut root = Map::new();
    for (key, value) in [
        (
            "protocol_version",
            Value::from(GAME_INFORMATION_PROTOCOL_VERSION),
        ),
        ("schema_digest", Value::from(GAME_INFORMATION_SCHEMA_DIGEST)),
        ("correlation_id", Value::from(correlation)),
        ("kind", Value::from(kind)),
    ] {
        root.insert(String::from(key), value);
    }
    root.insert(String::from("provenance"), provenance());
    root.insert(String::from("query"), Value::Null);
    root.insert(String::from("result"), Value::Null);
    root.insert(String::from("capabilities"), Value::Null);
    root.insert(String::from("error"), Value::Null);
    root
}

pub(crate) fn query_envelope(correlation: &str, query: &Value, result: Value) -> Value {
    let mut root = envelope(correlation, "query_response");
    root.insert(String::from("query"), query.clone());
    root.insert(String::from("result"), result);
    Value::Object(root)
}

pub(crate) fn error_envelope(
    correlation: &str,
    code: &str,
    field: Option<&str>,
    reason: &str,
    retryable: bool,
) -> Value {
    let mut root = envelope(correlation, "error_response");
    root.insert(
        String::from("error"),
        json!({
            "code": code,
            "field": field,
            "reason": reason,
            "retryable": retryable,
        }),
    );
    Value::Object(root)
}

pub(crate) fn capabilities_envelope(correlation: &str) -> Value {
    let mut root = envelope(correlation, "capabilities_response");
    root.insert(
        String::from("capabilities"),
        json!({
            "profile": "game-information-query-v1",
            "query_kinds": ["availability", "detail", "get", "list", "search"],
            "entity_kinds": ["card", "relic"],
            "projections": ["full", "standard", "summary"],
            "detail_levels": ["full", "standard", "summary"],
            "fields": CARD_FIELDS,
            "limits": {
                "item_bytes": 262144,
                "page_bytes": 262144,
                "page_items": 128,
                "text_bytes": 65536,
            },
            "max_message_bytes": 262144,
            "max_cursor_bytes": 512,
            "snapshot_policy": {
                "supports_live": true,
                "lifetime_generations": 128,
                "max_retained_snapshots": 8,
                "expiry_behavior": "reject_stale_snapshot",
                "invalidated_by": [
                    "content_change",
                    "epoch_change",
                    "profile_change",
                    "restore",
                    "restart",
                    "run_change"
                ],
            },
        }),
    );
    Value::Object(root)
}

pub(crate) fn corrupt_digest(mut value: Value) -> Value {
    if let Value::Object(root) = &mut value {
        root.insert(String::from("schema_digest"), Value::from("d".repeat(64)));
    }
    value
}
