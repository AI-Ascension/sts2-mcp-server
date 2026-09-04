// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::gateway::{Correlation, GatewayAdapter, GatewayError, GatewayMethod, GatewayRequest};
use crate::json::JsonValue;
use crate::projection::RuntimeV3GameplayContext;
use crate::protocol::{
    INVALID_PARAMS, METHOD_NOT_FOUND, RequestId, RpcError, RpcRequest, RpcResponse,
};
use crate::protocol_artifact_runtime_v3_gameplay::{
    RUNTIME_V3_GAMEPLAY_ACTION_ID, RUNTIME_V3_GAMEPLAY_MAX_CARD_INDEX,
};
use crate::server::McpServer;

use super::{headers, invalid_params, non_empty_string, response};

#[path = "mapping_runtime_v3_gameplay_context.rs"]
mod context;
#[path = "mapping_runtime_v3_gameplay_envelope.rs"]
mod envelope;

use context::{nonnegative_integer, optional_target, request_context};

const SUBMIT_ARGUMENTS: [&str; 9] = [
    "instance_id",
    "mcp_session_id",
    "lease_id",
    "lease_epoch",
    "generation",
    "operation_id",
    "action_id",
    "card_index",
    "target_id",
];
const STATE_ARGUMENTS: [&str; 5] = [
    "instance_id",
    "mcp_session_id",
    "lease_id",
    "lease_epoch",
    "generation",
];
const RECONCILE_ARGUMENTS: [&str; 6] = [
    "instance_id",
    "mcp_session_id",
    "lease_id",
    "lease_epoch",
    "generation",
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
                "tool is not in the active Runtime-v3 gameplay catalog",
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
        crate::catalog::GET_STATE_TOOL => state_call(server, id, arguments, &correlation_id),
        crate::catalog::SUBMIT_ACTION_TOOL => {
            submit_action_call(server, id, arguments, &correlation_id)
        }
        crate::catalog::RECONCILE_ACTION_TOOL => {
            reconcile_action_call(server, id, arguments, &correlation_id)
        }
        _ => RpcResponse::failure(
            Some(id),
            RpcError::new(
                METHOD_NOT_FOUND,
                "tool is not in the active Runtime-v3 gameplay catalog",
            ),
        ),
    }
}

fn state_call<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    id: RequestId,
    arguments: &BTreeMap<String, JsonValue>,
    correlation_id: &str,
) -> RpcResponse {
    if !super::has_only_arguments(arguments, &STATE_ARGUMENTS) {
        return invalid_params(id, "get_state arguments contain an unsupported field");
    }
    let context = match request_context(server, arguments, correlation_id, false) {
        Ok(context) => context,
        Err(message) => return invalid_params(id, message),
    };
    let gateway_request = GatewayRequest {
        method: GatewayMethod::Get,
        path: format!("/v3/instances/{}/state", context.instance_id),
        headers: headers(&context.mcp_session_id, correlation_id),
        body: None,
        correlation: Correlation {
            mcp_session_id: context.mcp_session_id.clone(),
            mcp_request_id: id.clone(),
        },
    };
    forward(server, id, gateway_request, &context, "state_response")
}

fn submit_action_call<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    id: RequestId,
    arguments: &BTreeMap<String, JsonValue>,
    correlation_id: &str,
) -> RpcResponse {
    if !super::has_only_arguments(arguments, &SUBMIT_ARGUMENTS) {
        return invalid_params(id, "submit_action arguments contain an unsupported field");
    }
    let context = match request_context(server, arguments, correlation_id, true) {
        Ok(context) => context,
        Err(message) => return invalid_params(id, message),
    };
    let Some(action_id) = non_empty_string(arguments, "action_id") else {
        return invalid_params(id, "action_id must be the fixed play_card action");
    };
    if action_id != RUNTIME_V3_GAMEPLAY_ACTION_ID {
        return invalid_params(id, "action_id must be the fixed play_card action");
    }
    let Some(card_index) = nonnegative_integer(arguments, "card_index")
        .filter(|value| *value <= RUNTIME_V3_GAMEPLAY_MAX_CARD_INDEX)
    else {
        return invalid_params(id, "card_index must be an integer between 0 and 64");
    };
    let target_id = match optional_target(arguments.get("target_id")) {
        Ok(target_id) => target_id,
        Err(message) => return invalid_params(id, message),
    };
    let gateway_request = GatewayRequest {
        method: GatewayMethod::Post,
        path: format!("/v3/instances/{}/action", context.instance_id),
        headers: headers(&context.mcp_session_id, correlation_id),
        body: Some(envelope::action_request(&context, card_index, target_id)),
        correlation: Correlation {
            mcp_session_id: context.mcp_session_id.clone(),
            mcp_request_id: id.clone(),
        },
    };
    forward(server, id, gateway_request, &context, "action_response")
}

fn reconcile_action_call<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    id: RequestId,
    arguments: &BTreeMap<String, JsonValue>,
    correlation_id: &str,
) -> RpcResponse {
    if !super::has_only_arguments(arguments, &RECONCILE_ARGUMENTS) {
        return invalid_params(
            id,
            "reconcile_action arguments contain an unsupported field",
        );
    }
    let context = match request_context(server, arguments, correlation_id, true) {
        Ok(context) => context,
        Err(message) => return invalid_params(id, message),
    };
    let gateway_request = GatewayRequest {
        method: GatewayMethod::Get,
        path: format!(
            "/v3/instances/{}/operations/{}",
            context.instance_id, context.operation_id
        ),
        headers: headers(&context.mcp_session_id, correlation_id),
        body: None,
        correlation: Correlation {
            mcp_session_id: context.mcp_session_id.clone(),
            mcp_request_id: id.clone(),
        },
    };
    forward(server, id, gateway_request, &context, "reconcile_response")
}

fn forward<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    id: RequestId,
    request: GatewayRequest,
    context: &RuntimeV3GameplayContext,
    expected_kind: &str,
) -> RpcResponse {
    match server.gateway.forward(request) {
        Ok(response) => response::gateway_success_v3(id, response, context, expected_kind),
        Err(error @ (GatewayError::Timeout | GatewayError::Unavailable)) => {
            uncertain_result(id, context, expected_kind, error)
        }
        Err(error) => gateway_error_result(id, error),
    }
}

fn uncertain_result(
    id: RequestId,
    context: &RuntimeV3GameplayContext,
    expected_kind: &str,
    error: GatewayError,
) -> RpcResponse {
    let error_code = match error {
        GatewayError::Timeout | GatewayError::Unavailable => {
            "sts2.runtime/unknown_after_disconnect"
        }
        _ => "sts2.runtime/unknown",
    };
    let body = envelope::result_envelope(context, expected_kind, "unknown", error_code, None);
    response::gateway_success_v3(
        id,
        crate::gateway::GatewayResponse { status: 504, body },
        context,
        expected_kind,
    )
}

fn gateway_error_result(id: RequestId, error: GatewayError) -> RpcResponse {
    let (code, message) = match error {
        GatewayError::Unauthorized => (-32001, "gateway authorization failed"),
        GatewayError::Forbidden => (-32007, "gateway scope authorization failed"),
        GatewayError::NotFound => (-32004, "gateway target was not found"),
        GatewayError::Unavailable => (-32003, "gateway is unavailable"),
        GatewayError::Timeout => (-32008, "gateway request timed out"),
        GatewayError::MalformedResponse => (-32002, "gateway returned an invalid response"),
        GatewayError::Rejected => (-32005, "gateway rejected the request"),
    };
    super::tool_result(id, format!("gateway error {code}: {message}"), true)
}
