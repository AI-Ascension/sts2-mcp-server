// SPDX-License-Identifier: MIT

use crate::catalog::COOP_SYNCHRONIZATION_TOOL;
use crate::gateway::{Correlation, GatewayAdapter, GatewayError, GatewayMethod, GatewayRequest};
use crate::json::JsonValue;
use crate::protocol::{METHOD_NOT_FOUND, RpcError, RpcRequest, RpcResponse};
use crate::server::McpServer;

use super::{has_only_arguments, invalid_params, tool_result};

#[path = "mapping_coop_context.rs"]
mod context;
#[path = "projection_coop_synchronization.rs"]
mod projection;
use context::Context;

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
    let context = match Context::read(
        arguments,
        server.gateway_session_id(),
        server.mcp_session_id(),
        &request.id.stable_text(),
    ) {
        Ok(context) => context,
        Err(message) => return invalid_params(request.id, message),
    };
    let gateway_request = GatewayRequest {
        method: GatewayMethod::Get,
        path: format!("/v1/instances/{}/coop/synchronization", context.instance),
        headers: context.headers(),
        body: None,
        correlation: Correlation {
            mcp_session_id: context.mcp_session.clone(),
            mcp_request_id: request.id.clone(),
        },
    };
    match server.gateway.forward(gateway_request) {
        Ok(response) => match projection::project_response(&response.body, &context) {
            Ok(body) => tool_result(
                request.id,
                body.to_json(),
                !(200..300).contains(&response.status),
            ),
            Err(message) => tool_result(request.id, message, true),
        },
        Err(error) => tool_result(request.id, gateway_error(error), true),
    }
}

fn gateway_error(error: GatewayError) -> &'static str {
    match error {
        GatewayError::Unauthorized => "co-op gateway authorization failed",
        GatewayError::Forbidden => "co-op gateway scope authorization failed",
        GatewayError::NotFound => "co-op target was not found",
        GatewayError::Unavailable => "co-op gateway is unavailable or not configured",
        GatewayError::Timeout => "co-op synchronization timed out",
        GatewayError::MalformedResponse => "co-op gateway response was malformed",
        GatewayError::Rejected => "co-op gateway rejected synchronization",
    }
}
