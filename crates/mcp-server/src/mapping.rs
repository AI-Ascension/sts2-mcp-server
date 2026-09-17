// SPDX-License-Identifier: MIT

use crate::gateway::GatewayAdapter;
use crate::json::JsonValue;
use crate::protocol::{INVALID_PARAMS, METHOD_NOT_FOUND, RpcError, RpcRequest, RpcResponse};
use crate::server::McpServer;

#[path = "mapping_checkpoint_reference.rs"]
mod checkpoint_reference;
#[path = "mapping_composed_limits.rs"]
mod composed_limits;
#[path = "mapping_coop_native.rs"]
mod coop_native;
#[path = "mapping_coop_receipt_query.rs"]
mod coop_receipt_query;
#[path = "mapping_coop_synchronization.rs"]
mod coop_synchronization;
#[path = "mapping_exact_restore.rs"]
mod exact_restore;
#[path = "mapping_game_information.rs"]
mod game_information;
#[path = "mapping_game_information_live_observation_bootstrap.rs"]
mod game_information_live_observation_bootstrap;
pub use game_information::validate_game_information_binding_discovery;
#[path = "mapping_helpers.rs"]
mod helpers;
#[path = "mapping_legacy.rs"]
mod legacy;
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
#[path = "mapping_save_profile.rs"]
mod save_profile;
#[path = "mapping_seeded_run.rs"]
mod seeded_run;

use composed_limits::{contains_page_items_over, enforce_composed_response, limit_error};
pub(crate) use helpers::safe_segment;
use helpers::{
    forward, gateway_error_result, has_only_arguments, headers, invalid_params, non_empty_string,
    nonnegative_integer, request_context, safe_header_value, tool_error_result,
    tool_error_result_with_metadata, tool_result,
};

pub(crate) fn tools_call<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    request: RpcRequest,
) -> RpcResponse {
    server.begin_tool_call();
    if let Some(response) = server.reject_stale_call(&request) {
        return response;
    }
    if server.catalog.is_save_profile() {
        return save_profile::tools_call(server, request);
    }
    let request_params = request.params.clone();
    let request_id = request.id.clone();
    // Validate snapshot identities before dispatch so malformed references never
    // reach the gateway.
    if let Err(message) = McpServer::<G>::validate_snapshot_params(&request_params) {
        return RpcResponse::failure(Some(request_id), RpcError::new(INVALID_PARAMS, message));
    }
    let response = if server.catalog.is_negotiated_composition() {
        composed_tools_call(server, request)
    } else if server.catalog.is_exact_restore() {
        exact_restore::tools_call(server, request)
    } else if server.catalog.is_game_information() {
        game_information::tools_call(server, request)
    } else if server.catalog.is_live_observation_bootstrap() {
        game_information_live_observation_bootstrap::tools_call(server, request)
    } else if server.catalog.is_coop_native() {
        coop_native::tools_call(server, request)
    } else if server.catalog.is_coop_receipt_query() {
        coop_receipt_query::tools_call(server, request)
    } else if server.catalog.is_coop_synchronization() {
        coop_synchronization::tools_call(server, request)
    } else if server.catalog.is_seeded_run() {
        seeded_run::tools_call(server, request)
    } else if server.catalog.is_runtime_v3_gameplay() {
        runtime_v3_gameplay::tools_call(server, request)
    } else if server.catalog.is_runtime_v4_expert() {
        runtime_v4_expert::tools_call(server, request)
    } else if server.catalog.is_runtime_v4_expert_rest_action() {
        runtime_v4_expert_rest_action::tools_call(server, request)
    } else if server.catalog.is_checkpoint_reference_v1() {
        checkpoint_reference::tools_call(server, request)
    } else if server.catalog.is_runtime_map_v1() {
        runtime_map::tools_call(server, request)
    } else if server.catalog.is_runtime_v2() {
        runtime_v2::tools_call(server, request)
    } else {
        legacy::tools_call(server, request)
    };
    // Explicit admission only. The dispatch path records admission when the call
    // is handed to the gateway, so every refusal that returns before that
    // hand-off leaves snapshot tracking untouched. Response shape is never used:
    // MCP tool errors are result-shaped responses that must not be mistaken for
    // admitted calls.
    if !server.tool_call_was_admitted() {
        return response;
    }
    if let Err(message) = server.remember_snapshot_from_params(&request_params) {
        return RpcResponse::failure(Some(request_id), RpcError::new(INVALID_PARAMS, message));
    }
    if let Err(message) = server.remember_snapshot_from_response(&response) {
        return RpcResponse::failure(Some(request_id), RpcError::new(INVALID_PARAMS, message));
    }
    response
}

