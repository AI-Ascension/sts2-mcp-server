// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::catalog::{RECONCILE_SEEDED_RUN_TOOL, START_SEEDED_RUN_TOOL};
use crate::gateway::{Correlation, GatewayAdapter, GatewayError, GatewayMethod, GatewayRequest};
use crate::json::JsonValue;
use crate::protocol::{
    INVALID_PARAMS, METHOD_NOT_FOUND, RequestId, RpcError, RpcRequest, RpcResponse,
};
use crate::server::McpServer;

#[path = "mapping_seeded_run_context.rs"]
mod context;
#[path = "mapping_seeded_run_request.rs"]
mod request;
#[path = "mapping_seeded_run_response.rs"]
mod response;
#[path = "mapping_seeded_run_validation.rs"]
mod validation;

const START_ARGUMENTS: [&str; 9] = [
    "instance_id",
    "mcp_session_id",
    "lease_id",
    "lease_epoch",
    "generation",
    "operation_id",
    "seed",
    "run_mode",
    "selected_context",
];
const RECONCILE_ARGUMENTS: [&str; 6] = [
    "instance_id",
    "mcp_session_id",
    "lease_id",
    "lease_epoch",
    "generation",
    "operation_id",
];
const MESSAGE_FIELDS: [&str; 20] = [
    "protocol_version",
    "schema_digest",
    "provenance",
    "correlation_id",
    "instance_id",
    "session_id",
    "lease_id",
    "lease_epoch",
    "generation",
    "kind",
    "operation_id",
    "requested_seed",
    "run_mode",
    "context_digest",
    "selected_context",
    "status",
    "canonical_seed",
    "observation",
    "effect_witness",
    "error_code",
];

#[derive(Clone, Debug, Eq, PartialEq)]
struct SeededContext {
    correlation_id: String,
    instance_id: String,
    mcp_session_id: String,
    session_id: String,
    lease_id: String,
    lease_epoch: i64,
    generation: i64,
    operation_id: String,
    selected_context: Option<JsonValue>,
    context_digest: Option<String>,
}

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
                "tool is not in the active seeded-run catalog",
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
        START_SEEDED_RUN_TOOL => start_call(server, id, arguments, &correlation_id),
        RECONCILE_SEEDED_RUN_TOOL => reconcile_call(server, id, arguments, &correlation_id),
        _ => RpcResponse::failure(
            Some(id),
            RpcError::new(
                METHOD_NOT_FOUND,
                "tool is not in the active seeded-run catalog",
            ),
        ),
    }
}

fn start_call<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    id: RequestId,
    arguments: &BTreeMap<String, JsonValue>,
    correlation_id: &str,
) -> RpcResponse {
    if !super::has_only_arguments(arguments, &START_ARGUMENTS) {
        return invalid_params(
            id,
            "start_seeded_run arguments contain an unsupported field",
        );
    }
    let mut context = match context::parse(server, arguments, correlation_id, true) {
        Ok(context) => context,
        Err(message) => return invalid_params(id, message),
    };
    let Some(seed) = arguments.get("seed").and_then(JsonValue::as_string) else {
        return invalid_params(id, "seed must be a non-empty bounded string");
    };
    if !validation::validate_seed(seed) {
        return invalid_params(
            id,
            "seed is empty, oversized, or contains a control character",
        );
    }
    let Some(run_mode) = arguments.get("run_mode").and_then(JsonValue::as_string) else {
        return invalid_params(id, "run_mode must be a supported enum value");
    };
    if !validation::validate_mode(run_mode) {
        return invalid_params(
            id,
            "run_mode must be seeded_training, seeded_replay, or diagnostic",
        );
    }
    let selected = arguments
        .get("selected_context")
        .cloned()
        .ok_or("selected_context is required");
    let selected = match selected {
        Ok(value) => value,
        Err(message) => return invalid_params(id, message),
    };
    let digest = match validation::validate_selected_context(&selected) {
        Ok(digest) => digest,
        Err(message) => return invalid_params(id, message),
    };
    context.context_digest = Some(digest);
    context.selected_context = Some(selected);
    let request = GatewayRequest {
        method: GatewayMethod::Post,
        path: format!("/v2/instances/{}/seeded-run", context.instance_id),
        headers: context::authority_headers(&context),
        body: Some(request::start_request(&context, seed, run_mode)),
        correlation: Correlation {
            mcp_session_id: context.mcp_session_id.clone(),
            mcp_request_id: id.clone(),
        },
    };
    forward(
        server,
        id,
        request,
        &context,
        "start_response",
        Some(seed),
        Some(run_mode),
    )
}

fn reconcile_call<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    id: RequestId,
    arguments: &BTreeMap<String, JsonValue>,
    correlation_id: &str,
) -> RpcResponse {
    if !super::has_only_arguments(arguments, &RECONCILE_ARGUMENTS) {
        return invalid_params(
            id,
            "reconcile_seeded_run arguments contain an unsupported field",
        );
    }
    let context = match context::parse(server, arguments, correlation_id, false) {
        Ok(context) => context,
        Err(message) => return invalid_params(id, message),
    };
    let request = GatewayRequest {
        method: GatewayMethod::Get,
        path: format!(
            "/v2/instances/{}/seeded-operations/{}",
            context.instance_id, context.operation_id
        ),
        headers: context::authority_headers(&context),
        body: None,
        correlation: Correlation {
            mcp_session_id: context.mcp_session_id.clone(),
            mcp_request_id: id.clone(),
        },
    };
    forward(
        server,
        id,
        request,
        &context,
        "reconcile_response",
        None,
        None,
    )
}

fn forward<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    id: RequestId,
    request: GatewayRequest,
    context: &SeededContext,
    expected_kind: &str,
    requested_seed: Option<&str>,
    run_mode: Option<&str>,
) -> RpcResponse {
    match server.gateway.forward(request) {
        Ok(response) => response::gateway_success(
            id,
            response,
            context,
            expected_kind,
            requested_seed,
            run_mode,
        ),
        Err(error @ (GatewayError::Timeout | GatewayError::Unavailable))
            if context.selected_context.is_some() =>
        {
            response::unknown_result(id, context, requested_seed, run_mode, error)
        }
        Err(error) => super::gateway_error_result(id, error),
    }
}

fn invalid_params(id: RequestId, message: impl Into<String>) -> RpcResponse {
    RpcResponse::failure(Some(id), RpcError::new(INVALID_PARAMS, message))
}
