// SPDX-License-Identifier: MIT
//! Synthetic content manifest and item shapes owned by the issue #51
//! acceptance producer.
//!
//! Every value here is original synthetic test data. It is generated, never
//! copied from a reference implementation, game asset, save or golden fixture,
//! and it never leaves the process.

use serde_json::{Value, json};

use super::producer::{CONTENT_MANIFEST_ID, INSTANCE_ID, LEASE_EPOCH, LIVE_ENTITY_ID, RUN_ID};

/// Live-state amount: deliberately different from the static definition amount
/// so a live observation is distinguishable from static content.
pub(crate) const LIVE_AMOUNT: i64 = 3;
/// Fields the producer advertises and can serve for card definitions.
pub(crate) const CARD_FIELDS: [&str; 10] = [
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
/// Fields the producer can serve for relic definitions.
pub(crate) const RELIC_FIELDS: [&str; 5] =
    ["description", "display_name", "rarity", "source_id", "tags"];

#[derive(Clone, Copy)]
pub(crate) struct Definition {
    pub(crate) kind: &'static str,
    pub(crate) namespaced_id: &'static str,
    pub(crate) display_name: &'static str,
    pub(crate) cost: i64,
    pub(crate) amount: i64,
    pub(crate) rarity: &'static str,
}

pub(crate) const DEFINITIONS: [Definition; 5] = [
    Definition {
        kind: "card",
        namespaced_id: "synthetic:bash",
        display_name: "Bash",
        cost: 2,
        amount: 8,
        rarity: "common",
    },
    Definition {
        kind: "card",
        namespaced_id: "synthetic:defend",
        display_name: "Defend",
        cost: 1,
        amount: 5,
        rarity: "common",
    },
    Definition {
        kind: "card",
        namespaced_id: "synthetic:strike",
        display_name: "Strike",
        cost: 1,
        amount: 6,
        rarity: "common",
    },
    Definition {
        kind: "relic",
        namespaced_id: "synthetic:ember",
        display_name: "Ember",
        cost: 0,
        amount: 0,
        rarity: "uncommon",
    },
    Definition {
        kind: "relic",
        namespaced_id: "synthetic:lantern",
        display_name: "Lantern",
        cost: 0,
        amount: 0,
        rarity: "common",
    },
];

/// How one item reports its requested fields.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FieldPolicy {
    Available,
    AvailabilityProbe,
    Live,
}

