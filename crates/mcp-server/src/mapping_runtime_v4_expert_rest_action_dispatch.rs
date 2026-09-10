// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::gateway::{Correlation, GatewayAdapter, GatewayMethod, GatewayRequest, GatewayResponse};
use crate::json::JsonValue;
use crate::protocol::{RequestId, RpcResponse};
use crate::protocol_artifact_runtime_v4_expert_rest_action::{
    RUNTIME_V4_EXPERT_REST_ACTION_ARTIFACT, RUNTIME_V4_EXPERT_REST_ACTION_GENERATOR,
    RUNTIME_V4_EXPERT_REST_ACTION_PROTOCOL_VERSION, RUNTIME_V4_EXPERT_REST_ACTION_SCHEMA_DIGEST,
    RUNTIME_V4_EXPERT_REST_ACTION_SCHEMA_SOURCE,
};
use crate::server::{McpServer, REST_ACTION_SELECTOR_CAPACITY_ERROR, RestActionOperationContext};

use super::super::{
    gateway_error_result, has_only_arguments, headers, invalid_params, tool_result,
};
use super::ACTION_ARGUMENTS;

#[path = "mapping_runtime_v4_expert_rest_action_binding.rs"]
mod binding;
pub(super) use binding::RestResponseBinding;

pub(super) fn expert_rest_action_call<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    id: RequestId,
    arguments: &BTreeMap<String, JsonValue>,
    correlation_id: &str,
) -> RpcResponse {
    if !has_only_arguments(arguments, &ACTION_ARGUMENTS) {
        return invalid_params(
            id,
            "sts2.expert_rest_action arguments contain an unsupported field",
        );
    }
    let Some(instance_id) = arguments.get("instance_id").and_then(JsonValue::as_string) else {
        return invalid_params(id, "instance_id must be a non-empty string");
    };
    let Some(mcp_session_id) = arguments
        .get("mcp_session_id")
        .and_then(JsonValue::as_string)
    else {
        return invalid_params(id, "mcp_session_id must be a non-empty string");
    };
    let Some(lease_id) = arguments.get("lease_id").and_then(JsonValue::as_string) else {
        return invalid_params(id, "lease_id must be a non-empty string");
    };
    if !super::super::safe_segment(instance_id)
        || !super::super::safe_header_value(mcp_session_id)
        || !super::super::safe_header_value(lease_id)
        || server
            .mcp_session_id()
            .is_some_and(|expected| expected != mcp_session_id)
    {
        return invalid_params(
            id,
            "expert-rest-action identity is unsafe or not bound to this MCP session",
        );
    }
    let Some(lease_epoch) = super::super::nonnegative_integer(arguments, "lease_epoch")
        .filter(|value| *value <= 9_007_199_254_740_991)
    else {
        return invalid_params(id, "lease_epoch exceeds the protocol bound");
    };
    let Some(generation) = super::super::nonnegative_integer(arguments, "generation")
        .filter(|value| *value <= 9_007_199_254_740_991)
    else {
        return invalid_params(id, "generation exceeds the protocol bound");
    };
    let Some(state_id) = arguments
        .get("state_id")
        .and_then(JsonValue::as_string)
        .filter(|value| super::super::safe_header_value(value))
    else {
        return invalid_params(id, "state_id must be a safe non-empty identity");
    };
    let Some(operation_id) = arguments
        .get("operation_id")
        .and_then(JsonValue::as_string)
        .filter(|value| super::super::safe_header_value(value) && !value.contains('/'))
    else {
        return invalid_params(id, "operation_id must be a safe non-empty identity");
    };
    let Some(action) = arguments.get("action") else {
        return invalid_params(id, "action must be a typed native rest-site legal action");
    };
    if crate::projection::validate_runtime_v4_expert_rest_action_reference(action).is_err() {
        return invalid_params(id, "action must be one valid typed native rest-site action");
    }

    let session_id = server
        .gateway_session_id()
        .unwrap_or(mcp_session_id)
        .to_owned();
    let body = JsonValue::object([
        (
            "protocol_version".into(),
            JsonValue::string(RUNTIME_V4_EXPERT_REST_ACTION_PROTOCOL_VERSION),
        ),
        (
            "schema_digest".into(),
            JsonValue::string(RUNTIME_V4_EXPERT_REST_ACTION_SCHEMA_DIGEST),
        ),
        (
            "provenance".into(),
            JsonValue::object([
                (
                    "artifact".into(),
                    JsonValue::string(RUNTIME_V4_EXPERT_REST_ACTION_ARTIFACT),
                ),
                (
                    "source".into(),
                    JsonValue::string(RUNTIME_V4_EXPERT_REST_ACTION_SCHEMA_SOURCE),
                ),
                (
                    "generator".into(),
                    JsonValue::string(RUNTIME_V4_EXPERT_REST_ACTION_GENERATOR),
                ),
            ]),
        ),
        ("profile".into(), JsonValue::string("expert-rest-action")),
        ("correlation_id".into(), JsonValue::string(correlation_id)),
        ("instance_id".into(), JsonValue::string(instance_id)),
        ("session_id".into(), JsonValue::string(session_id.as_str())),
        ("lease_id".into(), JsonValue::string(lease_id)),
        ("lease_epoch".into(), JsonValue::Number(lease_epoch)),
        ("generation".into(), JsonValue::Number(generation)),
        ("state_id".into(), JsonValue::string(state_id)),
        ("operation_id".into(), JsonValue::string(operation_id)),
        ("kind".into(), JsonValue::string("action_request")),
        ("action".into(), action.clone()),
        ("status".into(), JsonValue::Null),
        ("observation".into(), JsonValue::Null),
        ("transition".into(), JsonValue::Null),
        ("effect_witness".into(), JsonValue::Null),
        ("error_code".into(), JsonValue::Null),
    ]);
    let mut request_headers = headers(mcp_session_id, correlation_id);
    request_headers.extend([
        ("x-sts2-instance-id".into(), instance_id.into()),
        ("x-sts2-session-id".into(), session_id.clone()),
        ("x-sts2-lease-id".into(), lease_id.into()),
        ("x-sts2-lease-epoch".into(), lease_epoch.to_string()),
    ]);
    let request = GatewayRequest {
        method: GatewayMethod::Post,
        path: format!("/v4/instances/{instance_id}/expert-rest-action"),
        headers: request_headers,
        body: Some(body),
        correlation: Correlation {
            mcp_session_id: String::from(mcp_session_id),
            mcp_request_id: id.clone(),
        },
    };
    let binding = RestResponseBinding {
        correlation_id: correlation_id.to_owned(),
        instance_id: instance_id.to_owned(),
        session_id: session_id.clone(),
        lease_id: lease_id.to_owned(),
        lease_epoch,
        generation: Some(generation),
        state_id: Some(state_id.to_owned()),
        operation_id: operation_id.to_owned(),
        action: Some(action.clone()),
    };
    let operation_context = RestActionOperationContext {
        mcp_session_id: mcp_session_id.to_owned(),
        instance_id: instance_id.to_owned(),
        session_id: session_id.clone(),
        lease_id: lease_id.to_owned(),
        lease_epoch,
        generation,
        state_id: state_id.to_owned(),
        action: action.clone(),
        selection: None,
    };
    if !server.rest_action_operation_is_compatible(operation_id, &operation_context) {
        return invalid_params(
            id,
            "operation_id is already bound to a different Runtime-v4 REST action",
        );
    }
    if !server.reserve_rest_action_selector_capacity(operation_id, action) {
        return tool_result(id, REST_ACTION_SELECTOR_CAPACITY_ERROR, true);
    }
    if !server.remember_rest_action_operation(operation_id, operation_context) {
        server.release_rest_action_selector_reservation(operation_id);
        return invalid_params(
            id,
            "operation_id is already bound to a different Runtime-v4 REST action",
        );
    }
    match server.gateway.forward(request) {
        Ok(response) => expert_rest_action_response(server, id, response, binding),
        Err(error) => gateway_error_result(id, error),
    }
}

