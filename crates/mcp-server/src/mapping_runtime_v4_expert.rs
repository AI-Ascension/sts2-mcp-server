// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::catalog::{EXPERT_ACTION_TOOL, EXPERT_STATE_TOOL};
use crate::gateway::{Correlation, GatewayAdapter, GatewayMethod, GatewayRequest};
use crate::json::JsonValue;
use crate::protocol::{
    INVALID_PARAMS, METHOD_NOT_FOUND, RequestId, RpcError, RpcRequest, RpcResponse,
};
use crate::protocol_artifact_runtime_v4_expert::{
    RUNTIME_V4_EXPERT_ACTION_ARTIFACT, RUNTIME_V4_EXPERT_ACTION_GENERATOR,
    RUNTIME_V4_EXPERT_ACTION_PROTOCOL_VERSION, RUNTIME_V4_EXPERT_ACTION_SCHEMA_DIGEST,
    RUNTIME_V4_EXPERT_ACTION_SCHEMA_SOURCE,
};
use crate::server::McpServer;

use super::{gateway_error_result, has_only_arguments, headers, invalid_params, tool_result};

#[path = "mapping_runtime_v4_expert_helpers.rs"]
mod helpers;
use helpers::{expert_error_result, valid_potion_action};

const ARGUMENTS: [&str; 2] = ["instance_id", "mcp_session_id"];
const ACTION_ARGUMENTS: [&str; 8] = [
    "instance_id",
    "mcp_session_id",
    "lease_id",
    "lease_epoch",
    "generation",
    "state_id",
    "operation_id",
    "action",
];

pub(super) fn tools_call<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    request: RpcRequest,
) -> RpcResponse {
    let Some(params) = request.params.as_object() else {
        return RpcResponse::failure(
            Some(request.id),
            RpcError::new(INVALID_PARAMS, "tools/call params must be an object"),
        );
    };
    if !has_only_arguments(params, &["name", "arguments"]) {
        return invalid_params(request.id, "tools/call params contain an unsupported field");
    }
    let Some(tool_name) = params.get("name").and_then(JsonValue::as_string) else {
        return invalid_params(request.id, "tools/call requires a tool name");
    };
    if !matches!(tool_name, EXPERT_STATE_TOOL | EXPERT_ACTION_TOOL)
        || server.catalog.descriptor(tool_name).is_none()
    {
        return RpcResponse::failure(
            Some(request.id),
            RpcError::new(
                METHOD_NOT_FOUND,
                "tool is not in the active Runtime-v4 catalog",
            ),
        );
    }
    let Some(arguments) = params.get("arguments").and_then(JsonValue::as_object) else {
        return invalid_params(request.id, "tools/call arguments must be an object");
    };
    let id = request.id;
    let correlation_id = id.stable_text();
    if !super::safe_header_value(&correlation_id) {
        return invalid_params(
            id,
            "request id contains an unsafe or oversized header value",
        );
    }
    match tool_name {
        EXPERT_STATE_TOOL => expert_state_call(server, id, arguments, &correlation_id),
        EXPERT_ACTION_TOOL => expert_action_call(server, id, arguments, &correlation_id),
        _ => invalid_params(id, "Runtime-v4 expert tool is not active"),
    }
}

