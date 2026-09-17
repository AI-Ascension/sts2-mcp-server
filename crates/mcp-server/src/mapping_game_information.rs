// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::catalog::{GAME_INFORMATION_BINDING_TOOL, GAME_INFORMATION_CAPABILITIES_TOOL};
use crate::gateway::{GatewayAdapter, GatewayMethod};
use crate::json::JsonValue;
use crate::protocol::{METHOD_NOT_FOUND, RequestId, RpcError, RpcRequest, RpcResponse};
use crate::server::McpServer;

const MAX_QUERY_BODY_BYTES: usize = 16 * 1024;

#[path = "mapping_game_information_allowed.rs"]
mod allowed;
pub(super) use allowed::is_tool;
#[path = "mapping_game_information_request.rs"]
mod request;
#[path = "mapping_game_information_response.rs"]
mod response;
#[path = "mapping_game_information_transport.rs"]
mod transport;

pub(crate) const CAPABILITIES_PATH_SUFFIX: &str = "game-information/capabilities";
pub(crate) const QUERY_PATH_SUFFIX: &str = "game-information/query";
pub(crate) const BINDING_PATH_SUFFIX: &str = "game-information/lookup-binding";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CallKind {
    List,
    Search,
    Get,
    Detail,
    Availability,
}

impl CallKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::List => "list",
            Self::Search => "search",
            Self::Get => "get",
            Self::Detail => "detail",
            Self::Availability => "availability",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GameInformationContext {
    pub(crate) instance_id: String,
    pub(crate) mcp_session_id: String,
    pub(crate) gateway_session_id: String,
    pub(crate) lease_id: String,
    pub(crate) lease_epoch: i64,
    pub(crate) correlation_id: String,
    pub(crate) request_id: RequestId,
}

/// Validates the configured startup discovery against the pinned lookup-binding
/// schema and the exact owner scope, epoch, instance, and response correlation.
pub fn validate_game_information_binding_discovery(
    body: &JsonValue,
    instance_id: &str,
    correlation_id: &str,
    request: &JsonValue,
) -> Result<(), &'static str> {
    if request
        .as_object()
        .and_then(|object| object.get("operation"))
        != Some(&JsonValue::string("discovery"))
    {
        return Err("lookup-binding bootstrap only accepts discovery");
    }
    let context = GameInformationContext {
        instance_id: instance_id.to_owned(),
        mcp_session_id: String::from("bootstrap"),
        gateway_session_id: String::from("bootstrap"),
        lease_id: String::from("bootstrap"),
        lease_epoch: 0,
        correlation_id: correlation_id.to_owned(),
        request_id: RequestId::String(correlation_id.to_owned()),
    };
    let (_, is_error) = response::project_binding(body, &context, request)?;
    if is_error {
        return Err("lookup-binding bootstrap returned an error response");
    }
    Ok(())
}

pub(super) fn tools_call<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    request: RpcRequest,
) -> RpcResponse {
    let Some(params) = request.params.as_object() else {
        return super::invalid_params(
            request.id,
            "game-information tools/call params must be an object",
        );
    };
    if params
        .keys()
        .any(|key| !matches!(key.as_str(), "name" | "arguments"))
    {
        return super::invalid_params(
            request.id,
            "game-information tools/call has unsupported fields",
        );
    }
    let request_id = request.id.clone();
    let Some(tool_name) = params.get("name").and_then(JsonValue::as_string) else {
        return super::invalid_params(request_id, "tools/call requires a tool name");
    };
    if !allowed::is_tool(tool_name) {
        return RpcResponse::failure(
            Some(request_id),
            RpcError::new(METHOD_NOT_FOUND, "game-information tool is not active"),
        );
    }
    let Some(arguments) = params.get("arguments").and_then(JsonValue::as_object) else {
        return super::invalid_params(request_id, "game-information arguments must be an object");
    };
    if !super::has_only_arguments(arguments, allowed::arguments(tool_name)) {
        return super::invalid_params(
            request_id,
            "game-information arguments contain an unsupported field",
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
    let context = match transport::context(server, arguments, &correlation_id, id.clone()) {
        Ok(context) => context,
        Err(message) => return super::invalid_params(id, message),
    };
    if tool_name == GAME_INFORMATION_CAPABILITIES_TOOL {
        return forward_capabilities(server, context);
    }
    if tool_name == GAME_INFORMATION_BINDING_TOOL {
        let binding = match request::binding(arguments) {
            Ok(binding) => binding,
            Err(message) => return super::invalid_params(id, message),
        };
        return forward_binding(server, context, binding);
    }
    let Some(kind) = allowed::kind_for(tool_name) else {
        return super::invalid_params(id, "game-information query kind is unavailable");
    };
    let (context, query) = match request::query(arguments, kind, context) {
        Ok(value) => value,
        Err(message) => return super::invalid_params(id, message),
    };
    forward_query(server, context, query)
}

fn forward_binding<G: GatewayAdapter>(
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
        Err(error) => super::gateway_error_result(context.request_id, error),
    }
}

fn forward_capabilities<G: GatewayAdapter>(
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
        Err(error) => super::gateway_error_result(context.request_id, error),
    }
}

fn forward_query<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    context: GameInformationContext,
    query: JsonValue,
) -> RpcResponse {
    let body = transport::envelope(&context.correlation_id, "query_request", query.clone());
    if body.to_json().len() > MAX_QUERY_BODY_BYTES {
        return super::invalid_params(
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
        Err(error) => super::gateway_error_result(context.request_id, error),
    }
}

fn projected_result(id: RequestId, body: JsonValue, is_error: bool, status: u16) -> RpcResponse {
    let text = body.to_json();
    if is_error {
        if let Some(code) = response::protocol_error_code(&body) {
            return super::tool_error_result(
                id,
                code.to_owned(),
                response::protocol_error_category(code),
                text,
            );
        }
        return super::tool_error_result(
            id,
            "game_information_malformed_response",
            "malformed_response",
            text,
        );
    }
    if !(200..300).contains(&status) {
        return super::tool_error_result_with_metadata(
            id,
            format!("game_information_http_{status}"),
            response::status_error_category(status),
            text,
            None,
            Some(status),
        );
    }
    super::tool_result(id, text, false)
}

fn projection_error_result(id: RequestId, message: &'static str) -> RpcResponse {
    let (code, category) = response::projection_error_code(message);
    super::tool_error_result(id, code, category, message)
}

#[cfg(test)]
#[path = "mapping_game_information_binding_tests.rs"]
mod binding_tests;

fn identity<'a>(
    arguments: &'a BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<&'a str, &'static str> {
    read_string(arguments, key)
        .filter(|value| valid_identity(value))
        .ok_or("identity argument is empty, unsafe, or oversized")
}

fn read_string<'a>(arguments: &'a BTreeMap<String, JsonValue>, key: &str) -> Option<&'a str> {
    arguments
        .get(key)
        .and_then(JsonValue::as_string)
        .filter(|value| !value.is_empty())
}

fn valid_identity(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'/' | b'-')
        })
}
