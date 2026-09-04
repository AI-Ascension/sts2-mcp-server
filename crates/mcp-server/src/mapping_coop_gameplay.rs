// SPDX-License-Identifier: MIT

use std::collections::{BTreeMap, BTreeSet};

use crate::catalog::COOP_SYNCHRONIZATION_TOOL;
use crate::gateway::{Correlation, GatewayAdapter, GatewayError, GatewayMethod, GatewayRequest};
use crate::json::JsonValue;
use crate::protocol::{INVALID_PARAMS, METHOD_NOT_FOUND, RequestId, RpcError, RpcRequest, RpcResponse};
use crate::server::McpServer;

use super::{has_only_arguments, headers, invalid_params, safe_header_value, safe_segment, tool_result};

const RESPONSE_FIELDS: [&str; 16] = [
    "protocol_version", "schema_digest", "provenance", "correlation_id", "instance_id",
    "session_id", "lease_id", "lease_epoch", "generation", "kind", "players",
    "local_action", "shared_vote", "shared_effect", "ally_target", "synchronization",
];

pub(super) fn tools_call<G: GatewayAdapter>(server: &mut McpServer<G>, request: RpcRequest) -> RpcResponse {
    let Some(params) = request.params.as_object() else { return invalid_params(request.id, "tools/call params must be an object"); };
    if !has_only_arguments(params, &["name", "arguments"]) { return invalid_params(request.id, "co-op tools/call has unsupported fields"); }
    if params.get("name").and_then(JsonValue::as_string) != Some(COOP_SYNCHRONIZATION_TOOL) {
        return RpcResponse::failure(Some(request.id), RpcError::new(METHOD_NOT_FOUND, "co-op tool is not active"));
    }
    let Some(arguments) = params.get("arguments").and_then(JsonValue::as_object) else { return invalid_params(request.id, "co-op tool arguments must be an object"); };
    if !has_only_arguments(arguments, &["instance_id", "mcp_session_id", "lease_id", "lease_epoch", "generation"]) {
        return invalid_params(request.id, "co-op arguments contain an unsupported field");
    }
    let Some(instance_id) = arguments.get("instance_id").and_then(JsonValue::as_string) else { return invalid_params(request.id, "instance_id is required"); };
    let Some(session_id) = arguments.get("mcp_session_id").and_then(JsonValue::as_string) else { return invalid_params(request.id, "mcp_session_id is required"); };
    let Some(lease_id) = arguments.get("lease_id").and_then(JsonValue::as_string) else { return invalid_params(request.id, "lease_id is required"); };
    let Some(lease_epoch) = number(arguments, "lease_epoch") else { return invalid_params(request.id, "lease_epoch must be a nonnegative integer"); };
    let Some(generation) = number(arguments, "generation") else { return invalid_params(request.id, "generation must be a nonnegative integer"); };
    if lease_epoch > 9_007_199_254_740_991 || generation > 9_007_199_254_740_991 {
        return invalid_params(request.id, "co-op generation or lease epoch exceeds its bound");
    }
    if !safe_segment(instance_id) || !safe_header_value(session_id) || !safe_header_value(lease_id) {
        return invalid_params(request.id, "co-op identity is unsafe or oversized");
    }
    let correlation_id = request.id.stable_text();
    if !safe_header_value(&correlation_id) { return invalid_params(request.id, "request identity is unsafe"); }
    let gateway_request = GatewayRequest {
        method: GatewayMethod::Get,
        path: format!("/v1/instances/{instance_id}/coop/synchronization"),
        headers: headers(session_id, &correlation_id),
        body: None,
        correlation: Correlation { mcp_session_id: String::from(session_id), mcp_request_id: request.id.clone() },
    };
    match server.gateway.forward(gateway_request) {
        Ok(response) => match project_response(&response.body, &correlation_id, instance_id, session_id, lease_id, lease_epoch, generation) {
            Ok(body) => tool_result(request.id, body.to_json(), !(200..300).contains(&response.status)),
            Err(message) => tool_result(request.id, message, true),
        },
        Err(error) => gateway_error(request.id, error),
    }
}

fn number(arguments: &BTreeMap<String, JsonValue>, key: &str) -> Option<i64> {
    match arguments.get(key) { Some(JsonValue::Number(value)) if *value >= 0 => Some(*value), _ => None }
}

