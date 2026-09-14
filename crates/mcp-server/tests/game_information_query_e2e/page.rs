// SPDX-License-Identifier: MIT
//! Page shapes, cursor binding, snapshot fences and the deliberately oversized
//! page owned by the issue #51 acceptance producer.

use serde_json::{Value, json};

use super::content::{Definition, padded_item, text_bytes_of_item};
use super::producer::{INSTANCE_ID, LEASE_EPOCH, RUN_ID, SNAPSHOT_GENERATION, SNAPSHOT_ID};

/// Definitions carried by the padded oversized page.
pub(crate) const OVERSIZED_DEFINITIONS: usize = 128;
/// Target encoded size of the deliberately oversized page.
pub(crate) const OVERSIZED_PAGE_TARGET: usize = 261_900;
/// Bytes added to a page by one further 1024-byte `text_list` entry.
const ENTRY_STRIDE: usize = 1_027;
/// Largest accepted `text_list` entry.
const ENTRY_MAX: usize = 1_024;

/// Typed producer error: code, field, reason, retryable.
pub(crate) type ProducerError = (&'static str, Option<&'static str>, &'static str, bool);
pub(crate) const STALE_CURSOR: ProducerError = (
    "stale_cursor",
    Some("cursor"),
    "cursor binding no longer matches the requested query",
    false,
);
pub(crate) const STALE_SNAPSHOT: ProducerError = (
    "stale_snapshot",
    Some("snapshot_ref"),
    "retained snapshot or instance fence is stale",
    false,
);

pub(crate) fn query_limit(query: &Value, key: &str) -> Result<i64, ProducerError> {
    query
        .get("limits")
        .and_then(|limits| limits.get(key))
        .and_then(Value::as_i64)
        .ok_or((
            "invalid_bounds",
            Some("limits"),
            "query limits are missing or invalid",
            false,
        ))
}

pub(crate) fn filter_matches(query: &Value, definition: &Definition) -> bool {
    let filters = query.get("filters").unwrap_or(&Value::Null);
    if let Some(display_name) = filters.get("display_name").and_then(Value::as_str)
        && display_name != definition.display_name
    {
        return false;
    }
    for key in ["namespaced_ids", "instance_ids"] {
        let Some(values) = filters.get(key).and_then(Value::as_array) else {
            continue;
        };
        if !values.is_empty()
            && !values
                .iter()
                .any(|value| value.as_str() == Some(definition.namespaced_id))
        {
            return false;
        }
    }
    let refs = filters
        .get("definition_refs")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    refs.is_empty()
        || refs.iter().any(|value| {
            value.get("namespaced_id").and_then(Value::as_str) == Some(definition.namespaced_id)
        })
}

/// Opaque cursor bound to the normalized query: content, locale, scope, mode,
/// filters, bounds and ordering. Reuse across any binding change is refused as
/// `stale_cursor`.
fn cursor_digest(query: &Value) -> String {
    let mut binding = query.clone();
    if let Value::Object(object) = &mut binding {
        object.remove("cursor");
    }
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in binding.to_string().bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

pub(crate) fn cursor_for(query: &Value, entity_kind: &str, page: i64) -> String {
    format!("cursor:{entity_kind}:{page}:{}", cursor_digest(query))
}

pub(crate) fn cursor_page(
    query: &Value,
    entity_kind: &str,
    total: i64,
    page_items: i64,
) -> Result<i64, ProducerError> {
    let Some(cursor) = query.get("cursor").and_then(Value::as_str) else {
        return Ok(1);
    };
    let mut parts = cursor.split(':');
    let (Some("cursor"), Some(kind), Some(page), Some(digest), None) = (
        parts.next(),
        parts.next(),
        parts.next(),
        parts.next(),
        parts.next(),
    ) else {
        return Err(STALE_CURSOR);
    };
    if kind != entity_kind || digest != cursor_digest(query) {
        return Err(STALE_CURSOR);
    }
    let page: i64 = page.parse().map_err(|_| STALE_CURSOR)?;
    let step = page_items.max(1);
    let pages = (total.max(1) + step - 1) / step;
    if page < 2 || page > pages {
        return Err(STALE_CURSOR);
    }
    Ok(page)
}

pub(crate) fn query_without_cursor(query: &Value) -> Value {
    let mut value = query.clone();
    if let Value::Object(object) = &mut value {
        object.remove("cursor");
    }
    value
}

pub(crate) fn query_result(query: &Value, page: Value, generation: Value) -> Value {
    json!({
        "page": page,
        "result_generation": generation,
        "parent_observation": query.get("parent_observation").cloned().unwrap_or(Value::Null),
        "read_only": true,
    })
}

/// Builds one page body without its accounting member, so byte accounting is
/// reproducible and the padded page can be sized exactly.
pub(crate) fn page_body(
    query: &Value,
    items: Vec<Value>,
    ordering_key: &str,
    next_cursor: Option<String>,
    final_page: bool,
    total_count: i64,
) -> Value {
    json!({
        "next_cursor": next_cursor,
        "cursor_binding": if final_page { Value::Null } else { query_without_cursor(query) },
        "final_page": final_page,
        "total_count_known": true,
        "total_count": total_count,
        "coverage": "complete",
        "ordering": {
            "key": ordering_key,
            "direction": "ascending",
            "algorithm": "identity_bytes",
            "deterministic": true,
        },
        "items": items,
        "limits": query.get("limits").cloned().unwrap_or(Value::Null),
    })
}

/// Builds one page with reproducible canonical-JSON byte accounting.
pub(crate) fn page_value(
    query: &Value,
    items: Vec<Value>,
    ordering_key: &str,
    next_cursor: Option<String>,
    final_page: bool,
    total_count: i64,
) -> Value {
    let mut page = page_body(
        query,
        items,
        ordering_key,
        next_cursor,
        final_page,
        total_count,
    );
    let items = page
        .get("items")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let item_bytes = items
        .iter()
        .map(|item| item.to_string().len())
        .max()
        .unwrap_or(0);
    let payload_bytes = Value::Array(items.clone()).to_string().len();
    let text_bytes: usize = items.iter().map(text_bytes_of_item).sum();
    let accounting = json!({
        "item_count": items.len(),
        "item_bytes": item_bytes,
        "payload_bytes": payload_bytes,
        "page_bytes": page.to_string().len(),
        "text_bytes": text_bytes,
    });
    page.as_object_mut()
        .expect("page is an object")
        .insert(String::from("accounting"), accounting);
    page
}

fn padded_items(tags: Option<Vec<String>>) -> Vec<Value> {
    let mut items: Vec<Value> = (1..OVERSIZED_DEFINITIONS)
        .map(|index| padded_item(index, None))
        .collect();
    items.push(padded_item(OVERSIZED_DEFINITIONS, tags));
    items
}

/// A well-formed page whose envelope exceeds the pinned message bound, so the
/// MCP boundary must refuse it as an oversized result instead of projecting
/// a partial page.
pub(crate) fn oversized_page(query: &Value) -> Value {
    let total = OVERSIZED_DEFINITIONS as i64;
    let probe = padded_items(Some(vec![String::from("s")]));
    let base = page_body(query, probe, "definition_ref", None, true, total)
        .to_string()
        .len();
    for target in (OVERSIZED_PAGE_TARGET - 3..=OVERSIZED_PAGE_TARGET).rev() {
        if target <= base {
            continue;
        }
        let deficit = target - base;
        let full = deficit / ENTRY_STRIDE;
        let remainder = deficit % ENTRY_STRIDE;
        if full > 63 || remainder > ENTRY_MAX - 1 {
            continue;
        }
        let mut values = vec![String::from("s").repeat(ENTRY_MAX); full];
        values.push(String::from("s").repeat(remainder + 1));
        let items = padded_items(Some(values));
        assert_eq!(
            page_body(query, items.clone(), "definition_ref", None, true, total)
                .to_string()
                .len(),
            target,
            "oversized page target"
        );
        return query_result(
            query,
            page_value(query, items, "definition_ref", None, true, total),
            Value::Null,
        );
    }
    panic!("synthetic oversized page cannot reach the pinned message bound");
}

pub(crate) fn snapshot_is_coherent(binding: &Value) -> bool {
    let instance = binding.get("instance_ref").filter(|value| !value.is_null());
    let snapshot = binding.get("snapshot_ref").filter(|value| !value.is_null());
    let (Some(instance), Some(snapshot)) = (instance, snapshot) else {
        return false;
    };
    instance.get("instance_id").and_then(Value::as_str) == Some(INSTANCE_ID)
        && instance.get("run_id").and_then(Value::as_str) == Some(RUN_ID)
        && instance.get("epoch").and_then(Value::as_i64) == Some(LEASE_EPOCH)
        && snapshot.get("snapshot_id").and_then(Value::as_str) == Some(SNAPSHOT_ID)
        && snapshot.get("state_generation").and_then(Value::as_i64) == Some(SNAPSHOT_GENERATION)
}

pub(crate) fn snapshot_entity_id(binding: &Value) -> Option<&str> {
    binding
        .get("instance_ref")
        .and_then(|instance| instance.get("entity_id"))
        .and_then(Value::as_str)
}
