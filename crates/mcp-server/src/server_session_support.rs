// SPDX-License-Identifier: MIT

use crate::json::{JsonValue, parse};

pub(super) fn collect_snapshot_ids(
    value: &JsonValue,
    references: &mut Vec<String>,
) -> Result<(), &'static str> {
    match value {
        JsonValue::Object(object) => {
            if let Some(snapshot_id) = object.get("snapshot_id") {
                let snapshot_id = snapshot_id
                    .as_string()
                    .filter(|value| valid_snapshot_identity(value))
                    .ok_or("snapshot identity is empty, unsafe, or oversized")?;
                if !references.iter().any(|existing| existing == snapshot_id) {
                    references.push(snapshot_id.to_owned());
                }
            }
            for child in object.values() {
                collect_snapshot_ids(child, references)?;
            }
        }
        JsonValue::Array(values) => {
            for child in values {
                collect_snapshot_ids(child, references)?;
            }
        }
        _ => {}
    }
    Ok(())
}

pub(super) fn collect_snapshot_ids_from_response(
    value: &JsonValue,
    references: &mut Vec<String>,
) -> Result<(), &'static str> {
    match value {
        JsonValue::Object(object) => {
            collect_snapshot_ids(value, references)?;
            if let Some(JsonValue::String(text)) = object.get("text")
                && let Ok(parsed) = parse(text)
            {
                collect_snapshot_ids_from_response(&parsed, references)?;
            }
            for (key, child) in object {
                if key != "text" {
                    collect_snapshot_ids_from_response(child, references)?;
                }
            }
        }
        JsonValue::Array(values) => {
            collect_snapshot_ids(value, references)?;
            for child in values {
                collect_snapshot_ids_from_response(child, references)?;
            }
        }
        _ => {}
    }
    Ok(())
}

pub(super) fn catalog_revision_matches(
    composition: &crate::catalog::NegotiatedCapabilitySet,
    revision: &str,
) -> bool {
    composition.revision == revision
        || composition
            .operations()
            .any(|operation| operation.revision == revision)
}

pub(super) fn valid_snapshot_identity(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':' | b'/')
        })
}

pub(super) fn valid_revision_identity(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

pub(super) fn tools_changed_notification() -> String {
    JsonValue::object([
        ("jsonrpc".into(), JsonValue::string("2.0")),
        (
            "method".into(),
            JsonValue::string("notifications/tools/list_changed"),
        ),
        ("params".into(), JsonValue::object([])),
    ])
    .to_json()
}
