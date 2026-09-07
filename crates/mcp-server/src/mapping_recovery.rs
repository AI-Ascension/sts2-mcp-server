// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::RECOVERY_MAX_FRAME_BYTES;
use crate::gateway::{Correlation, GatewayAdapter, GatewayError, GatewayMethod, GatewayRequest};
use crate::json::JsonValue;
use crate::protocol::{METHOD_NOT_FOUND, RequestId, RpcError, RpcRequest, RpcResponse};
use crate::server::McpServer;
use crate::{recovery, recovery::FrameIdentity};

use super::{has_only_arguments, invalid_params, tool_result};
use crate::catalog::{
    BOOTSTRAP_TOOL, HOST_FENCE_TOOL, LEASE_ACQUIRE_TOOL, LEASE_RENEW_TOOL, LEASE_REVOKE_TOOL,
    OPERATION_DISPATCH_TOOL, OPERATION_INTENT_TOOL, OPERATION_LOOKUP_TOOL,
    OPERATION_RECONCILE_TOOL,
};

pub(crate) fn tools_call<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    request: RpcRequest,
) -> RpcResponse {
    let Some(params) = request.params.as_object() else {
        return invalid_params(request.id, "recovery tools/call params must be an object");
    };
    if !has_only_arguments(params, &["name", "arguments"]) {
        return invalid_params(
            request.id,
            "recovery tools/call params contain an unsupported field",
        );
    }
    let Some(tool_name) = params.get("name").and_then(JsonValue::as_string) else {
        return invalid_params(request.id, "recovery tools/call requires a tool name");
    };
    let Some(kind) = tool_kind(tool_name) else {
        return RpcResponse::failure(
            Some(request.id),
            RpcError::new(METHOD_NOT_FOUND, "recovery tool is not active"),
        );
    };
    if server.catalog().descriptor(tool_name).is_none() {
        return RpcResponse::failure(
            Some(request.id),
            RpcError::new(
                METHOD_NOT_FOUND,
                "recovery tool is not in the active catalog",
            ),
        );
    }
    let Some(arguments) = params.get("arguments").and_then(JsonValue::as_object) else {
        return invalid_params(request.id, "recovery tool arguments must be an object");
    };
    if !has_only_arguments(arguments, &["mcp_session_id", "payload"]) {
        return invalid_params(
            request.id,
            "recovery tool arguments contain an unsupported field",
        );
    }
    let id = request.id;
    let Some(session) = arguments
        .get("mcp_session_id")
        .and_then(JsonValue::as_string)
        .filter(|value| super::safe_header_value(value))
    else {
        return invalid_params(id, "mcp_session_id is empty or unsafe");
    };
    if server
        .mcp_session_id()
        .is_some_and(|expected| expected != session)
    {
        return invalid_params(
            id,
            "MCP session identity does not match the configured session",
        );
    }
    let Some(payload) = arguments.get("payload").cloned() else {
        return invalid_params(id, "recovery payload is required");
    };
    let frame_identity = FrameIdentity {
        principal_id: &server.recovery_principal_id,
        role: &server.recovery_role,
        proof: server.recovery_proof.as_deref(),
    };
    let (frame, correlation) = match recovery::build_request(kind, payload, frame_identity) {
        Ok(value) => value,
        Err(message) => return invalid_params(id, message),
    };
    if let Err(message) = recovery::validate_request(&frame, kind, &correlation, None) {
        return invalid_params(id, message);
    }
    let gateway_request = GatewayRequest {
        method: GatewayMethod::Post,
        path: path(kind).to_owned(),
        headers: headers(
            session,
            &correlation,
            recovery::capability(kind).unwrap_or(""),
        ),
        body: Some(frame.clone()),
        correlation: Correlation {
            mcp_session_id: session.to_owned(),
            mcp_request_id: recovery::correlation_request_id(correlation.clone()),
        },
    };
    forward(server, id, kind, correlation, frame, gateway_request)
}

