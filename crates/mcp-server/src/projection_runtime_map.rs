// SPDX-License-Identifier: MIT

use std::collections::{BTreeMap, BTreeSet};

use crate::json::JsonValue;
use crate::protocol_artifact_runtime_map::{
    RUNTIME_MAP_V1_ARTIFACT, RUNTIME_MAP_V1_GENERATOR, RUNTIME_MAP_V1_MAX_BINDINGS,
    RUNTIME_MAP_V1_MAX_EDGES, RUNTIME_MAP_V1_MAX_GENERATION, RUNTIME_MAP_V1_MAX_HISTORY,
    RUNTIME_MAP_V1_MAX_NODES, RUNTIME_MAP_V1_PROTOCOL_VERSION, RUNTIME_MAP_V1_SCHEMA_DIGEST,
    RUNTIME_MAP_V1_SCHEMA_SOURCE,
};

#[path = "projection_runtime_map/shape.rs"]
mod shape;
use shape::{
    ROOT_FIELDS, SNAPSHOT_FIELDS, bounded_id_array, bounded_number, bounded_signed, enum_string,
    exact_object, has_cycle, optional_identity, optional_u32, require_identity, require_string,
    require_text, valid_text, validate_timeout,
};

const SNAPSHOT_SCHEMA_VERSION: &str = "visible-map-v1";
const MAX_ID_BYTES: usize = 128;
const MAX_HOST_ACTION_ID_BYTES: usize = 512;
const MAX_REASON_BYTES: usize = 256;
const MAX_TEXT_BYTES: usize = 128;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RuntimeMapProjectionContext {
    pub(crate) correlation_id: String,
    pub(crate) instance_id: String,
    pub(crate) session_id: String,
    pub(crate) lease_id: String,
    pub(crate) lease_epoch: i64,
    pub(crate) generation: i64,
}

pub(crate) fn project_runtime_map_gateway_body(
    body: &JsonValue,
    context: &RuntimeMapProjectionContext,
) -> Result<JsonValue, &'static str> {
    let object = exact_object(body, &ROOT_FIELDS, "Runtime-map response")?;
    require_string(object, "protocol_version", RUNTIME_MAP_V1_PROTOCOL_VERSION)?;
    require_string(object, "schema_digest", RUNTIME_MAP_V1_SCHEMA_DIGEST)?;
    let provenance = exact_object(
        object
            .get("provenance")
            .ok_or("Runtime-map provenance is missing")?,
        &["artifact", "source", "generator"],
        "Runtime-map provenance",
    )?;
    require_string(provenance, "artifact", RUNTIME_MAP_V1_ARTIFACT)?;
    require_string(provenance, "source", RUNTIME_MAP_V1_SCHEMA_SOURCE)?;
    require_string(provenance, "generator", RUNTIME_MAP_V1_GENERATOR)?;
    for (field, expected) in [
        ("correlation_id", context.correlation_id.as_str()),
        ("instance_id", context.instance_id.as_str()),
        ("session_id", context.session_id.as_str()),
        ("lease_id", context.lease_id.as_str()),
    ] {
        require_string(object, field, expected)?;
    }
    if bounded_number(object.get("lease_epoch"), RUNTIME_MAP_V1_MAX_GENERATION)?
        != context.lease_epoch
    {
        return Err("Runtime-map lease epoch does not match the request");
    }
    if bounded_number(object.get("generation"), RUNTIME_MAP_V1_MAX_GENERATION)?
        != context.generation
    {
        return Err("Runtime-map generation does not match the request");
    }
    require_string(object, "kind", "snapshot_response")?;
    validate_timeout(object.get("timeout"))?;
    let snapshot = validate_snapshot(object.get("snapshot"))?;
    if bounded_number(snapshot.get("generation"), RUNTIME_MAP_V1_MAX_GENERATION)?
        != context.generation
    {
        return Err("Runtime-map snapshot generation does not match the request");
    }
    Ok(JsonValue::object(
        ROOT_FIELDS
            .into_iter()
            .map(|field| {
                (
                    String::from(field),
                    object.get(field).cloned().unwrap_or(JsonValue::Null),
                )
            })
            .collect::<Vec<_>>(),
    ))
}