fn project_response(body: &JsonValue, correlation: &str, instance: &str, session: &str, lease: &str, epoch: i64, generation: i64) -> Result<JsonValue, &'static str> {
    let object = body.as_object().ok_or("co-op synchronization response must be an object")?;
    if object.len() != RESPONSE_FIELDS.len() || RESPONSE_FIELDS.iter().any(|field| !object.contains_key(*field)) { return Err("co-op response has unknown or missing fields"); }
    if object.get("protocol_version").and_then(JsonValue::as_string) != Some("coop-gameplay-v1")
        || object.get("schema_digest").and_then(JsonValue::as_string) != Some("2c34d013315fbf2e16de03dbe2bd4c43d4c13c744292548cc46ea960af5e1fa2")
        || object.get("correlation_id").and_then(JsonValue::as_string) != Some(correlation)
        || object.get("instance_id").and_then(JsonValue::as_string) != Some(instance)
        || object.get("session_id").and_then(JsonValue::as_string) != Some(session)
        || object.get("lease_id").and_then(JsonValue::as_string) != Some(lease)
        || object.get("lease_epoch") != Some(&JsonValue::Number(epoch))
        || object.get("generation") != Some(&JsonValue::Number(generation))
        || object.get("kind").and_then(JsonValue::as_string) != Some("synchronization_response")
    { return Err("co-op response identity or metadata mismatched"); }
    let Some(provenance) = object.get("provenance").and_then(JsonValue::as_object) else { return Err("co-op provenance is missing"); };
    if provenance.len() != 3 || provenance.get("artifact").and_then(JsonValue::as_string) != Some("sts2-protocol/coop-gameplay-v1") || provenance.get("source").and_then(JsonValue::as_string) != Some("schemas/coop-gameplay-v1.schema.json") || provenance.get("generator").and_then(JsonValue::as_string) != Some("hand-authored") { return Err("co-op provenance is unsupported"); }
    let players = object.get("players").ok_or("co-op players are missing")?;
    let Some(sync) = object.get("synchronization").and_then(JsonValue::as_object) else { return Err("co-op synchronization is missing"); };
    for field in ["local_action", "shared_vote", "shared_effect", "ally_target"] {
        if !matches!(object.get(field), Some(JsonValue::Null)) {
            return Err("co-op synchronization response contains an unsupported payload");
        }
    }
    let peer_ids = valid_players(players)?;
    if !valid_sync(sync, generation, &peer_ids) { return Err("co-op synchronization is malformed"); }
    Ok(JsonValue::object([
        (String::from("protocol_version"), JsonValue::string("coop-gameplay-v1")),
        (String::from("schema_digest"), JsonValue::string("2c34d013315fbf2e16de03dbe2bd4c43d4c13c744292548cc46ea960af5e1fa2")),
        (String::from("provenance"), JsonValue::object([
            (String::from("artifact"), JsonValue::string("sts2-protocol/coop-gameplay-v1")),
            (String::from("source"), JsonValue::string("schemas/coop-gameplay-v1.schema.json")),
            (String::from("generator"), JsonValue::string("hand-authored")),
        ])),
        (String::from("correlation_id"), JsonValue::string(correlation)),
        (String::from("instance_id"), JsonValue::string(instance)),
        (String::from("session_id"), JsonValue::string(session)),
        (String::from("lease_id"), JsonValue::string(lease)),
        (String::from("lease_epoch"), JsonValue::Number(epoch)),
        (String::from("generation"), JsonValue::Number(generation)),
        (String::from("kind"), JsonValue::string("synchronization_response")),
        (String::from("players"), players.clone()),
        (String::from("local_action"), JsonValue::Null),
        (String::from("shared_vote"), JsonValue::Null),
        (String::from("shared_effect"), JsonValue::Null),
        (String::from("ally_target"), JsonValue::Null),
        (String::from("synchronization"), sync.clone()),
    ]))
}

fn valid_players(value: &JsonValue) -> Result<BTreeSet<String>, &'static str> {
    let Some(players) = (match value { JsonValue::Array(values) => Some(values), _ => None }) else { return Err("co-op players are malformed"); };
    if !(2..=4).contains(&players.len()) { return Err("co-op peer count is out of bounds"); }
    let mut ids = std::collections::BTreeSet::new();
    let mut locals = 0;
    for player in players {
        let object = player.as_object().ok_or("co-op player is malformed")?;
        if object.len() != 2 || !object.contains_key("peer_id") || !object.contains_key("role") { return Err("co-op player has unknown fields"); }
        let id = object.get("peer_id").and_then(JsonValue::as_string).ok_or("co-op peer ID is malformed")?;
        let role = object.get("role").and_then(JsonValue::as_string).ok_or("co-op peer role is malformed")?;
        if !safe_identity(id) || !matches!(role, "local" | "ally") || !ids.insert(id.to_owned()) { return Err("co-op peer identity is invalid"); }
        if role == "local" { locals += 1; }
    }
    if locals != 1 { return Err("co-op peer set must contain one local peer"); }
    Ok(ids)
}

fn valid_sync(sync: &BTreeMap<String, JsonValue>, generation: i64, peer_ids: &BTreeSet<String>) -> bool {
    if sync.len() != 4 || !["status", "generation", "peer_count", "missing_peers"].iter().all(|key| sync.contains_key(*key)) { return false; }
    let synchronized = sync.get("status").and_then(JsonValue::as_string) == Some("synchronized");
    matches!(sync.get("status").and_then(JsonValue::as_string), Some("synchronized" | "disagreement" | "disconnected"))
        && sync.get("generation") == Some(&JsonValue::Number(generation))
        && matches!(sync.get("peer_count"), Some(JsonValue::Number(value)) if (2..=4).contains(value))
        && match sync.get("missing_peers") {
            Some(JsonValue::Array(values)) if values.len() <= 4 => {
                let mut missing = BTreeSet::new();
                (!synchronized || values.is_empty())
                    && values.iter().all(|value| value.as_string().is_some_and(|id| safe_identity(id) && peer_ids.contains(id) && missing.insert(id.to_owned())))
            }
            _ => false,
        }
}

fn safe_identity(value: &str) -> bool {
    !value.is_empty() && value.len() <= 512 && value.bytes().all(|byte| byte.is_ascii_alphanumeric() || b"._:/-".contains(&byte))
}

fn gateway_error(id: RequestId, error: GatewayError) -> RpcResponse {
    let text = match error { GatewayError::Unauthorized => "co-op gateway authorization failed", GatewayError::NotFound => "co-op target was not found", GatewayError::Unavailable => "co-op gateway is unavailable", GatewayError::Timeout => "co-op synchronization timed out", GatewayError::MalformedResponse => "co-op gateway response was malformed", GatewayError::Rejected => "co-op gateway rejected synchronization" };
    tool_result(id, text, true)
}
