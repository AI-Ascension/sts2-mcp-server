// SPDX-License-Identifier: MIT

use crate::catalog::{
    SAVE_PROFILE_CREATE_DISPOSABLE_TOOL, SAVE_PROFILE_CURRENT_TOOL, SAVE_PROFILE_LIST_TOOL,
    SAVE_PROFILE_SELECT_TOOL, SAVE_PROFILE_STATUS_TOOL,
};
use crate::gateway::{GatewayAdapter, GatewayError, GatewayRequest};
use crate::json::JsonValue;
use crate::protocol::{
    INVALID_PARAMS, METHOD_NOT_FOUND, RequestId, RpcError, RpcRequest, RpcResponse,
};
use crate::server::McpServer;

#[path = "mapping_save_profile_request.rs"]
mod request;
#[path = "mapping_save_profile_response.rs"]
mod response;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum CallKind {
    List,
    Current,
    Select,
    CreateDisposable,
    Status,
}

impl CallKind {
    pub(super) const fn is_mutation(self) -> bool {
        matches!(self, Self::Select | Self::CreateDisposable)
    }

    pub(super) const fn expected_route(self) -> &'static str {
        match self {
            Self::List => "list",
            Self::Current => "current",
            Self::Select => "select",
            Self::CreateDisposable => "createdisposable",
            Self::Status => "lookup",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Context {
    pub(super) instance_id: String,
    pub(super) mcp_session_id: String,
    pub(super) gateway_session_id: String,
    pub(super) lease_id: String,
    pub(super) lease_epoch: i64,
    pub(super) correlation_id: String,
    pub(super) operation_id: String,
    pub(super) requested_profile: Option<String>,
    pub(super) kind: CallKind,
    pub(super) request_id: RequestId,
}

pub(super) fn tools_call<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    request: RpcRequest,
) -> RpcResponse {
    let Some(params) = request.params.as_object() else {
        return invalid_params(
            request.id,
            "save-profile tools/call params must be an object",
        );
    };
    if !super::has_only_arguments(params, &["name", "arguments"]) {
        return invalid_params(request.id, "save-profile tools/call has unsupported fields");
    }
    let request_id = request.id.clone();
    let Some(name) = params.get("name").and_then(JsonValue::as_string) else {
        return invalid_params(request_id, "tools/call requires a tool name");
    };
    let Some(kind) = kind_for(name) else {
        return RpcResponse::failure(
            Some(request_id),
            RpcError::new(METHOD_NOT_FOUND, "save-profile tool is not active"),
        );
    };
    if server.catalog.descriptor(name).is_none() {
        return RpcResponse::failure(
            Some(request_id),
            RpcError::new(
                METHOD_NOT_FOUND,
                "save-profile capability is not advertised",
            ),
        );
    }
    let Some(arguments) = params.get("arguments").and_then(JsonValue::as_object) else {
        return invalid_params(request_id, "save-profile arguments must be an object");
    };
    let allowed = match kind {
        CallKind::Select => &request::SELECT_ARGUMENTS[..],
        CallKind::Status => &request::STATUS_ARGUMENTS[..],
        _ => &request::COMMON_ARGUMENTS[..],
    };
    if !super::has_only_arguments(arguments, allowed) {
        return invalid_params(
            request_id,
            "save-profile arguments contain an unsupported field",
        );
    }
    let id = request.id;
    let correlation_id = id.stable_text();
    if !request::safe_gateway_identity(&correlation_id) {
        return invalid_params(
            id,
            "request id contains an unsafe or oversized header value",
        );
    }
    let context = match request::context(server, arguments, correlation_id, id.clone(), kind) {
        Ok(value) => value,
        Err(message) => return invalid_params(id, message),
    };
    let gateway_request = match request::gateway_request(&context, arguments) {
        Ok(request) => request,
        Err(message) => return invalid_params(context.request_id.clone(), message),
    };
    forward(server, context, gateway_request)
}

fn forward<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    context: Context,
    request: GatewayRequest,
) -> RpcResponse {
    match server.gateway.forward(request) {
        Ok(response) => response::gateway_success(context, response),
        Err(error) if context.kind.is_mutation() && uncertain(error) => {
            response::unknown_result(context, error)
        }
        Err(error) => super::gateway_error_result(context.request_id, error),
    }
}

fn kind_for(name: &str) -> Option<CallKind> {
    match name {
        SAVE_PROFILE_LIST_TOOL => Some(CallKind::List),
        SAVE_PROFILE_CURRENT_TOOL => Some(CallKind::Current),
        SAVE_PROFILE_SELECT_TOOL => Some(CallKind::Select),
        SAVE_PROFILE_CREATE_DISPOSABLE_TOOL => Some(CallKind::CreateDisposable),
        SAVE_PROFILE_STATUS_TOOL => Some(CallKind::Status),
        _ => None,
    }
}

fn uncertain(error: GatewayError) -> bool {
    matches!(
        error,
        GatewayError::Timeout
            | GatewayError::Unavailable
            | GatewayError::MalformedResponse
            | GatewayError::ResponseTooLarge
    )
}

fn invalid_params(id: RequestId, message: impl Into<String>) -> RpcResponse {
    RpcResponse::failure(Some(id), RpcError::new(INVALID_PARAMS, message))
}