fn composed_tools_call<G: GatewayAdapter>(
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
    let tool_name = tool_name.to_owned();
    if server.catalog.descriptor(&tool_name).is_none() {
        return RpcResponse::failure(
            Some(request.id),
            RpcError::new(
                METHOD_NOT_FOUND,
                "tool is not in the active negotiated catalog",
            ),
        );
    }
    let Some(limits) = server.negotiated_limits(&tool_name) else {
        return invalid_params(
            request.id,
            "negotiated limits are unavailable for this tool",
        );
    };
    let request_params = JsonValue::Object(params.clone());
    let request_id = request.id.clone();
    if request_params.to_json().len() > limits.max_request_bytes {
        return limit_error(
            request_id,
            "negotiated_request_limit_exceeded",
            "request exceeds the negotiated per-tool byte limit",
            limits.max_content_bytes,
        );
    }
    if contains_page_items_over(&request_params, limits.max_page_items) {
        return limit_error(
            request_id,
            "negotiated_pagination_limit_exceeded",
            "request exceeds the negotiated page-item limit",
            limits.max_content_bytes,
        );
    }
    if tool_name == crate::catalog::CAPABILITY_DISCOVERY_TOOL {
        let Some(arguments) = params.get("arguments").and_then(JsonValue::as_object) else {
            return invalid_params(
                request.id,
                "capability discovery arguments must be an object",
            );
        };
        if !arguments.is_empty() {
            return invalid_params(request.id, "capability discovery does not accept arguments");
        }
        let Some(composition) = server.catalog.composition() else {
            return RpcResponse::failure(
                Some(request.id),
                RpcError::new(
                    METHOD_NOT_FOUND,
                    "negotiated capability metadata is unavailable",
                ),
            );
        };
        let mut metadata = composition.to_json();
        if let JsonValue::Object(object) = &mut metadata {
            object.insert(
                "refresh_required".to_owned(),
                JsonValue::Bool(server.refresh_required()),
            );
            object.insert(
                "session_epoch".to_owned(),
                JsonValue::Number(i64::try_from(server.session_epoch()).unwrap_or(i64::MAX)),
            );
        }
        // Local discovery is admitted without a gateway hand-off.
        server.admit_tool_call();
        let response = tool_result(request.id, metadata.to_json(), false);
        return enforce_composed_response(request_id, response, limits);
    }
    let response = server.with_dispatch_operation(&tool_name, |server| {
        if game_information::is_tool(&tool_name) {
            return game_information::tools_call(server, request);
        }
        if game_information_live_observation_bootstrap::is_tool(&tool_name) {
            return game_information_live_observation_bootstrap::tools_call(server, request);
        }
        if matches!(
            tool_name.as_str(),
            crate::catalog::MAP_SNAPSHOT_TOOL
                | crate::catalog::OBSERVE_TOOL
                | crate::catalog::LEGAL_ACTIONS_TOOL
                | crate::catalog::DISPATCH_ACTION_TOOL
                | crate::catalog::WAIT_FOR_TRANSITION_TOOL
                | crate::catalog::REOBSERVE_TOOL
                | crate::catalog::RECOVER_TOOL
        ) {
            return runtime_map::tools_call(server, request);
        }
        RpcResponse::failure(
            Some(request.id),
            RpcError::new(
                METHOD_NOT_FOUND,
                "negotiated tool has no bounded gateway mapping",
            ),
        )
    });
    enforce_composed_response(request_id, response, limits)
}
