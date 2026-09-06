// SPDX-License-Identifier: MIT

use super::context::Context;
use crate::json::JsonValue;
use std::collections::{BTreeMap, BTreeSet};

const DIGEST: &str = "d410858cabbd38612345120c2196423130c7b21d788fd2b0d775cd82887087ec";
const FIELDS: [&str; 13] = [
    "protocol_version",
    "schema_digest",
    "provenance",
    "correlation_id",
    "instance_id",
    "session_id",
    "lease_id",
    "lease_epoch",
    "generation",
    "kind",
    "source",
    "players",
    "synchronization",
];

pub(super) fn project_response(
    body: &JsonValue,
    context: &Context,
) -> Result<JsonValue, &'static str> {
    let object = body.as_object().ok_or("co-op response must be an object")?;
    if !exact_fields(object, &FIELDS) || body.to_json().len() > 16 * 1024 {
        return Err("co-op response has unknown, missing, or oversized fields");
    }
    validate_metadata(object, context)?;
    let Some(JsonValue::Number(generation)) = object.get("generation") else {
        return Err("co-op generation must be an integer");
    };
    if !(0..=9_007_199_254_740_991).contains(generation) {
        return Err("co-op generation exceeds its bound");
    }
    let players = object.get("players").ok_or("co-op roster is missing")?;
    let peers = validate_players(players)?;
    let sync = object
        .get("synchronization")
        .and_then(JsonValue::as_object)
        .ok_or("co-op synchronization is missing")?;
    validate_sync(sync, *generation, &peers)?;
    // The complete closed response has been validated, including its evidence-source label.
    Ok(body.clone())
}

fn validate_metadata(
    object: &BTreeMap<String, JsonValue>,
    context: &Context,
) -> Result<(), &'static str> {
    for (field, expected) in [
        ("protocol_version", "coop-synchronization-v1"),
        ("schema_digest", DIGEST),
        ("kind", "synchronization_response"),
        ("source", "gateway_peer_reports"),
        ("correlation_id", &context.correlation),
        ("instance_id", &context.instance),
        ("session_id", &context.session),
        ("lease_id", &context.lease),
    ] {
        if object.get(field).and_then(JsonValue::as_string) != Some(expected) {
            return Err("co-op identity or metadata mismatched");
        }
    }
    if object.get("lease_epoch") != Some(&JsonValue::Number(context.epoch)) {
        return Err("co-op lease epoch mismatched");
    }
    let provenance = object
        .get("provenance")
        .and_then(JsonValue::as_object)
        .ok_or("co-op provenance is missing")?;
    if !exact_fields(provenance, &["artifact", "source", "generator"])
        || provenance.get("artifact").and_then(JsonValue::as_string)
            != Some("sts2-protocol/coop-synchronization-v1")
        || provenance.get("source").and_then(JsonValue::as_string)
            != Some("schemas/coop-synchronization-v1.schema.json")
        || provenance.get("generator").and_then(JsonValue::as_string) != Some("hand-authored")
    {
        return Err("co-op provenance is unsupported");
    }
    Ok(())
}

fn validate_players(value: &JsonValue) -> Result<BTreeSet<&str>, &'static str> {
    let JsonValue::Array(players) = value else {
        return Err("co-op roster must be an array");
    };
    if !(2..=4).contains(&players.len()) {
        return Err("co-op roster requires two to four peers");
    }
    let mut ids = BTreeSet::new();
    let mut locals = 0;
    for player in players {
        let object = player.as_object().ok_or("co-op peer must be an object")?;
        if !exact_fields(object, &["peer_id", "role"]) {
            return Err("co-op peer has unknown or missing fields");
        }
        let id = object
            .get("peer_id")
            .and_then(JsonValue::as_string)
            .ok_or("co-op peer ID is missing")?;
        let role = object
            .get("role")
            .and_then(JsonValue::as_string)
            .ok_or("co-op peer role is missing")?;
        if !identity(id) || !ids.insert(id) || !matches!(role, "local" | "ally") {
            return Err("co-op peer identity or role is invalid");
        }
        if role == "local" {
            locals += 1;
        }
    }
    if locals != 1 {
        return Err("co-op roster requires exactly one local peer");
    }
    Ok(ids)
}

fn validate_sync(
    sync: &BTreeMap<String, JsonValue>,
    generation: i64,
    peers: &BTreeSet<&str>,
) -> Result<(), &'static str> {
    if !exact_fields(
        sync,
        &["status", "generation", "peer_count", "missing_peers"],
    ) || sync.get("generation") != Some(&JsonValue::Number(generation))
        || sync.get("peer_count") != Some(&JsonValue::Number(peers.len() as i64))
    {
        return Err("co-op synchronization shape or generation is invalid");
    }
    let status = sync
        .get("status")
        .and_then(JsonValue::as_string)
        .ok_or("co-op status is missing")?;
    let Some(JsonValue::Array(missing)) = sync.get("missing_peers") else {
        return Err("co-op missing peers must be an array");
    };
    let mut unique = BTreeSet::new();
    if !matches!(status, "synchronized" | "disagreement" | "disconnected")
        || (status == "disconnected") == missing.is_empty()
        || missing.len() > peers.len()
        || missing.iter().any(|value| {
            value
                .as_string()
                .is_none_or(|id| !peers.contains(id) || !unique.insert(id))
        })
    {
        return Err("co-op status or missing peer set is invalid");
    }
    Ok(())
}

fn exact_fields(object: &BTreeMap<String, JsonValue>, fields: &[&str]) -> bool {
    object.len() == fields.len() && fields.iter().all(|field| object.contains_key(*field))
}

fn identity(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._:/-".contains(&byte))
}