fn forward<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    id: RequestId,
    kind: &str,
    correlation: String,
    request_frame: JsonValue,
    request: GatewayRequest,
) -> RpcResponse {
    match server.gateway.forward(request) {
        Ok(response) => match recovery::project_response(&response.body, kind, &correlation) {
            Ok(body) => bounded_tool_result(
                id,
                body.to_json(),
                recovery::is_error(&response.body, response.status),
            ),
            Err(_) => unknown_response(id, kind, &request_frame, &correlation),
        },
        Err(
            GatewayError::Timeout | GatewayError::Unavailable | GatewayError::MalformedResponse,
        ) => unknown_response(id, kind, &request_frame, &correlation),
        Err(GatewayError::NotFound) => bounded_tool_result(
            id,
            "recovery gateway returned NOT_FOUND; execution state remains unproven",
            true,
        ),
        Err(error) => bounded_tool_result(id, gateway_error(error), true),
    }
}

fn unknown_response(
    id: RequestId,
    kind: &str,
    frame: &JsonValue,
    correlation: &str,
) -> RpcResponse {
    bounded_tool_result(
        id,
        recovery::unknown(kind, frame, correlation).to_json(),
        true,
    )
}

fn bounded_tool_result(id: RequestId, text: impl Into<String>, is_error: bool) -> RpcResponse {
    let response = tool_result(id.clone(), text, is_error);
    if response.to_json().len() <= RECOVERY_MAX_FRAME_BYTES {
        response
    } else {
        tool_result(id, "recovery gateway returned an oversized response", true)
    }
}

fn gateway_error(error: GatewayError) -> &'static str {
    match error {
        GatewayError::Unauthorized => "recovery gateway authorization failed",
        GatewayError::Forbidden => "recovery capability authorization failed",
        GatewayError::NotFound => "recovery gateway returned NOT_FOUND; execution remains unproven",
        GatewayError::Unavailable => "recovery gateway is unavailable",
        GatewayError::Timeout => "recovery transport timed out; operation state is unknown",
        GatewayError::MalformedResponse => {
            "recovery gateway response was malformed; operation state is unknown"
        }
        GatewayError::Rejected => "recovery gateway rejected the request",
    }
}

fn headers(session: &str, correlation: &str, capability: &str) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("x-mcp-session-id".to_owned(), session.to_owned()),
        ("x-mcp-request-id".to_owned(), correlation.to_owned()),
        ("x-sts2-correlation-id".to_owned(), correlation.to_owned()),
        (
            "x-sts2-recovery-capability".to_owned(),
            capability.to_owned(),
        ),
    ])
}

fn path(kind: &str) -> &'static str {
    match kind {
        "bootstrap" => "/v1/recovery/bootstrap",
        "host_fence" => "/v1/recovery/host-fence",
        "lease_acquire" => "/v1/recovery/lease/acquire",
        "lease_renew" => "/v1/recovery/lease/renew",
        "lease_revoke" => "/v1/recovery/lease/revoke",
        "operation_intent" => "/v1/recovery/operation/intent",
        "operation_dispatch" => "/v1/recovery/operation/dispatch",
        "operation_lookup" => "/v1/recovery/operation/lookup",
        "operation_reconcile" => "/v1/recovery/operation/reconcile",
        _ => "/v1/recovery/invalid",
    }
}

fn tool_kind(tool: &str) -> Option<&'static str> {
    match tool {
        BOOTSTRAP_TOOL => Some("bootstrap"),
        HOST_FENCE_TOOL => Some("host_fence"),
        LEASE_ACQUIRE_TOOL => Some("lease_acquire"),
        LEASE_RENEW_TOOL => Some("lease_renew"),
        LEASE_REVOKE_TOOL => Some("lease_revoke"),
        OPERATION_INTENT_TOOL => Some("operation_intent"),
        OPERATION_DISPATCH_TOOL => Some("operation_dispatch"),
        OPERATION_LOOKUP_TOOL => Some("operation_lookup"),
        OPERATION_RECONCILE_TOOL => Some("operation_reconcile"),
        _ => None,
    }
}
