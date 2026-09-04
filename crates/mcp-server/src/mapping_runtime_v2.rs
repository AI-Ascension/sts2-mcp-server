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

use super::{invalid_params, non_empty_string, response};

#[path = "mapping_runtime_v2_envelope.rs"]
mod envelope;

#[path = "mapping_runtime_v2_outcome.rs"]
mod outcome;
use outcome::{gateway_error_result, uncertain_result};

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
    let context = match request_context(server, arguments, correlation_id, false) {
        Ok(context) => context,
        Err(message) => return invalid_params(id, message),
    };
    let gateway_request = GatewayRequest {
        method: GatewayMethod::Get,
        path: format!("/v2/instances/{}/state", context.instance_id),
        headers: authority_headers(&context),
        body: Some(envelope::request_envelope(
            &context,
            "state_request",
            false,
            false,
        )),
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
        return invalid_params(id, "action_id must be the fixed end_turn action");
    };
    if action_id != RUNTIME_V2_ACTION_ID {
        return invalid_params(id, "action_id must be the fixed end_turn action");
    }
    let gateway_request = GatewayRequest {
        method: GatewayMethod::Post,
        path: format!("/v2/instances/{}/action", context.instance_id),
        headers: authority_headers(&context),
        body: Some(envelope::request_envelope(
            &context,
            "action_request",
            true,
            true,
        )),
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
            "/v2/instances/{}/operations/{}",
            context.instance_id, context.operation_id
        ),
        headers: authority_headers(&context),
        body: None,
        correlation: Correlation {
            mcp_session_id: context.mcp_session_id.clone(),
            mcp_request_id: id.clone(),
        },
    };
    forward(server, id, gateway_request, &context, "reconcile_response")
}

fn authority_headers(context: &RuntimeV2Context) -> BTreeMap<String, String> {
    let mut headers = super::headers(&context.mcp_session_id, &context.correlation_id);
    for (name, value) in [
        ("x-sts2-instance-id", context.instance_id.clone()),
        ("x-sts2-session-id", context.session_id.clone()),
        ("x-sts2-lease-id", context.lease_id.clone()),
        ("x-sts2-lease-epoch", context.lease_epoch.to_string()),
    ] {
        headers.insert(String::from(name), value);
    }
    headers
}

fn request_context<G: GatewayAdapter>(
    server: &McpServer<G>,
    arguments: &BTreeMap<String, JsonValue>,
    correlation_id: &str,
    require_operation_id: bool,
) -> Result<RuntimeV2Context, &'static str> {
    let instance_id = non_empty_string(arguments, "instance_id")
        .ok_or("instance_id must be a non-empty string")?;
    let mcp_session_id = non_empty_string(arguments, "mcp_session_id")
        .ok_or("mcp_session_id must be a non-empty string")?;
    if let Some(expected) = server.mcp_session_id()
        && expected != mcp_session_id
    {
        return Err("MCP session identity does not match the configured session");
    }
    let session_id = server.gateway_session_id().unwrap_or(mcp_session_id);
    let lease_id =
        non_empty_string(arguments, "lease_id").ok_or("lease_id must be a non-empty string")?;
    let operation_id = if require_operation_id {
        non_empty_string(arguments, "operation_id")
            .ok_or("operation_id is required for Runtime-v2 operations")?
    } else {
        ""
    };
    if !super::safe_segment(instance_id)
        || !super::safe_header_value(mcp_session_id)
        || !super::safe_header_value(session_id)
        || !super::safe_header_value(lease_id)
        || (require_operation_id
            && (!super::safe_header_value(operation_id) || operation_id.contains('/')))
    {
        return Err("Runtime-v2 identity is unsafe or oversized");
    }
    let lease_epoch = bounded_argument(arguments, "lease_epoch")?;
    let generation = bounded_argument(arguments, "generation")?;
    Ok(RuntimeV2Context {
        correlation_id: String::from(correlation_id),
        instance_id: String::from(instance_id),
        session_id: String::from(session_id),
        mcp_session_id: String::from(mcp_session_id),
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
        Ok(response)
            if response.status != 429
                && crate::projection::project_runtime_v2_gateway_body(
                    &response.body,
                    context,
                    expected_kind,
                )
                .is_err() =>
        {
            uncertain_result(id, context, expected_kind, GatewayError::MalformedResponse)
        }
        Ok(response) => response::gateway_success_v2(id, response, context, expected_kind),
        Err(
            error @ (GatewayError::Timeout
            | GatewayError::Unavailable
            | GatewayError::MalformedResponse),
        ) => uncertain_result(id, context, expected_kind, error),
        Err(error) => gateway_error_result(id, error),
    }
}