fn validate_snapshot(
    value: Option<&JsonValue>,
) -> Result<&std::collections::BTreeMap<String, JsonValue>, &'static str> {
    let object = exact_object(
        value.ok_or("Runtime-map snapshot is missing")?,
        &SNAPSHOT_FIELDS,
        "Runtime-map snapshot",
    )?;
    require_identity(object, "state_id", MAX_ID_BYTES)?;
    if bounded_number(object.get("generation"), RUNTIME_MAP_V1_MAX_GENERATION)?
        > RUNTIME_MAP_V1_MAX_GENERATION
    {
        return Err("Runtime-map snapshot generation is outside the bound");
    }
    require_string(object, "schema_version", SNAPSHOT_SCHEMA_VERSION)?;
    require_identity(object, "projection_version", MAX_ID_BYTES)?;
    require_text(object, "game_build", MAX_TEXT_BYTES)?;
    require_text(object, "mod_version", MAX_TEXT_BYTES)?;
    optional_identity(object, "map_instance_id", MAX_ID_BYTES)?;
    optional_u32(object, "act_id")?;
    optional_identity(object, "scope_id", MAX_ID_BYTES)?;

    let availability = enum_string(
        object,
        "availability",
        &["available", "unavailable", "not_observable", "unsupported"],
    )?;
    let completeness = enum_string(
        object,
        "completeness",
        &["complete", "incomplete", "unknown"],
    )?;
    enum_string(object, "freshness", &["current", "historical"])?;
    let reason_present = match object.get("reason") {
        Some(JsonValue::Null) => false,
        Some(JsonValue::String(reason)) if valid_text(reason, MAX_REASON_BYTES) => true,
        _ => return Err("Runtime-map reason is invalid"),
    };
    let available = availability == "available";
    let complete = completeness == "complete";
    if reason_present != (!available || !complete) || (!available && complete) {
        return Err("Runtime-map availability and completeness are inconsistent");
    }
    if available
        && (matches!(object.get("map_instance_id"), Some(JsonValue::Null))
            || matches!(object.get("act_id"), Some(JsonValue::Null))
            || matches!(object.get("scope_id"), Some(JsonValue::Null)))
    {
        return Err("Runtime-map available snapshots require map identity");
    }

    let nodes = object
        .get("nodes")
        .and_then(JsonValue::as_array)
        .ok_or("Runtime-map nodes must be an array")?;
    if nodes.len() > RUNTIME_MAP_V1_MAX_NODES {
        return Err("Runtime-map node collection exceeds the bound");
    }
    let mut node_ids = BTreeSet::new();
    let mut visited_nodes = BTreeSet::new();
    for node in nodes {
        let node = exact_object(
            node,
            &["id", "row", "column", "category", "visited"],
            "Runtime-map node",
        )?;
        let id = require_identity(node, "id", MAX_ID_BYTES)?;
        bounded_signed(node.get("row"), -32_768, 32_767)?;
        bounded_signed(node.get("column"), -32_768, 32_767)?;
        if !node_ids.insert(id.to_owned()) {
            return Err("Runtime-map node identity is duplicated");
        }
        if matches!(node.get("visited"), Some(JsonValue::Bool(true))) {
            visited_nodes.insert(id.to_owned());
        }
        enum_string(
            node,
            "category",
            &[
                "unknown", "start", "monster", "elite", "rest", "shop", "event", "treasure",
                "boss", "other",
            ],
        )?;
        if !matches!(node.get("visited"), Some(JsonValue::Bool(_))) {
            return Err("Runtime-map node visited flag is invalid");
        }
    }

    let edges = object
        .get("edges")
        .and_then(JsonValue::as_array)
        .ok_or("Runtime-map edges must be an array")?;
    if edges.len() > RUNTIME_MAP_V1_MAX_EDGES {
        return Err("Runtime-map edge collection exceeds the bound");
    }
    let mut edge_ids = BTreeSet::new();
    let mut adjacency = BTreeMap::<String, Vec<String>>::new();
    for id in &node_ids {
        adjacency.insert(id.clone(), Vec::new());
    }
    for edge in edges {
        let edge = exact_object(edge, &["from", "to"], "Runtime-map edge")?;
        let from = require_identity(edge, "from", MAX_ID_BYTES)?;
        let to = require_identity(edge, "to", MAX_ID_BYTES)?;
        if from == to
            || !node_ids.contains(from)
            || !node_ids.contains(to)
            || !edge_ids.insert((from, to))
        {
            return Err("Runtime-map edge relation is invalid");
        }
        adjacency
            .get_mut(from)
            .ok_or("Runtime-map edge source is unknown")?
            .push(to.to_owned());
    }
    if has_cycle(&adjacency) {
        return Err("Runtime-map graph contains a cycle");
    }

    let position_value = object
        .get("position")
        .ok_or("Runtime-map position is missing")?;
    let position_kind = position_value
        .as_object()
        .and_then(|position| position.get("kind"))
        .and_then(JsonValue::as_string);
    let position_fields: &[&str] = if position_kind == Some("current") {
        &["kind", "node_id"]
    } else {
        &["kind"]
    };
    let position = exact_object(position_value, position_fields, "Runtime-map position")?;
    let current_node = match position_kind {
        Some("current") => {
            let node_id = require_identity(position, "node_id", MAX_ID_BYTES)?;
            if !node_ids.contains(node_id) || !visited_nodes.contains(node_id) {
                return Err("Runtime-map current position is unknown");
            }
            Some(node_id)
        }
        Some("pre_start") => {
            if !object
                .get("history")
                .and_then(JsonValue::as_array)
                .is_some_and(Vec::is_empty)
            {
                return Err("Runtime-map pre-start position has history");
            }
            None
        }
        Some("unavailable") => None,
        _ => return Err("Runtime-map position kind is invalid"),
    };

    let history = bounded_id_array(object, "history", RUNTIME_MAP_V1_MAX_HISTORY, &node_ids)?;
    if history.iter().any(|id| !visited_nodes.contains(*id)) {
        return Err("Runtime-map history contains an unvisited node");
    }
    bounded_id_array(
        object,
        "terminal_node_ids",
        RUNTIME_MAP_V1_MAX_HISTORY,
        &node_ids,
    )?;

    let bindings = object
        .get("bindings")
        .and_then(JsonValue::as_array)
        .ok_or("Runtime-map bindings must be an array")?;
    if bindings.len() > RUNTIME_MAP_V1_MAX_BINDINGS {
        return Err("Runtime-map binding collection exceeds the bound");
    }
    let mut graph_binding_ids = BTreeSet::new();
    let mut host_action_ids = BTreeSet::new();
    let mut action_option_ids = BTreeSet::new();
    for binding in bindings {
        let binding = exact_object(
            binding,
            &["graph_node_id", "host_action_id", "action"],
            "Runtime-map binding",
        )?;
        let graph_node_id = require_identity(binding, "graph_node_id", MAX_ID_BYTES)?;
        let host_action_id = require_identity(binding, "host_action_id", MAX_HOST_ACTION_ID_BYTES)?;
        let action = exact_object(
            binding
                .get("action")
                .ok_or("Runtime-map binding action is missing")?,
            &["kind", "node_id"],
            "Runtime-map binding action",
        )?;
        let action_node_id = require_identity(action, "node_id", MAX_ID_BYTES)?;
        if action.get("kind").and_then(JsonValue::as_string) != Some("select_map_node")
            || !node_ids.contains(graph_node_id)
            || current_node == Some(graph_node_id)
            || host_action_id == graph_node_id
            || !graph_binding_ids.insert(graph_node_id)
            || !host_action_ids.insert(host_action_id)
            || !action_option_ids.insert(action_node_id)
        {
            return Err("Runtime-map binding is invalid");
        }
    }
    Ok(object)
}
