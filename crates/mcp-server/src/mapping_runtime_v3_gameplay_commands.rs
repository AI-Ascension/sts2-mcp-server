// SPDX-License-Identifier: MIT

use super::{context, envelope, forward, gateway_request, has_only_arguments, invalid_params};
use crate::gateway::{GatewayAdapter, GatewayMethod};
use crate::json::JsonValue;
use crate::protocol::{RequestId, RpcResponse};
use crate::server::McpServer;
use std::collections::BTreeMap;

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

pub(super) fn dispatch_call<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    id: RequestId,
    arguments: &BTreeMap<String, JsonValue>,
    correlation_id: &str,
) -> RpcResponse {
    if !has_only_arguments(arguments, &DISPATCH_ARGUMENTS) {
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

pub(super) fn wait_call<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    id: RequestId,
    arguments: &BTreeMap<String, JsonValue>,
    correlation_id: &str,
) -> RpcResponse {
    if !has_only_arguments(arguments, &WAIT_ARGUMENTS) {
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

pub(super) fn recover_call<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    id: RequestId,
    arguments: &BTreeMap<String, JsonValue>,
    correlation_id: &str,
) -> RpcResponse {
    if !has_only_arguments(arguments, &RECOVER_ARGUMENTS) {
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

fn bounded_wait(arguments: &BTreeMap<String, JsonValue>) -> Option<i64> {
    match arguments.get("wait_for_millis") {
        Some(JsonValue::Number(value)) if (1..=120_000).contains(value) => Some(*value),
        _ => None,
    }
}
