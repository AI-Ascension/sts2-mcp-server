// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::catalog::{
    DISPATCH_ACTION_TOOL, LEGAL_ACTIONS_TOOL, OBSERVE_TOOL, RECOVER_TOOL, REOBSERVE_TOOL,
    WAIT_FOR_TRANSITION_TOOL,
};
use crate::gateway::{Correlation, GatewayAdapter, GatewayError, GatewayMethod, GatewayRequest};
use crate::json::JsonValue;
use crate::protocol::{
    INVALID_PARAMS, METHOD_NOT_FOUND, RequestId, RpcError, RpcRequest, RpcResponse,
};
use crate::server::McpServer;

use super::{has_only_arguments, headers, invalid_params, response, tool_result};

#[path = "mapping_runtime_v3_gameplay_commands.rs"]
mod commands;
use commands::{dispatch_call, recover_call, wait_call};

#[path = "mapping_runtime_v3_gameplay_context.rs"]
mod context;
#[path = "mapping_runtime_v3_gameplay_envelope.rs"]
mod envelope;

const OBSERVE_ARGUMENTS: [&str; 5] = [
    "instance_id",
    "mcp_session_id",
    "lease_id",
    "lease_epoch",
    "generation",
];
const LEGAL_ACTION_ARGUMENTS: [&str; 6] = [
    "instance_id",
    "mcp_session_id",
    "lease_id",
    "lease_epoch",
    "generation",
    "state_id",
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
    if server.catalog.descriptor(tool_name).is_none() {
        return RpcResponse::failure(
            Some(request.id),
            RpcError::new(
                METHOD_NOT_FOUND,
                "tool is not in the active Runtime-v3 catalog",
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
        OBSERVE_TOOL => observe_call(server, id, arguments, &correlation_id),
        LEGAL_ACTIONS_TOOL => legal_actions_call(server, id, arguments, &correlation_id),
        DISPATCH_ACTION_TOOL => dispatch_call(server, id, arguments, &correlation_id),
        WAIT_FOR_TRANSITION_TOOL => wait_call(server, id, arguments, &correlation_id),
        REOBSERVE_TOOL => reobserve_call(server, id, arguments, &correlation_id),
        RECOVER_TOOL => recover_call(server, id, arguments, &correlation_id),
        _ => RpcResponse::failure(
            Some(id),
            RpcError::new(
                METHOD_NOT_FOUND,
                "tool is not in the active Runtime-v3 catalog",
            ),
        ),
    }
}

fn observe_call<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    id: RequestId,
    arguments: &BTreeMap<String, JsonValue>,
    correlation_id: &str,
) -> RpcResponse {
    call_read(
        server,
        id,
        arguments,
        correlation_id,
        &OBSERVE_ARGUMENTS,
        "state_request",
        "state_response",
    )
}

fn legal_actions_call<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    id: RequestId,
    arguments: &BTreeMap<String, JsonValue>,
    correlation_id: &str,
) -> RpcResponse {
    if !has_only_arguments(arguments, &LEGAL_ACTION_ARGUMENTS) {
        return invalid_params(
            id,
            "sts2.legal_actions arguments contain an unsupported field",
        );
    }
    let context =
        match context::RuntimeV3GameplayContext::parse(arguments, correlation_id, true, false) {
            Ok(context) => context,
            Err(message) => return invalid_params(id, message),
        };
    let body = envelope::request_envelope(
        &context,
        "legal_actions_request",
        context.state_id.as_deref(),
        None,
        None,
        None,
        None,
    );
    let request = gateway_request(
        &context,
        id.clone(),
        GatewayMethod::Get,
        format!("/v3/instances/{}/legal-actions", context.instance_id()),
        body,
    );
    forward(
        server,
        id,
        request,
        &context,
        "legal_actions_response",
        false,
    )
}

fn reobserve_call<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    id: RequestId,
    arguments: &BTreeMap<String, JsonValue>,
    correlation_id: &str,
) -> RpcResponse {
    call_read(
        server,
        id,
        arguments,
        correlation_id,
        &OBSERVE_ARGUMENTS,
        "reobserve_request",
        "reobserve_response",
    )
}

fn call_read<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    id: RequestId,
    arguments: &BTreeMap<String, JsonValue>,
    correlation_id: &str,
    allowed: &[&str],
    request_kind: &str,
    response_kind: &str,
) -> RpcResponse {
    if !has_only_arguments(arguments, allowed) {
        return invalid_params(id, "Runtime-v3 read arguments contain an unsupported field");
    }
    let context =
        match context::RuntimeV3GameplayContext::parse(arguments, correlation_id, false, false) {
            Ok(context) => context,
            Err(message) => return invalid_params(id, message),
        };
    let body = envelope::request_envelope(&context, request_kind, None, None, None, None, None);
    let path_suffix = if request_kind == "reobserve_request" {
        "reobserve"
    } else {
        "state"
    };
    let request = gateway_request(
        &context,
        id.clone(),
        GatewayMethod::Get,
        format!("/v3/instances/{}/{}", context.instance_id(), path_suffix),
        body,
    );
    forward(server, id, request, &context, response_kind, false)
}

fn gateway_request(
    context: &context::RuntimeV3GameplayContext,
    id: RequestId,
    method: GatewayMethod,
    path: String,
    body: JsonValue,
) -> GatewayRequest {
    GatewayRequest {
        method,
        path,
        headers: headers(context.session_id(), context.correlation_id()),
        body: Some(body),
        correlation: Correlation {
            mcp_session_id: String::from(context.session_id()),
            mcp_request_id: id,
        },
    }
}

fn forward<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    id: RequestId,
    request: GatewayRequest,
    context: &context::RuntimeV3GameplayContext,
    expected_kind: &str,
    mutation: bool,
) -> RpcResponse {
    match server.gateway.forward(request) {
        Ok(response)
            if mutation
                && crate::projection::project_runtime_v3_gateway_body(
                    &response.body,
                    &context.projection,
                    expected_kind,
                )
                .is_err() =>
        {
            let body = envelope::unknown_response(
                context,
                expected_kind,
                "sts2.runtime/unknown_after_invalid_response",
            );
            response::gateway_success_v3(
                id,
                crate::gateway::GatewayResponse { status: 502, body },
                &context.projection,
                expected_kind,
            )
        }
        Ok(response) => {
            response::gateway_success_v3(id, response, &context.projection, expected_kind)
        }
        Err(
            GatewayError::Timeout | GatewayError::Unavailable | GatewayError::MalformedResponse,
        ) if mutation => {
            let body = envelope::unknown_response(
                context,
                expected_kind,
                "sts2.runtime/unknown_after_disconnect",
            );
            response::gateway_success_v3(
                id,
                crate::gateway::GatewayResponse { status: 504, body },
                &context.projection,
                expected_kind,
            )
        }
        Err(error) => gateway_error_result(id, error),
    }
}

fn gateway_error_result(id: RequestId, error: GatewayError) -> RpcResponse {
    let (code, message) = match error {
        GatewayError::Unauthorized => (-32001, "gateway authorization failed"),
        GatewayError::NotFound => (-32004, "gateway target was not found"),
        GatewayError::Unavailable => (-32003, "gateway is unavailable"),
        GatewayError::Timeout => (-32008, "gateway request timed out"),
        GatewayError::MalformedResponse => (-32002, "gateway returned an invalid response"),
        GatewayError::Rejected => (-32005, "gateway rejected the request"),
    };
    tool_result(id, format!("gateway error {code}: {message}"), true)
}