pub(crate) fn requested_fields(query: &Value) -> Vec<String> {
    let mut names: Vec<String> = query
        .get("fields")
        .and_then(Value::as_array)
        .map(|fields| {
            fields
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

fn definition_ref(definition: &Definition) -> Value {
    json!({
        "content_manifest_id": CONTENT_MANIFEST_ID,
        "entity_kind": definition.kind,
        "namespaced_id": definition.namespaced_id,
        "variant": Value::Null,
    })
}

pub(crate) fn live_instance_ref() -> Value {
    json!({
        "instance_id": INSTANCE_ID,
        "run_id": RUN_ID,
        "epoch": LEASE_EPOCH,
        "entity_kind": "card",
        "entity_id": LIVE_ENTITY_ID,
    })
}

fn source(policy: FieldPolicy) -> (&'static str, &'static str) {
    if policy == FieldPolicy::Live {
        ("game_mod", INSTANCE_ID)
    } else {
        ("content_manifest", CONTENT_MANIFEST_ID)
    }
}

fn available(name: &str, kind: &str, value: Value, unit: Value, policy: FieldPolicy) -> Value {
    let (source_kind, reference) = source(policy);
    json!({
        "name": name,
        "kind": kind,
        "availability": "available",
        "value": value,
        "unit": unit,
        "source": {"kind": source_kind, "ref": reference},
        "reason": Value::Null,
    })
}

fn unavailable(name: &str, field_kind: &str, reason: &str, policy: FieldPolicy) -> Value {
    let (source_kind, reference) = source(policy);
    json!({
        "name": name,
        "kind": field_kind,
        "availability": "not_observable",
        "value": Value::Null,
        "unit": Value::Null,
        "source": {"kind": source_kind, "ref": reference},
        "reason": reason,
    })
}

fn definition_field(definition: &Definition, name: &str, policy: FieldPolicy) -> Value {
    if policy == FieldPolicy::AvailabilityProbe {
        return unavailable(
            name,
            "text",
            "field is not observable in the requested scope",
            policy,
        );
    }
    match name {
        "amount" if policy == FieldPolicy::Live => available(
            name,
            "integer",
            Value::from(LIVE_AMOUNT),
            Value::from("count"),
            policy,
        ),
        "amount" => available(
            name,
            "integer",
            Value::from(definition.amount),
            Value::from("count"),
            policy,
        ),
        "cost" => available(
            name,
            "integer",
            Value::from(definition.cost),
            Value::from("count"),
            policy,
        ),
        "position" => available(name, "integer", Value::from(0), Value::from("none"), policy),
        "display_name" => available(
            name,
            "text",
            Value::from(definition.display_name),
            Value::Null,
            policy,
        ),
        "description" if policy == FieldPolicy::Live => unavailable(
            name,
            "text",
            "description is not exposed in the player scope",
            policy,
        ),
        "description" => available(
            name,
            "text",
            Value::from(format!(
                "{} synthetic definition text",
                definition.display_name
            )),
            Value::Null,
            policy,
        ),
        "flags" => available(name, "boolean", Value::from(false), Value::Null, policy),
        "owner" => available(
            name,
            "text",
            Value::from("synthetic-owner"),
            Value::Null,
            policy,
        ),
        "rarity" => available(
            name,
            "text",
            Value::from(definition.rarity),
            Value::Null,
            policy,
        ),
        "source_id" => available(
            name,
            "text",
            Value::from(CONTENT_MANIFEST_ID),
            Value::Null,
            policy,
        ),
        "tags" => available(name, "text_list", json!(["synthetic"]), Value::Null, policy),
        _ => unavailable(name, "text", "field is not usable in this page", policy),
    }
}

pub(crate) fn definition_item(
    definition: &Definition,
    fields: &[String],
    policy: FieldPolicy,
    instance_ref: Value,
) -> Value {
    let built: Vec<Value> = fields
        .iter()
        .map(|name| definition_field(definition, name, policy))
        .collect();
    json!({
        "definition_ref": definition_ref(definition),
        "instance_ref": instance_ref,
        "fields": built,
    })
}

/// One padded page item: available integers plus a `text_list` that carries
/// the bytes used to drive a page to the pinned message bound.
pub(crate) fn padded_item(index: usize, tags: Option<Vec<String>>) -> Value {
    let mut fields = vec![
        available(
            "amount",
            "integer",
            Value::from(1),
            Value::from("count"),
            FieldPolicy::Available,
        ),
        available(
            "cost",
            "integer",
            Value::from(1),
            Value::from("count"),
            FieldPolicy::Available,
        ),
        unavailable(
            "description",
            "text",
            "page padding is not observable",
            FieldPolicy::Available,
        ),
        unavailable(
            "display_name",
            "text",
            "page padding is not observable",
            FieldPolicy::Available,
        ),
        available(
            "flags",
            "boolean",
            Value::from(false),
            Value::Null,
            FieldPolicy::Available,
        ),
        unavailable(
            "owner",
            "text",
            "page padding is not observable",
            FieldPolicy::Available,
        ),
        available(
            "position",
            "integer",
            Value::from(0),
            Value::from("none"),
            FieldPolicy::Available,
        ),
        unavailable(
            "rarity",
            "text",
            "page padding is not observable",
            FieldPolicy::Available,
        ),
        unavailable(
            "source_id",
            "text",
            "page padding is not observable",
            FieldPolicy::Available,
        ),
    ];
    match tags {
        Some(values) => fields.push(available(
            "tags",
            "text_list",
            Value::Array(values.into_iter().map(Value::from).collect()),
            Value::Null,
            FieldPolicy::Available,
        )),
        None => fields.push(unavailable(
            "tags",
            "text_list",
            "page padding is not observable",
            FieldPolicy::Available,
        )),
    }
    json!({
        "definition_ref": {
            "content_manifest_id": CONTENT_MANIFEST_ID,
            "entity_kind": "card",
            "namespaced_id": format!("synthetic:padded-{index:03}"),
            "variant": Value::Null,
        },
        "instance_ref": Value::Null,
        "fields": fields,
    })
}

fn text_bytes_of_field(field: &Value) -> usize {
    if field.get("availability").and_then(Value::as_str) != Some("available") {
        return 0;
    }
    match field.get("kind").and_then(Value::as_str) {
        Some("text") => field
            .get("value")
            .and_then(Value::as_str)
            .map_or(0, str::len),
        Some("text_list") => field
            .get("value")
            .and_then(Value::as_array)
            .map_or(0, |values| {
                values.iter().filter_map(Value::as_str).map(str::len).sum()
            }),
        _ => 0,
    }
}

pub(crate) fn text_bytes_of_item(item: &Value) -> usize {
    item.get("fields")
        .and_then(Value::as_array)
        .map_or(0, |fields| fields.iter().map(text_bytes_of_field).sum())
}
