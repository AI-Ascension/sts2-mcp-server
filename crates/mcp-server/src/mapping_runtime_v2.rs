// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::gateway::{Correlation, GatewayAdapter, GatewayError, GatewayMethod, GatewayRequest};
use crate::json::JsonValue;
use crate::projection::RuntimeV2Context;
use crate::protocol::{
    INVALID_PARAMS, METHOD_NOT_FOUND, RequestId, RpcError, RpcRequest, RpcResponse,
};
use crate::protocol_artifact_runtime_v2::{RUNTIME_V2_ACTION_ID, RUNTIME_V2_MAX_GENERATION};
use crate::server::McpServer;

use super::{headers, invalid_params, non_empty_string, response};

#[path = "mapping_runtime_v2_envelope.rs"]
mod envelope;

const SUBMIT_ARGUMENTS: [&str; 7] = [
    "instance_id",
    "mcp_session_id",
    "lease_id",
    "lease_epoch",
    "generation",
    "operation_id",
    "action_id",
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
                "tool is not in the active Runtime-v2 catalog",
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
                "tool is not in the active Runtime-v2 catalog",
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
    let context = match request_context(arguments, correlation_id, false) {
        Ok(context) => context,
        Err(message) => return invalid_params(id, message),
    };
    let gateway_request = GatewayRequest {
        method: GatewayMethod::Get,
        path: format!("/v2/instances/{}/state", context.instance_id),
        headers: headers(&context.session_id, correlation_id),
        body: Some(envelope::request_envelope(
            &context,
            "state_request",
            false,
            false,
        )),
        correlation: Correlation {
            mcp_session_id: context.session_id.clone(),
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
    let context = match request_context(arguments, correlation_id, true) {
        Ok(context) => context,
        Err(message) => return invalid_params(id, message),
    };
    let Some(action_id) = non_empty_string(arguments, "action_id") else {
        return invalid_params(id, "action_id must be the fixed end_turn action");
    };
    if action_id != RUNTIME_V2_ACTION_ID {
        return invalid_params(id, "action_id must be the fixed end_turn action");
    }
    let gateway_request = GatewayRequest {
        method: GatewayMethod::Post,
        path: format!("/v2/instances/{}/action", context.instance_id),
        headers: headers(&context.session_id, correlation_id),
        body: Some(envelope::request_envelope(
            &context,
            "action_request",
            true,
            true,
        )),
        correlation: Correlation {
            mcp_session_id: context.session_id.clone(),
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
    let context = match request_context(arguments, correlation_id, true) {
        Ok(context) => context,
        Err(message) => return invalid_params(id, message),
    };
    let gateway_request = GatewayRequest {
        method: GatewayMethod::Get,
        path: format!(
            "/v2/instances/{}/operations/{}",
            context.instance_id, context.operation_id
        ),
        headers: headers(&context.session_id, correlation_id),
        body: None,
        correlation: Correlation {
            mcp_session_id: context.session_id.clone(),
            mcp_request_id: id.clone(),
        },
    };
    forward(server, id, gateway_request, &context, "reconcile_response")
}

fn request_context(
    arguments: &BTreeMap<String, JsonValue>,
    correlation_id: &str,
    require_operation_id: bool,
) -> Result<RuntimeV2Context, &'static str> {
    let instance_id = non_empty_string(arguments, "instance_id")
        .ok_or("instance_id must be a non-empty string")?;
    let session_id = non_empty_string(arguments, "mcp_session_id")
        .ok_or("mcp_session_id must be a non-empty string")?;
    let lease_id =
        non_empty_string(arguments, "lease_id").ok_or("lease_id must be a non-empty string")?;
    let operation_id = if require_operation_id {
        non_empty_string(arguments, "operation_id")
            .ok_or("operation_id is required for Runtime-v2 operations")?
    } else {
        ""
    };
    if !super::safe_segment(instance_id)
        || !super::safe_header_value(session_id)
        || !super::safe_header_value(lease_id)
        || (require_operation_id && !super::safe_header_value(operation_id))
    {
        return Err("Runtime-v2 identity is unsafe or oversized");
    }
    let lease_epoch = bounded_argument(arguments, "lease_epoch")?;
    let generation = bounded_argument(arguments, "generation")?;
    Ok(RuntimeV2Context {
        correlation_id: String::from(correlation_id),
        instance_id: String::from(instance_id),
        session_id: String::from(session_id),
        lease_id: String::from(lease_id),
        lease_epoch,
        generation,
        operation_id: String::from(operation_id),
    })
}

fn bounded_argument(
    arguments: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<i64, &'static str> {
    match arguments.get(key) {
        Some(JsonValue::Number(value)) if *value >= 0 && *value <= RUNTIME_V2_MAX_GENERATION => {
            Ok(*value)
        }
        _ => Err("Runtime-v2 generation or lease_epoch is outside the protocol bound"),
    }
}

fn forward<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    id: RequestId,
    request: GatewayRequest,
    context: &RuntimeV2Context,
    expected_kind: &str,
) -> RpcResponse {
    match server.gateway.forward(request) {
        Ok(response) => response::gateway_success_v2(id, response, context, expected_kind),
        Err(error @ (GatewayError::Timeout | GatewayError::Unavailable)) => {
            uncertain_result(id, context, expected_kind, error)
        }
        Err(error) => gateway_error_result(id, error),
    }
}

fn uncertain_result(
    id: RequestId,
    context: &RuntimeV2Context,
    expected_kind: &str,
    error: GatewayError,
) -> RpcResponse {
    let error_code = match error {
        GatewayError::Timeout | GatewayError::Unavailable => {
            "sts2.runtime/unknown_after_disconnect"
        }
        _ => "sts2.runtime/unknown",
    };
    let body = envelope::result_envelope(context, expected_kind, "unknown", error_code, None, None);
    response::gateway_success_v2(
        id,
        crate::gateway::GatewayResponse { status: 504, body },
        context,
        expected_kind,
    )
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
    super::tool_result(id, format!("gateway error {code}: {message}"), true)
}
