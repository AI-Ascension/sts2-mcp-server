// SPDX-License-Identifier: MIT

use crate::gateway::{GatewayAdapter, GatewayMethod};
use crate::json::JsonValue;
use crate::protocol::{RequestId, RpcResponse};
use crate::server::McpServer;

use super::super::{
    gateway_error_result, invalid_params, tool_error_result, tool_error_result_with_metadata,
    tool_result,
};
use super::{
    BINDING_PATH_SUFFIX, CAPABILITIES_PATH_SUFFIX, CONTENT_MANIFEST_PATH_SUFFIX,
    GameInformationContext, QUERY_PATH_SUFFIX,
};
use super::{response, transport};

const MAX_QUERY_BODY_BYTES: usize = 16 * 1024;

pub(super) fn forward_binding<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    context: GameInformationContext,
    binding: JsonValue,
) -> RpcResponse {
    let request = transport::gateway_request(
        &context,
        GatewayMethod::Post,
        format!(
            "/v1/instances/{}/{}",
            context.instance_id, BINDING_PATH_SUFFIX
        ),
        Some(binding.clone()),
    );
    match server.forward_gateway(request) {
        Ok(response) => match response::project_binding(&response.body, &context, &binding) {
            Ok((body, is_error)) => {
                projected_result(context.request_id, body, is_error, response.status)
            }
            Err(message) => projection_error_result(context.request_id, message),
        },
        Err(error) => gateway_error_result(context.request_id, error),
    }
}

pub(super) fn forward_capabilities<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    context: GameInformationContext,
) -> RpcResponse {
    let request = transport::gateway_request(
        &context,
        GatewayMethod::Get,
        format!(
            "/v1/instances/{}/{}",
            context.instance_id, CAPABILITIES_PATH_SUFFIX
        ),
        None,
    );
    match server.forward_gateway(request) {
        Ok(response) => match response::project_capabilities(&response.body, &context) {
            Ok((body, is_error)) => {
                projected_result(context.request_id, body, is_error, response.status)
            }
            Err(message) => projection_error_result(context.request_id, message),
        },
        Err(error) => gateway_error_result(context.request_id, error),
    }
}

/// Reads the whole content catalog through the gateway's fixed bodyless route.
///
/// The tool carries no selector and no body: the gateway, not the adapter, decides which content
/// authority the request resolves to, and a manifest the gateway refuses is relayed as the typed
/// protocol refusal rather than a shortened catalog.
pub(super) fn forward_content_manifest<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    context: GameInformationContext,
) -> RpcResponse {
    let request = transport::gateway_request(
        &context,
        GatewayMethod::Get,
        format!(
            "/v1/instances/{}/{}",
            context.instance_id, CONTENT_MANIFEST_PATH_SUFFIX
        ),
        None,
    );
    match server.forward_gateway(request) {
        Ok(response) => {
            match response::manifest::project_content_manifest(&response.body, &context) {
                Ok((body, is_error)) => {
                    projected_result(context.request_id, body, is_error, response.status)
                }
                Err(message) => projection_error_result(context.request_id, message),
            }
        }
        Err(error) => gateway_error_result(context.request_id, error),
    }
}

pub(super) fn forward_query<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    context: GameInformationContext,
    query: JsonValue,
) -> RpcResponse {
    let body = transport::envelope(&context.correlation_id, "query_request", query.clone());
    if body.to_json().len() > MAX_QUERY_BODY_BYTES {
        return invalid_params(
            context.request_id,
            "game-information query exceeds the request bound",
        );
    }
    let request = transport::gateway_request(
        &context,
        GatewayMethod::Post,
        format!(
            "/v1/instances/{}/{}",
            context.instance_id, QUERY_PATH_SUFFIX
        ),
        Some(body),
    );
    match server.forward_gateway(request) {
        Ok(response) => match response::project_query(&response.body, &context, &query) {
            Ok((body, is_error)) => {
                projected_result(context.request_id, body, is_error, response.status)
            }
            Err(message) => projection_error_result(context.request_id, message),
        },
        Err(error) => gateway_error_result(context.request_id, error),
    }
}

fn projected_result(id: RequestId, body: JsonValue, is_error: bool, status: u16) -> RpcResponse {
    let text = body.to_json();
    if is_error {
        if let Some(code) = response::protocol_error_code(&body) {
            return tool_error_result(
                id,
                code.to_owned(),
                response::protocol_error_category(code),
                text,
            );
        }
        return tool_error_result(
            id,
            "game_information_malformed_response",
            "malformed_response",
            text,
        );
    }
    if !(200..300).contains(&status) {
        return tool_error_result_with_metadata(
            id,
            format!("game_information_http_{status}"),
            response::status_error_category(status),
            text,
            None,
            Some(status),
        );
    }
    tool_result(id, text, false)
}

fn projection_error_result(id: RequestId, message: &'static str) -> RpcResponse {
    let (code, category) = response::projection_error_code(message);
    tool_error_result(id, code, category, message)
}
