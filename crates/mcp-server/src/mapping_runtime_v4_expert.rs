// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::catalog::{EXPERT_ACTION_TOOL, EXPERT_RECONCILE_TOOL, EXPERT_STATE_TOOL};
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
const RECONCILE_ARGUMENTS: [&str; 5] = [
    "instance_id",
    "mcp_session_id",
    "lease_id",
    "lease_epoch",
    "operation_id",
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
    if !matches!(
        tool_name,
        EXPERT_STATE_TOOL | EXPERT_ACTION_TOOL | EXPERT_RECONCILE_TOOL
    ) || server.catalog.descriptor(tool_name).is_none()
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
        EXPERT_RECONCILE_TOOL => expert_reconcile_call(server, id, arguments, &correlation_id),
        _ => invalid_params(id, "Runtime-v4 expert tool is not active"),
    }
}

fn expert_reconcile_call<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    id: RequestId,
    arguments: &BTreeMap<String, JsonValue>,
    correlation_id: &str,
) -> RpcResponse {
    if !has_only_arguments(arguments, &RECONCILE_ARGUMENTS) {
        return invalid_params(
            id,
            "sts2.expert_reconcile arguments contain an unsupported field",
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
    let Some(operation_id) = arguments
        .get("operation_id")
        .and_then(JsonValue::as_string)
        .filter(|value| super::safe_header_value(value) && !value.contains('/'))
    else {
        return invalid_params(id, "operation_id must be a safe non-empty identity");
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
            "expert-reconcile identity is unsafe or not bound to this MCP session",
        );
    }
    let Some(lease_epoch) = super::nonnegative_integer(arguments, "lease_epoch")
        .filter(|value| *value <= 9_007_199_254_740_991)
    else {
        return invalid_params(id, "lease_epoch exceeds the protocol bound");
    };
    // Reconciliation is a bodyless, mutating-operation read. Carry every caller-supplied
    // authority field in headers so the runtime adapter can reject stale or foreign leases
    // before it opens a downstream connection. The adapter may inject its configured values
    // only after this admission check has accepted the request.
    let mut request_headers = headers(mcp_session_id, correlation_id);
    request_headers.insert(
        String::from("x-sts2-instance-id"),
        String::from(instance_id),
    );
    request_headers.insert(
        String::from("x-sts2-session-id"),
        String::from(server.gateway_session_id().unwrap_or(mcp_session_id)),
    );
    request_headers.insert(String::from("x-sts2-lease-id"), String::from(lease_id));
    request_headers.insert(String::from("x-sts2-lease-epoch"), lease_epoch.to_string());
    let request = GatewayRequest {
        method: GatewayMethod::Get,
        path: format!("/v4/instances/{instance_id}/expert-actions/{operation_id}"),
        headers: request_headers,
        body: None,
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

include!("mapping_runtime_v4_expert_dispatch.rs");
include!("mapping_runtime_v4_expert_state.rs");