fn expert_action_call<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    id: RequestId,
    arguments: &BTreeMap<String, JsonValue>,
    correlation_id: &str,
) -> RpcResponse {
    if !has_only_arguments(arguments, &ACTION_ARGUMENTS) {
        return invalid_params(
            id,
            "sts2.expert_action arguments contain an unsupported field",
        );
    }
    let Some(instance_id) = arguments.get("instance_id").and_then(JsonValue::as_string) else {
        return invalid_params(id, "instance_id must be a non-empty string");
    };
    let Some(mcp_session_id) = arguments
        .get("mcp_session_id")
        .and_then(JsonValue::as_string)
    else {
        return invalid_params(id, "mcp_session_id must be a non-empty string");
    };
    let Some(lease_id) = arguments.get("lease_id").and_then(JsonValue::as_string) else {
        return invalid_params(id, "lease_id must be a non-empty string");
    };
    if !super::safe_segment(instance_id)
        || !super::safe_header_value(mcp_session_id)
        || !super::safe_header_value(lease_id)
        || server
            .mcp_session_id()
            .is_some_and(|expected| expected != mcp_session_id)
    {
        return invalid_params(
            id,
            "expert-action identity is unsafe or not bound to this MCP session",
        );
    }
    let Some(lease_epoch) = super::nonnegative_integer(arguments, "lease_epoch")
        .filter(|value| *value <= 9_007_199_254_740_991)
    else {
        return invalid_params(id, "lease_epoch exceeds the protocol bound");
    };
    let Some(generation) = super::nonnegative_integer(arguments, "generation")
        .filter(|value| *value <= 9_007_199_254_740_991)
    else {
        return invalid_params(id, "generation exceeds the protocol bound");
    };
    let Some(state_id) = arguments
        .get("state_id")
        .and_then(JsonValue::as_string)
        .filter(|value| super::safe_header_value(value))
    else {
        return invalid_params(id, "state_id must be a safe non-empty identity");
    };
    let Some(operation_id) = arguments
        .get("operation_id")
        .and_then(JsonValue::as_string)
        .filter(|value| super::safe_header_value(value) && !value.contains('/'))
    else {
        return invalid_params(id, "operation_id must be a safe non-empty identity");
    };
    let Some(action) = arguments.get("action") else {
        return invalid_params(id, "action must be a host-generated potion legal action");
    };
    if !valid_potion_action(action) {
        return invalid_params(
            id,
            "action must contain exactly one use_potion legal action",
        );
    }
    let body = JsonValue::object([
        (
            "protocol_version".into(),
            JsonValue::string(RUNTIME_V4_EXPERT_ACTION_PROTOCOL_VERSION),
        ),
        (
            "schema_digest".into(),
            JsonValue::string(RUNTIME_V4_EXPERT_ACTION_SCHEMA_DIGEST),
        ),
        (
            "provenance".into(),
            JsonValue::object([
                (
                    "artifact".into(),
                    JsonValue::string(RUNTIME_V4_EXPERT_ACTION_ARTIFACT),
                ),
                (
                    "source".into(),
                    JsonValue::string(RUNTIME_V4_EXPERT_ACTION_SCHEMA_SOURCE),
                ),
                (
                    "generator".into(),
                    JsonValue::string(RUNTIME_V4_EXPERT_ACTION_GENERATOR),
                ),
            ]),
        ),
        ("profile".into(), JsonValue::string("expert-action")),
        ("correlation_id".into(), JsonValue::string(correlation_id)),
        ("instance_id".into(), JsonValue::string(instance_id)),
        (
            "session_id".into(),
            JsonValue::string(server.gateway_session_id().unwrap_or(mcp_session_id)),
        ),
        ("lease_id".into(), JsonValue::string(lease_id)),
        ("lease_epoch".into(), JsonValue::Number(lease_epoch)),
        ("generation".into(), JsonValue::Number(generation)),
        ("state_id".into(), JsonValue::string(state_id)),
        ("operation_id".into(), JsonValue::string(operation_id)),
        ("kind".into(), JsonValue::string("action_request")),
        ("action".into(), action.clone()),
        ("status".into(), JsonValue::Null),
        ("observation".into(), JsonValue::Null),
        ("transition".into(), JsonValue::Null),
        ("error_code".into(), JsonValue::Null),
    ]);
    let request = GatewayRequest {
        method: GatewayMethod::Post,
        path: format!("/v4/instances/{instance_id}/expert-action"),
        headers: headers(mcp_session_id, correlation_id),
        body: Some(body),
        correlation: Correlation {
            mcp_session_id: String::from(mcp_session_id),
            mcp_request_id: id.clone(),
        },
    };
    match server.gateway.forward(request) {
        Ok(response) => expert_action_response(server, id, response),
        Err(error) => gateway_error_result(id, error),
    }
}

fn expert_action_response<G: GatewayAdapter>(
    _server: &mut McpServer<G>,
    id: RequestId,
    response: crate::gateway::GatewayResponse,
) -> RpcResponse {
    if !(response.status == 200 || response.status == 409 || response.status == 503)
        || crate::projection::project_runtime_v4_expert_action_gateway_body(&response.body).is_err()
    {
        return expert_error_result(id, response.status, &response.body);
    }
    tool_result(id, response.body.to_json(), response.status != 200)
}

fn expert_state_call<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    id: RequestId,
    arguments: &BTreeMap<String, JsonValue>,
    correlation_id: &str,
) -> RpcResponse {
    if !has_only_arguments(arguments, &ARGUMENTS) {
        return invalid_params(
            id,
            "sts2.expert_state arguments contain an unsupported field",
        );
    }
    let Some(instance_id) = arguments.get("instance_id").and_then(JsonValue::as_string) else {
        return invalid_params(id, "instance_id must be a non-empty string");
    };
    let Some(mcp_session_id) = arguments
        .get("mcp_session_id")
        .and_then(JsonValue::as_string)
    else {
        return invalid_params(id, "mcp_session_id must be a non-empty string");
    };
    if !super::safe_segment(instance_id) || !super::safe_header_value(mcp_session_id) {
        return invalid_params(id, "expert-state identity is unsafe or oversized");
    }
    if server
        .mcp_session_id()
        .is_some_and(|expected| expected != mcp_session_id)
    {
        return invalid_params(
            id,
            "MCP session identity does not match the configured session",
        );
    }
    let request = GatewayRequest {
        method: GatewayMethod::Get,
        path: format!("/v4/instances/{instance_id}/expert-state"),
        headers: headers(mcp_session_id, correlation_id),
        body: None,
        correlation: Correlation {
            mcp_session_id: String::from(mcp_session_id),
            mcp_request_id: id.clone(),
        },
    };
    match server.gateway.forward(request) {
        Ok(response) if (200..300).contains(&response.status) => {
            let Ok(body) =
                crate::projection::project_runtime_v4_expert_gateway_body(&response.body)
            else {
                return tool_result(
                    id,
                    "gateway response is not a valid Runtime-v4 expert state",
                    true,
                );
            };
            let text = body.to_json();
            if text.len() > super::response::RUNTIME_V4_EXPERT_MAX_RESPONSE_BYTES {
                return tool_result(
                    id,
                    "gateway returned an oversized expert-state response",
                    true,
                );
            }
            tool_result(id, text, false)
        }
        Ok(response) => expert_error_result(id, response.status, &response.body),
        Err(error) => gateway_error_result(id, error),
    }
}
