// SPDX-License-Identifier: MIT

use crate::gateway::{GatewayAdapter, GatewayError, GatewayRequest};
use crate::projection::project_coop_native_legal_catalog;
use crate::projection::project_coop_native_response;
use crate::protocol::{RequestId, RpcResponse};
use crate::server::McpServer;

use super::context::Context;

pub(super) fn forward<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    id: RequestId,
    request: GatewayRequest,
    context: &Context,
    expected_kind: &str,
    expected_operation: Option<&str>,
) -> RpcResponse {
    let expected_generation = request
        .body
        .as_ref()
        .and_then(crate::json::JsonValue::as_object)
        .and_then(|object| object.get("expected_host_generation"))
        .and_then(|value| match value {
            crate::json::JsonValue::Number(value) => Some(*value),
            _ => None,
        });
    let expected_recovery_kind = if request.path.ends_with("/rejoin") {
        Some("rejoin")
    } else if request.path.ends_with("/recover") {
        Some("reconcile")
    } else {
        None
    };
    match server.gateway.forward(request) {
        Ok(response) => match project_coop_native_response(
            &response.body,
            &context.projection_context(),
            expected_kind,
            Some(response.status),
            expected_operation,
            expected_generation,
            expected_recovery_kind,
        ) {
            Ok((body, is_error)) => crate::mapping::tool_result(id, body.to_json(), is_error),
            Err(message) => crate::mapping::tool_result(id, message, true),
        },
        Err(error) => gateway_error(id, error),
    }
}

pub(super) fn forward_legal_catalog<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    id: RequestId,
    request: GatewayRequest,
    context: &Context,
    actor_peer: &str,
    expected_generation: i64,
) -> RpcResponse {
    match server.gateway.forward(request) {
        Ok(response) => match project_coop_native_legal_catalog(
            &response.body,
            &context.projection_context(),
            actor_peer,
            expected_generation,
            response.status,
        ) {
            Ok((body, is_error)) => crate::mapping::tool_result(id, body.to_json(), is_error),
            Err(message) => crate::mapping::tool_result(id, message, true),
        },
        Err(error) => gateway_error(id, error),
    }
}

fn gateway_error(id: RequestId, error: GatewayError) -> RpcResponse {
    let message = match error {
        GatewayError::Unauthorized => "native co-op gateway authorization failed",
        GatewayError::Forbidden => "native co-op gateway scope authorization failed",
        GatewayError::NotFound => "native co-op target or operation was not found",
        GatewayError::Unavailable => "native co-op gateway is unavailable",
        GatewayError::Timeout => "native co-op operation outcome is unknown after timeout",
        GatewayError::MalformedResponse => "native co-op gateway response was malformed",
        GatewayError::Rejected => "native co-op gateway rejected the operation",
    };
    crate::mapping::tool_result(id, message, true)
}