pub(super) fn expert_rest_action_response<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    id: RequestId,
    response: GatewayResponse,
    binding: RestResponseBinding,
) -> RpcResponse {
    let status = response
        .body
        .as_object()
        .and_then(|body| body.get("status"));
    let valid_status = match status.and_then(JsonValue::as_string) {
        Some("accepted") => response.status == 202,
        Some("settled") => response.status == 200,
        Some("rejected") => matches!(response.status, 400 | 409),
        Some("unknown") => matches!(response.status, 404 | 408 | 502 | 504),
        Some("cancelled") => response.status == 499,
        _ => false,
    };
    let admission = server
        .rest_action_selection_admission(
            &binding.instance_id,
            &binding.session_id,
            &binding.lease_id,
            binding.lease_epoch,
            &response.body,
        )
        .cloned();
    let projection =
        crate::projection::project_runtime_v4_expert_rest_action_gateway_body_with_admission(
            &response.body,
            admission.as_ref(),
        );
    let binding_matches = binding.matches(&response.body);
    if !valid_status || projection.is_err() || !binding_matches {
        return super::expert_error_result(id, response.status, &response.body);
    }
    if matches!(
        status.and_then(JsonValue::as_string),
        Some("settled" | "rejected" | "cancelled")
    ) {
        server.release_rest_action_selector_reservation(&binding.operation_id);
    }
    if !server.remember_rest_action_selection(
        &binding.instance_id,
        &binding.session_id,
        &binding.lease_id,
        binding.lease_epoch,
        &response.body,
    ) {
        return tool_result(id, REST_ACTION_SELECTOR_CAPACITY_ERROR, true);
    }
    let is_error = matches!(
        status.and_then(JsonValue::as_string),
        Some("rejected" | "unknown" | "cancelled")
    );
    tool_result(id, response.body.to_json(), is_error)
}
