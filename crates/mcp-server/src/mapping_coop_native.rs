// SPDX-License-Identifier: MIT

#[path = "mapping_coop_native_calls.rs"]
mod calls;
#[path = "mapping_coop_native_context.rs"]
mod context;
#[path = "mapping_coop_native_gateway.rs"]
mod gateway;
#[path = "mapping_coop_native_request.rs"]
mod request;

use crate::catalog::{
    COOP_NATIVE_ACTION_TOOL, COOP_NATIVE_EFFECT_TOOL, COOP_NATIVE_OBSERVATION_TOOL,
    COOP_NATIVE_RECOVER_TOOL, COOP_NATIVE_REJOIN_TOOL, COOP_NATIVE_VOTE_TOOL,
};
use crate::gateway::GatewayAdapter;
use crate::json::JsonValue;
use crate::mapping::{has_only_arguments, invalid_params, safe_header_value};
use crate::protocol::{METHOD_NOT_FOUND, RpcError, RpcRequest, RpcResponse};
use crate::server::McpServer;

use calls::{action_call, effect_call, observation_call, recover_call, rejoin_call, vote_call};
use context::Context;

const COMMON_ARGUMENTS: [&str; 4] = ["instance_id", "mcp_session_id", "lease_id", "lease_epoch"];

pub(super) fn tools_call<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    request: RpcRequest,
) -> RpcResponse {
    let Some(params) = request.params.as_object() else {
        return invalid_params(
            request.id,
            "native co-op tools/call params must be an object",
        );
    };
    if !has_only_arguments(params, &["name", "arguments"]) {
        return invalid_params(request.id, "native co-op tools/call has unsupported fields");
    }
    let Some(tool_name) = params.get("name").and_then(JsonValue::as_string) else {
        return invalid_params(request.id, "native co-op tool name is missing");
    };
    if !matches!(
        tool_name,
        COOP_NATIVE_OBSERVATION_TOOL
            | COOP_NATIVE_ACTION_TOOL
            | COOP_NATIVE_VOTE_TOOL
            | COOP_NATIVE_REJOIN_TOOL
            | COOP_NATIVE_EFFECT_TOOL
            | COOP_NATIVE_RECOVER_TOOL
    ) {
        return RpcResponse::failure(
            Some(request.id),
            RpcError::new(METHOD_NOT_FOUND, "native co-op tool is not active"),
        );
    }
    let Some(arguments) = params.get("arguments").and_then(JsonValue::as_object) else {
        return invalid_params(request.id, "native co-op tool arguments must be an object");
    };
    let correlation = request.id.stable_text();
    if !safe_header_value(&correlation) {
        return invalid_params(
            request.id,
            "request id contains an unsafe or oversized header value",
        );
    }
    if tool_name == COOP_NATIVE_EFFECT_TOOL {
        return effect_call(server, request.id, arguments, &correlation);
    }
    let extra = match tool_name {
        COOP_NATIVE_OBSERVATION_TOOL => &[] as &[&str],
        COOP_NATIVE_ACTION_TOOL => &[
            "operation_id",
            "actor_peer",
            "expected_host_generation",
            "action",
        ],
        COOP_NATIVE_VOTE_TOOL => &[
            "operation_id",
            "actor_peer",
            "expected_host_generation",
            "vote",
        ],
        COOP_NATIVE_REJOIN_TOOL => &[
            "operation_id",
            "actor_peer",
            "expected_host_generation",
            "recovery",
        ],
        COOP_NATIVE_RECOVER_TOOL => &["operation_id", "recovery"],
        COOP_NATIVE_EFFECT_TOOL => &[] as &[&str],
        _ => &[] as &[&str],
    };
    let context = match Context::read(server, arguments, &correlation, extra) {
        Ok(context) => context,
        Err(message) => return invalid_params(request.id, message),
    };
    match tool_name {
        COOP_NATIVE_OBSERVATION_TOOL => observation_call(server, request.id, context),
        COOP_NATIVE_ACTION_TOOL => action_call(server, request.id, arguments, context),
        COOP_NATIVE_VOTE_TOOL => vote_call(server, request.id, arguments, context),
        COOP_NATIVE_REJOIN_TOOL => rejoin_call(server, request.id, arguments, context),
        COOP_NATIVE_RECOVER_TOOL => recover_call(server, request.id, arguments, context),
        COOP_NATIVE_EFFECT_TOOL => unreachable!(),
        _ => RpcResponse::failure(
            Some(request.id),
            RpcError::new(METHOD_NOT_FOUND, "native co-op tool is not active"),
        ),
    }
}

#[cfg(test)]
#[path = "mapping_coop_native_tests.rs"]
mod tests;
