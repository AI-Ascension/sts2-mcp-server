// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::gateway::{Correlation, GatewayAdapter, GatewayMethod, GatewayRequest};
use crate::json::JsonValue;
use crate::protocol::{METHOD_NOT_FOUND, RpcError, RpcRequest, RpcResponse};
use crate::recovery_frame::{
    RecoveryOperation, build_recovery_request, validate_recovery_response,
};
use crate::server::McpServer;

use super::{
    gateway_error_result, has_only_arguments, invalid_params, safe_header_value, tool_error_result,
    tool_result,
};

const ARGUMENT_FIELDS: [&str; 2] = ["mcp_session_id", "payload"];

/// Dispatches the two recovery tools that carry a gateway route.
///
/// This sideband forwards; it never resolves. The frame is built from the
/// configured caller identity and the caller-supplied payload, and the gateway
/// response is surfaced verbatim because only the runtime owner can cross-check
/// the returned operation record against its own durable intent.
pub(super) fn tools_call<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    request: RpcRequest,
) -> RpcResponse {
    let Some(params) = request.params.as_object() else {
        return invalid_params(request.id, "recovery tools/call params must be an object");
    };
    if !has_only_arguments(params, &["name", "arguments"]) {
        return invalid_params(request.id, "recovery tools/call has unsupported fields");
    }
    let Some(tool) = params.get("name").and_then(JsonValue::as_string) else {
        return invalid_params(request.id, "recovery tool name is missing");
    };
    if server.catalog.descriptor(tool).is_none() {
        return RpcResponse::failure(
            Some(request.id),
            RpcError::new(METHOD_NOT_FOUND, "recovery tool is not active"),
        );
    }
    let Some(operation) = RecoveryOperation::from_tool(tool) else {
        // The closed nine-tool surface advertises the runtime owner's routes.
        // Those are forwarded to the profile that carries them rather than being
        // silently answered here.
        return tool_error_result(
            request.id,
            "watchdog_recovery_route_not_installed",
            "invalid_input",
            "this recovery sideband profile does not carry the requested watchdog route",
        );
    };
    // The catalog's wired flag and this mapping's route table must agree; a
    // drift between them would silently install a route the profile denies.
    if !crate::catalog::watchdog_recovery::is_wired(tool) {
        return RpcResponse::failure(
            Some(request.id),
            RpcError::new(METHOD_NOT_FOUND, "recovery tool is not wired"),
        );
    }
    let Some(arguments) = params.get("arguments").and_then(JsonValue::as_object) else {
        return invalid_params(request.id, "recovery arguments must be an object");
    };
    if !has_only_arguments(arguments, &ARGUMENT_FIELDS) {
        return invalid_params(request.id, "recovery arguments have unsupported fields");
    }
    let Some(payload) = arguments.get("payload") else {
        return invalid_params(request.id, "recovery payload is missing");
    };
    if payload.as_object().is_none() {
        return invalid_params(request.id, "recovery payload must be an object");
    }
    let Some(session) = arguments
        .get("mcp_session_id")
        .and_then(JsonValue::as_string)
    else {
        return invalid_params(request.id, "recovery mcp_session_id is missing");
    };
    let active_session = server.mcp_session_id();
    if !safe_header_value(session) || active_session.is_some_and(|active| active != session) {
        return invalid_params(
            request.id,
            "recovery mcp_session_id is not the active MCP session",
        );
    }
    let session = session.to_owned();
    let Some(principal) = server.gateway().frame_principal() else {
        return tool_error_result(
            request.id,
            "watchdog_recovery_principal_unavailable",
            "denied",
            "the recovery sideband requires a configured gateway caller identity",
        );
    };
    let built = match build_recovery_request(operation, principal, payload) {
        Ok(built) => built,
        Err(message) => return invalid_params(request.id, message),
    };
    let id = request.id;
    let headers = transport_headers(&session, &built.correlation_id);
    let gateway_request = GatewayRequest {
        method: GatewayMethod::Post,
        path: String::from(operation.path()),
        headers,
        body: Some(built.frame.clone()),
        correlation: Correlation {
            mcp_session_id: session,
            mcp_request_id: id.clone(),
        },
    };
    match server.forward_gateway(gateway_request) {
        Ok(response) => {
            match validate_recovery_response(&response.body, operation, &built.correlation_id) {
                // Forwarded verbatim: the record inside the frame is the runtime
                // owner's evidence and is never re-derived or trimmed here.
                Ok(()) => tool_result(id, response.body.to_json(), false),
                Err(message) => tool_error_result(
                    id,
                    "watchdog_recovery_frame_invalid",
                    "malformed_response",
                    format!("gateway recovery frame is not acceptable: {message}"),
                ),
            }
        }
        Err(error) => gateway_error_result(id, error),
    }
}

fn transport_headers(mcp_session: &str, correlation_id: &str) -> BTreeMap<String, String> {
    BTreeMap::from([
        (String::from("x-mcp-session-id"), String::from(mcp_session)),
        (
            String::from("x-sts2-correlation-id"),
            String::from(correlation_id),
        ),
    ])
}

#[cfg(test)]
#[path = "mapping_watchdog_recovery_tests.rs"]
mod tests;
