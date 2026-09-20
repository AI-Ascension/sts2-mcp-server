// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::catalog::{
    GAME_INFORMATION_BINDING_TOOL, GAME_INFORMATION_CAPABILITIES_TOOL,
    GAME_INFORMATION_CONTENT_MANIFEST_TOOL,
};
use crate::gateway::GatewayAdapter;
use crate::json::JsonValue;
use crate::protocol::{METHOD_NOT_FOUND, RequestId, RpcError, RpcRequest, RpcResponse};
use crate::server::McpServer;

#[path = "mapping_game_information_allowed.rs"]
mod allowed;
pub(super) use allowed::is_tool;
#[path = "mapping_game_information_forward.rs"]
mod forward;
use forward::{forward_binding, forward_capabilities, forward_content_manifest, forward_query};
#[path = "mapping_game_information_request.rs"]
mod request;
#[path = "mapping_game_information_response.rs"]
mod response;
#[path = "mapping_game_information_transport.rs"]
mod transport;

pub(crate) const CAPABILITIES_PATH_SUFFIX: &str = "game-information/capabilities";
pub(crate) const CONTENT_MANIFEST_PATH_SUFFIX: &str = "game-information/content-manifest";
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
    if tool_name == GAME_INFORMATION_CONTENT_MANIFEST_TOOL {
        return forward_content_manifest(server, context);
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
