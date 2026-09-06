// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::catalog::EXPERT_STATE_TOOL;
use crate::gateway::{Correlation, GatewayAdapter, GatewayMethod, GatewayRequest};
use crate::json::JsonValue;
use crate::protocol::{
    INVALID_PARAMS, METHOD_NOT_FOUND, RequestId, RpcError, RpcRequest, RpcResponse,
};
use crate::server::McpServer;

use super::{gateway_error_result, has_only_arguments, headers, invalid_params, tool_result};

const ARGUMENTS: [&str; 2] = ["instance_id", "mcp_session_id"];

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
    if tool_name != EXPERT_STATE_TOOL || server.catalog.descriptor(tool_name).is_none() {
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
    expert_state_call(server, id, arguments, &correlation_id)
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

fn expert_error_result(id: RequestId, status: u16, body: &JsonValue) -> RpcResponse {
    let Some(object) = body.as_object() else {
        return tool_result(
            id,
            format!("gateway returned Runtime-v4 status {status}"),
            true,
        );
    };
    if object.len() != 1
        || !matches!(object.get("error_code"), Some(JsonValue::String(value))
            if !value.is_empty()
                && value.len() <= 128
                && value.bytes().all(|byte| byte.is_ascii_alphanumeric()
                    || matches!(byte, b'-' | b'_' | b'.' | b':' | b'/')))
    {
        return tool_result(
            id,
            format!("gateway returned Runtime-v4 status {status}"),
            true,
        );
    }
    tool_result(id, body.to_json(), true)
}
