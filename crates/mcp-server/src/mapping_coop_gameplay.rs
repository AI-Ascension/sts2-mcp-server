// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::catalog::COOP_SYNCHRONIZATION_TOOL;
use crate::gateway::{Correlation, GatewayAdapter, GatewayError, GatewayMethod, GatewayRequest};
use crate::json::JsonValue;
use crate::protocol::{METHOD_NOT_FOUND, RequestId, RpcError, RpcRequest, RpcResponse};
use crate::server::McpServer;

use super::{
    has_only_arguments, headers, invalid_params, safe_header_value, safe_segment, tool_result,
};

#[path = "projection_coop_gameplay.rs"]
mod projection;
use projection::project_response;

pub(super) fn tools_call<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    request: RpcRequest,
) -> RpcResponse {
    let Some(params) = request.params.as_object() else {
        return invalid_params(request.id, "tools/call params must be an object");
    };
    if !has_only_arguments(params, &["name", "arguments"]) {
        return invalid_params(request.id, "co-op tools/call has unsupported fields");
    }
    if params.get("name").and_then(JsonValue::as_string) != Some(COOP_SYNCHRONIZATION_TOOL) {
        return RpcResponse::failure(
            Some(request.id),
            RpcError::new(METHOD_NOT_FOUND, "co-op tool is not active"),
        );
    }
    let Some(arguments) = params.get("arguments").and_then(JsonValue::as_object) else {
        return invalid_params(request.id, "co-op tool arguments must be an object");
    };
    if !has_only_arguments(
        arguments,
        &[
            "instance_id",
            "mcp_session_id",
            "lease_id",
            "lease_epoch",
            "generation",
        ],
    ) {
        return invalid_params(request.id, "co-op arguments contain an unsupported field");
    }
    let Some(instance_id) = arguments.get("instance_id").and_then(JsonValue::as_string) else {
        return invalid_params(request.id, "instance_id is required");
    };
    let Some(mcp_session_id) = arguments
        .get("mcp_session_id")
        .and_then(JsonValue::as_string)
    else {
        return invalid_params(request.id, "mcp_session_id is required");
    };
    if server
        .mcp_session_id()
        .is_some_and(|expected| expected != mcp_session_id)
    {
        return invalid_params(
            request.id,
            "MCP session identity does not match the configured session",
        );
    }
    let session_id = server
        .gateway_session_id()
        .unwrap_or(mcp_session_id)
        .to_owned();
    let Some(lease_id) = arguments.get("lease_id").and_then(JsonValue::as_string) else {
        return invalid_params(request.id, "lease_id is required");
    };
    let Some(lease_epoch) = number(arguments, "lease_epoch") else {
        return invalid_params(request.id, "lease_epoch must be a nonnegative integer");
    };
    let Some(generation) = number(arguments, "generation") else {
        return invalid_params(request.id, "generation must be a nonnegative integer");
    };
    if lease_epoch > 9_007_199_254_740_991 || generation > 9_007_199_254_740_991 {
        return invalid_params(
            request.id,
            "co-op generation or lease epoch exceeds its bound",
        );
    }
    if !safe_segment(instance_id)
        || !safe_header_value(mcp_session_id)
        || !safe_header_value(&session_id)
        || !safe_header_value(lease_id)
    {
        return invalid_params(request.id, "co-op identity is unsafe or oversized");
    }
    let correlation_id = request.id.stable_text();
    if !safe_header_value(&correlation_id) {
        return invalid_params(request.id, "request identity is unsafe");
    }
    let gateway_request = GatewayRequest {
        method: GatewayMethod::Get,
        path: format!("/v1/instances/{instance_id}/coop/synchronization"),
        headers: headers(mcp_session_id, &correlation_id),
        body: None,
        correlation: Correlation {
            mcp_session_id: String::from(mcp_session_id),
            mcp_request_id: request.id.clone(),
        },
    };
    match server.gateway.forward(gateway_request) {
        Ok(response) => match project_response(
            &response.body,
            &correlation_id,
            instance_id,
            &session_id,
            lease_id,
            lease_epoch,
            generation,
        ) {
            Ok(body) => tool_result(
                request.id,
                body.to_json(),
                !(200..300).contains(&response.status),
            ),
            Err(message) => tool_result(request.id, message, true),
        },
        Err(error) => gateway_error(request.id, error),
    }
}

fn number(arguments: &BTreeMap<String, JsonValue>, key: &str) -> Option<i64> {
    match arguments.get(key) {
        Some(JsonValue::Number(value)) if *value >= 0 => Some(*value),
        _ => None,
    }
}

fn gateway_error(id: RequestId, error: GatewayError) -> RpcResponse {
    let text = match error {
        GatewayError::Unauthorized => "co-op gateway authorization failed",
        GatewayError::NotFound => "co-op target was not found",
        GatewayError::Unavailable => "co-op gateway is unavailable",
        GatewayError::Timeout => "co-op synchronization timed out",
        GatewayError::MalformedResponse => "co-op gateway response was malformed",
        GatewayError::Rejected => "co-op gateway rejected synchronization",
    };
    tool_result(id, text, true)
}
