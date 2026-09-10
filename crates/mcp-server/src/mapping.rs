// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::catalog::{GET_STATE_TOOL, SUBMIT_ACTION_TOOL};
use crate::gateway::{Correlation, GatewayAdapter, GatewayMethod, GatewayRequest};
use crate::json::JsonValue;
use crate::protocol::{
    INVALID_PARAMS, METHOD_NOT_FOUND, RequestId, RpcError, RpcRequest, RpcResponse,
};
use crate::server::McpServer;

const INVALID_REQUEST_ID: &str = "request id contains an unsafe or oversized header value";

#[path = "mapping_coop_native.rs"]
mod coop_native;
#[path = "mapping_coop_receipt_query.rs"]
mod coop_receipt_query;
#[path = "mapping_coop_synchronization.rs"]
mod coop_synchronization;
#[path = "mapping_helpers.rs"]
mod helpers;
#[path = "mapping_response.rs"]
mod response;
#[path = "mapping_runtime.rs"]
mod runtime;
#[path = "mapping_runtime_map.rs"]
mod runtime_map;
#[path = "mapping_runtime_v2.rs"]
mod runtime_v2;
#[path = "mapping_runtime_v3_gameplay.rs"]
mod runtime_v3_gameplay;
#[path = "mapping_runtime_v4_expert.rs"]
mod runtime_v4_expert;
#[path = "mapping_runtime_v4_expert_rest_action.rs"]
mod runtime_v4_expert_rest_action;
#[path = "mapping_seeded_run.rs"]
mod seeded_run;

pub(crate) use helpers::safe_segment;
use helpers::{
    forward, gateway_error_result, has_only_arguments, headers, invalid_params, non_empty_string,
    nonnegative_integer, poc_action_request, request_context, safe_header_value, tool_result,
};

pub(crate) fn tools_call<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    request: RpcRequest,
) -> RpcResponse {
    if server.catalog.is_coop_native() {
        return coop_native::tools_call(server, request);
    }
    if server.catalog.is_coop_receipt_query() {
        return coop_receipt_query::tools_call(server, request);
    }
    if server.catalog.is_coop_synchronization() {
        return coop_synchronization::tools_call(server, request);
    }
    if server.catalog.is_seeded_run() {
        return seeded_run::tools_call(server, request);
    }
    if server.catalog.is_runtime_v3_gameplay() {
        return runtime_v3_gameplay::tools_call(server, request);
    }
    if server.catalog.is_runtime_v4_expert() {
        return runtime_v4_expert::tools_call(server, request);
    }
    if server.catalog.is_runtime_v4_expert_rest_action() {
        return runtime_v4_expert_rest_action::tools_call(server, request);
    }
    if server.catalog.is_runtime_map_v1() {
        return runtime_map::tools_call(server, request);
    }
    if server.catalog.is_runtime_v2() {
        return runtime_v2::tools_call(server, request);
    }
    let Some(params) = request.params.as_object() else {
        return RpcResponse::failure(
            Some(request.id),
            RpcError::new(INVALID_PARAMS, "tools/call params must be an object"),
        );
    };
    let Some(tool_name) = params.get("name").and_then(JsonValue::as_string) else {
        return RpcResponse::failure(
            Some(request.id),
            RpcError::new(INVALID_PARAMS, "tools/call requires a tool name"),
        );
    };
    if server.catalog.descriptor(tool_name).is_none() {
        return RpcResponse::failure(
            Some(request.id),
            RpcError::new(METHOD_NOT_FOUND, "tool is not in the active catalog"),
        );
    }
    let Some(arguments) = params.get("arguments").and_then(JsonValue::as_object) else {
        return RpcResponse::failure(
            Some(request.id),
            RpcError::new(INVALID_PARAMS, "tools/call arguments must be an object"),
        );
    };
    let id = request.id;
    let correlation_id = id.stable_text();
    if !safe_header_value(&correlation_id) {
        return invalid_params(id, INVALID_REQUEST_ID);
    }
    match tool_name {
        GET_STATE_TOOL => state_call(server, id, arguments, &correlation_id),
        SUBMIT_ACTION_TOOL if server.catalog.is_runtime_v1() => {
            runtime::runtime_action_call(server, id, arguments, &correlation_id)
        }
        SUBMIT_ACTION_TOOL => action_call(server, id, arguments, &correlation_id),
        _ => RpcResponse::failure(
            Some(id),
            RpcError::new(METHOD_NOT_FOUND, "tool is not in the active catalog"),
        ),
    }
}

fn state_call<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    id: RequestId,
    arguments: &BTreeMap<String, JsonValue>,
    correlation_id: &str,
) -> RpcResponse {
    if !has_only_arguments(arguments, &["instance_id", "mcp_session_id"]) {
        return invalid_params(id, "tools/call arguments contain an unsupported field");
    }
    let (instance_id, session_id) = match request_context(arguments) {
        Ok(context) => context,
        Err(message) => {
            return invalid_params(id, message);
        }
    };
    let gateway_request = GatewayRequest {
        method: GatewayMethod::Get,
        path: format!("/v1/instances/{instance_id}/state"),
        headers: headers(session_id, correlation_id),
        body: None,
        correlation: Correlation {
            mcp_session_id: String::from(session_id),
            mcp_request_id: id.clone(),
        },
    };
    forward(server, id, gateway_request)
}

fn action_call<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    id: RequestId,
    arguments: &BTreeMap<String, JsonValue>,
    correlation_id: &str,
) -> RpcResponse {
    if !has_only_arguments(
        arguments,
        &[
            "instance_id",
            "mcp_session_id",
            "generation",
            "action_id",
            "units",
        ],
    ) {
        return invalid_params(id, "tools/call arguments contain an unsupported field");
    }
    let (instance_id, session_id) = match request_context(arguments) {
        Ok(context) => context,
        Err(message) => {
            return invalid_params(id, message);
        }
    };
    let Some(generation) = nonnegative_integer(arguments, "generation")
        .filter(|value| *value <= crate::protocol_artifact::POC_MAX_GENERATION)
    else {
        return invalid_params(id, "generation exceeds the protocol bound");
    };
    let Some(action_id) = non_empty_string(arguments, "action_id") else {
        return invalid_params(id, "action_id must be a non-empty string");
    };
    if action_id != "use_budget" {
        return invalid_params(id, "action_id must be use_budget");
    }
    let Some(units) =
        nonnegative_integer(arguments, "units").filter(|value| (0..=8).contains(value))
    else {
        return invalid_params(id, "units must be an integer between 0 and 8");
    };
    let gateway_request = GatewayRequest {
        method: GatewayMethod::Post,
        path: format!("/v1/instances/{instance_id}/action"),
        headers: headers(session_id, correlation_id),
        body: Some(poc_action_request(
            correlation_id,
            instance_id,
            generation,
            action_id,
            units,
        )),
        correlation: Correlation {
            mcp_session_id: String::from(session_id),
            mcp_request_id: id.clone(),
        },
    };
    forward(server, id, gateway_request)
}
