// SPDX-License-Identifier: MIT

use crate::LIVE_BOOTSTRAP_MAX_BODY_BYTES;
use crate::catalog::GAME_INFORMATION_LIVE_OBSERVATION_BOOTSTRAP_TOOL;
use crate::gateway::{Correlation, GatewayAdapter, GatewayMethod, GatewayRequest};
use crate::json::JsonValue;
use crate::protocol::{METHOD_NOT_FOUND, RpcError, RpcRequest, RpcResponse};
use crate::server::McpServer;

#[path = "mapping_game_information_live_observation_bootstrap_request.rs"]
mod request;
#[path = "mapping_game_information_live_observation_bootstrap_response.rs"]
mod response;
#[path = "mapping_game_information_live_observation_bootstrap_shapes.rs"]
mod shapes;
use request::RequestContext;

const PATH_SUFFIX: &str = "game-information/live-observation-bootstrap";
const ARGUMENTS: [&str; 13] = [
    "instance_id",
    "mcp_session_id",
    "lease_id",
    "lease_epoch",
    "run_id",
    "authority_epoch",
    "content_manifest_id",
    "locale",
    "definition_ref",
    "instance_ref",
    "max_visible_entities",
    "max_item_bytes",
    "max_message_bytes",
];

pub(super) fn is_tool(name: &str) -> bool {
    name == GAME_INFORMATION_LIVE_OBSERVATION_BOOTSTRAP_TOOL
}

pub(super) fn tools_call<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    request: RpcRequest,
) -> RpcResponse {
    let Some(params) = request.params.as_object() else {
        return super::invalid_params(
            request.id,
            "live-observation-bootstrap tools/call params must be an object",
        );
    };
    if !super::has_only_arguments(params, &["name", "arguments"]) {
        return super::invalid_params(
            request.id,
            "live-observation-bootstrap tools/call has unsupported fields",
        );
    }
    if params.get("name").and_then(JsonValue::as_string)
        != Some(GAME_INFORMATION_LIVE_OBSERVATION_BOOTSTRAP_TOOL)
    {
        return RpcResponse::failure(
            Some(request.id),
            RpcError::new(
                METHOD_NOT_FOUND,
                "live-observation-bootstrap tool is not active",
            ),
        );
    }
    let Some(arguments) = params.get("arguments").and_then(JsonValue::as_object) else {
        return super::invalid_params(
            request.id,
            "live-observation-bootstrap arguments must be an object",
        );
    };
    if !super::has_only_arguments(arguments, &ARGUMENTS) {
        return super::invalid_params(
            request.id,
            "live-observation-bootstrap arguments contain an unsupported field",
        );
    }
    let id = request.id;
    let correlation_id = id.stable_text();
    if !super::safe_header_value(&correlation_id) {
        return super::invalid_params(
            id,
            "request id contains an unsafe or oversized header value",
        );
    }
    let context = match RequestContext::read(server, arguments, correlation_id.clone(), id.clone())
    {
        Ok(context) => context,
        Err(message) => return super::invalid_params(id, message),
    };
    let body = context.to_request();
    if body.to_json().len() > LIVE_BOOTSTRAP_MAX_BODY_BYTES
        || body.to_json().len() > context.max_message_bytes
    {
        return super::invalid_params(id, "live-observation-bootstrap request exceeds its bound");
    }
    let gateway_request = GatewayRequest {
        method: GatewayMethod::Post,
        path: format!("/v1/instances/{}/{}", context.instance_id, PATH_SUFFIX),
        headers: context.headers(),
        body: Some(body.clone()),
        correlation: Correlation {
            mcp_session_id: context.mcp_session_id.clone(),
            mcp_request_id: id.clone(),
        },
    };
    match server.forward_gateway(gateway_request) {
        Ok(response) => match response::project_response(&response.body, &context, &body) {
            Ok((projected, is_error)) => {
                let text = projected.to_json();
                if is_error {
                    let code = projected
                        .as_object()
                        .and_then(|object| object.get("error"))
                        .and_then(JsonValue::as_object)
                        .and_then(|error| error.get("code"))
                        .and_then(JsonValue::as_string)
                        .unwrap_or("live_bootstrap_error");
                    super::tool_error_result(
                        id,
                        code.to_owned(),
                        response::error_category(code),
                        text,
                    )
                } else if !(200..300).contains(&response.status) {
                    super::tool_error_result_with_metadata(
                        id,
                        format!("live_bootstrap_http_{}", response.status),
                        response::status_category(response.status),
                        text,
                        None,
                        Some(response.status),
                    )
                } else {
                    super::tool_result(id, text, false)
                }
            }
            Err(message) => super::tool_error_result(
                id,
                "live_bootstrap_malformed_response",
                "malformed_response",
                message,
            ),
        },
        Err(error) => super::gateway_error_result(id, error),
    }
}
