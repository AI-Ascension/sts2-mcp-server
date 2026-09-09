// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::gateway::{GatewayAdapter, GatewayMethod};
use crate::json::JsonValue;
use crate::mapping::invalid_params;
use crate::projection::project_coop_native_effect;
use crate::protocol::{RequestId, RpcResponse};
use crate::protocol_artifact_coop_native::COOP_NATIVE_MAX_BODY_BYTES;
use crate::server::McpServer;

use super::context::{Context, EnvelopePayload, envelope};
use super::gateway::forward;
use super::request::{action, generation, operation_id, peer, recovery, vote};

pub(super) fn observation_call<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    id: RequestId,
    context: Context,
) -> RpcResponse {
    let request = context.gateway_request(GatewayMethod::Get, "observation", None, id.clone());
    forward(server, id, request, &context, "observation", None)
}

pub(super) fn action_call<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    id: RequestId,
    arguments: &BTreeMap<String, JsonValue>,
    context: Context,
) -> RpcResponse {
    let operation_id = match operation_id(arguments, "operation_id") {
        Ok(value) => value,
        Err(message) => return invalid_params(id, message),
    };
    let actor_peer = match peer(arguments, "actor_peer") {
        Ok(value) => value,
        Err(message) => return invalid_params(id, message),
    };
    let expected_generation = match generation(arguments, "expected_host_generation") {
        Ok(value) => value,
        Err(message) => return invalid_params(id, message),
    };
    let action = match action(arguments.get("action")) {
        Ok(value) => value,
        Err(message) => return invalid_params(id, message),
    };
    let body = envelope(
        &context,
        "local_action_request",
        Some(operation_id),
        Some(actor_peer),
        Some(expected_generation),
        EnvelopePayload {
            action: Some(action),
            vote: None,
            recovery: None,
        },
    );
    post_call(
        server,
        id,
        context,
        "action",
        body,
        "effect_response",
        Some(operation_id),
    )
}

pub(super) fn vote_call<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    id: RequestId,
    arguments: &BTreeMap<String, JsonValue>,
    context: Context,
) -> RpcResponse {
    let operation_id = match operation_id(arguments, "operation_id") {
        Ok(value) => value,
        Err(message) => return invalid_params(id, message),
    };
    let actor_peer = match peer(arguments, "actor_peer") {
        Ok(value) => value,
        Err(message) => return invalid_params(id, message),
    };
    let expected_generation = match generation(arguments, "expected_host_generation") {
        Ok(value) => value,
        Err(message) => return invalid_params(id, message),
    };
    let vote = match vote(arguments.get("vote")) {
        Ok(value) => value,
        Err(message) => return invalid_params(id, message),
    };
    let body = envelope(
        &context,
        "shared_vote_request",
        Some(operation_id),
        Some(actor_peer),
        Some(expected_generation),
        EnvelopePayload {
            action: None,
            vote: Some(vote),
            recovery: None,
        },
    );
    post_call(
        server,
        id,
        context,
        "vote",
        body,
        "effect_response",
        Some(operation_id),
    )
}

pub(super) fn rejoin_call<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    id: RequestId,
    arguments: &BTreeMap<String, JsonValue>,
    context: Context,
) -> RpcResponse {
    let operation_id = match operation_id(arguments, "operation_id") {
        Ok(value) => value,
        Err(message) => return invalid_params(id, message),
    };
    let actor_peer = match peer(arguments, "actor_peer") {
        Ok(value) => value,
        Err(message) => return invalid_params(id, message),
    };
    let expected_generation = match generation(arguments, "expected_host_generation") {
        Ok(value) => value,
        Err(message) => return invalid_params(id, message),
    };
    let recovery = match recovery(arguments.get("recovery"), "rejoin") {
        Ok(value) => value,
        Err(message) => return invalid_params(id, message),
    };
    let body = envelope(
        &context,
        "rejoin_request",
        Some(operation_id),
        Some(actor_peer),
        Some(expected_generation),
        EnvelopePayload {
            action: None,
            vote: None,
            recovery: Some(recovery),
        },
    );
    post_call(
        server,
        id,
        context,
        "rejoin",
        body,
        "recovery_response",
        Some(operation_id),
    )
}

pub(super) fn recover_call<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    id: RequestId,
    arguments: &BTreeMap<String, JsonValue>,
    context: Context,
) -> RpcResponse {
    let operation_id = match operation_id(arguments, "operation_id") {
        Ok(value) => value,
        Err(message) => return invalid_params(id, message),
    };
    let recovery = match recovery(arguments.get("recovery"), "reconcile") {
        Ok(value) => value,
        Err(message) => return invalid_params(id, message),
    };
    let body = envelope(
        &context,
        "recovery_response",
        Some(operation_id),
        None,
        None,
        EnvelopePayload {
            action: None,
            vote: None,
            recovery: Some(recovery),
        },
    );
    post_call(
        server,
        id,
        context,
        "recover",
        body,
        "recovery_response",
        Some(operation_id),
    )
}

pub(super) fn effect_call<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    id: RequestId,
    arguments: &BTreeMap<String, JsonValue>,
    correlation: &str,
) -> RpcResponse {
    let context = match Context::read(server, arguments, correlation, &["envelope"]) {
        Ok(context) => context,
        Err(message) => return invalid_params(id, message),
    };
    let Some(envelope) = arguments.get("envelope") else {
        return invalid_params(id, "native effect envelope is missing");
    };
    match project_coop_native_effect(envelope, &context.projection_context()) {
        Ok((body, is_error)) => crate::mapping::tool_result(id, body.to_json(), is_error),
        Err(message) => crate::mapping::tool_result(id, message, true),
    }
}

fn post_call<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    id: RequestId,
    context: Context,
    route: &str,
    body: JsonValue,
    expected_kind: &str,
    expected_operation: Option<&str>,
) -> RpcResponse {
    if body.to_json().len() > COOP_NATIVE_MAX_BODY_BYTES {
        return invalid_params(id, "native co-op request exceeds the protocol body limit");
    }
    let request = context.gateway_request(GatewayMethod::Post, route, Some(body), id.clone());
    forward(
        server,
        id,
        request,
        &context,
        expected_kind,
        expected_operation,
    )
}
