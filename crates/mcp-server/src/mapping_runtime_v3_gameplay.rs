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

use super::{headers, invalid_params, response, tool_result};

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
const DISPATCH_ARGUMENTS: [&str; 8] = [
    "instance_id",
    "mcp_session_id",
    "lease_id",
    "lease_epoch",
    "generation",
    "state_id",
    "operation_id",
    "action",
];
const WAIT_ARGUMENTS: [&str; 7] = [
    "instance_id",
    "mcp_session_id",
    "lease_id",
    "lease_epoch",
    "generation",
    "operation_id",
    "wait_for_millis",
];
const RECOVER_ARGUMENTS: [&str; 7] = [
    "instance_id",
    "mcp_session_id",
    "lease_id",
    "lease_epoch",
    "generation",
    "recovery_kind",
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
    if !super::has_only_arguments(params, &["name", "arguments"]) {
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
    if !super::has_only_arguments(arguments, &LEGAL_ACTION_ARGUMENTS) {
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

fn dispatch_call<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    id: RequestId,
    arguments: &BTreeMap<String, JsonValue>,
    correlation_id: &str,
) -> RpcResponse {
    if !super::has_only_arguments(arguments, &DISPATCH_ARGUMENTS) {
        return invalid_params(
            id,
            "sts2.dispatch_action arguments contain an unsupported field",
        );
    }
    let context =
        match context::RuntimeV3GameplayContext::parse(arguments, correlation_id, true, true) {
            Ok(context) => context,
            Err(message) => return invalid_params(id, message),
        };
    let Some(action) = arguments.get("action") else {
        return invalid_params(id, "sts2.dispatch_action requires one LegalAction");
    };
    let action = match crate::projection::project_runtime_v3_legal_action(action) {
        Ok(action) => action,
        Err(message) => return invalid_params(id, message),
    };
    let body = envelope::request_envelope(
        &context,
        "dispatch_action_request",
        context.state_id.as_deref(),
        context.operation_id.as_deref(),
        Some(action),
        None,
        None,
    );
    let request = gateway_request(
        &context,
        id.clone(),
        GatewayMethod::Post,
        format!("/v3/instances/{}/action", context.instance_id()),
        body,
    );
    forward(
        server,
        id,
        request,
        &context,
        "dispatch_action_response",
        true,
    )
}

fn wait_call<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    id: RequestId,
    arguments: &BTreeMap<String, JsonValue>,
    correlation_id: &str,
) -> RpcResponse {
    if !super::has_only_arguments(arguments, &WAIT_ARGUMENTS) {
        return invalid_params(
            id,
            "sts2.wait_for_transition arguments contain an unsupported field",
        );
    }
    let context =
        match context::RuntimeV3GameplayContext::parse(arguments, correlation_id, false, true) {
            Ok(context) => context,
            Err(message) => return invalid_params(id, message),
        };
    let Some(wait_for_millis) = bounded_wait(arguments) else {
        return invalid_params(
            id,
            "wait_for_millis must be an integer between 1 and 120000",
        );
    };
    let body = envelope::request_envelope(
        &context,
        "wait_request",
        None,
        context.operation_id.as_deref(),
        None,
        Some(wait_for_millis),
        None,
    );
    let request = gateway_request(
        &context,
        id.clone(),
        GatewayMethod::Post,
        format!("/v3/instances/{}/wait", context.instance_id()),
        body,
    );
    forward(server, id, request, &context, "wait_response", true)
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

fn recover_call<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    id: RequestId,
    arguments: &BTreeMap<String, JsonValue>,
    correlation_id: &str,
) -> RpcResponse {
    if !super::has_only_arguments(arguments, &RECOVER_ARGUMENTS) {
        return invalid_params(id, "sts2.recover arguments contain an unsupported field");
    }
    let context =
        match context::RuntimeV3GameplayContext::parse(arguments, correlation_id, false, false) {
            Ok(context) => context,
            Err(message) => return invalid_params(id, message),
        };
    let Some(kind) = arguments
        .get("recovery_kind")
        .and_then(JsonValue::as_string)
    else {
        return invalid_params(id, "recovery_kind must be an allowlisted string");
    };
    if !matches!(
        kind,
        "reobserve" | "reconcile" | "release_lease" | "stop_episode"
    ) {
        return invalid_params(id, "recovery_kind is not allowlisted");
    }
    if kind == "reconcile" && context.operation_id.is_none() {
        return invalid_params(id, "reconcile recovery requires operation_id");
    }
    if kind != "reconcile" && context.operation_id.is_some() {
        return invalid_params(id, "operation_id is only valid for reconcile recovery");
    }
    let recovery = JsonValue::object([
        (String::from("kind"), JsonValue::string(kind)),
        (
            String::from("operation_id"),
            context
                .operation_id
                .as_deref()
                .map_or(JsonValue::Null, JsonValue::string),
        ),
    ]);
    let body = envelope::request_envelope(
        &context,
        "recover_request",
        None,
        None,
        None,
        None,
        Some(recovery),
    );
    let request = gateway_request(
        &context,
        id.clone(),
        GatewayMethod::Post,
        format!("/v3/instances/{}/recover", context.instance_id()),
        body,
    );
    forward(server, id, request, &context, "recover_response", true)
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
    if !super::has_only_arguments(arguments, allowed) {
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
        Ok(response) => {
            response::gateway_success_v3(id, response, &context.projection, expected_kind)
        }
        Err(GatewayError::Timeout | GatewayError::Unavailable) if mutation => {
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

fn bounded_wait(arguments: &BTreeMap<String, JsonValue>) -> Option<i64> {
    match arguments.get("wait_for_millis") {
        Some(JsonValue::Number(value)) if (1..=120_000).contains(value) => Some(*value),
        _ => None,
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
