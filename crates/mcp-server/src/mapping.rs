// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::catalog::{GET_STATE_TOOL, MAX_IDENTIFIER_BYTES, SUBMIT_ACTION_TOOL};
use crate::gateway::{Correlation, GatewayAdapter, GatewayError, GatewayMethod, GatewayRequest};
use crate::json::JsonValue;
use crate::protocol::{
    INVALID_PARAMS, METHOD_NOT_FOUND, RequestId, RpcError, RpcRequest, RpcResponse,
};
use crate::protocol_artifact::{
    POC_ARTIFACT, POC_GENERATOR, POC_MAX_GENERATION, POC_PROTOCOL_VERSION, POC_SCHEMA_DIGEST,
    POC_SCHEMA_SOURCE,
};
use crate::server::McpServer;

#[path = "mapping_response.rs"]
mod response;
#[path = "mapping_runtime.rs"]
mod runtime;
#[path = "mapping_runtime_v2.rs"]
mod runtime_v2;
#[path = "mapping_runtime_v3_gameplay.rs"]
mod runtime_v3_gameplay;
#[path = "mapping_coop_gameplay.rs"]
mod coop_gameplay;

pub(crate) fn tools_call<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    request: RpcRequest,
) -> RpcResponse {
    if server.catalog.is_coop_gameplay() {
        return coop_gameplay::tools_call(server, request);
    }
    if server.catalog.is_runtime_v3_gameplay() {
        return runtime_v3_gameplay::tools_call(server, request);
    }
    if server.catalog.is_runtime_v2() {
        return runtime_v2::tools_call(server, request);
    }
    let Some(params) = request.params.as_object() else {
        return RpcResponse::failure(
            Some(request.id),
            RpcError::new(INVALID_PARAMS, "tools/call params must be an object"),
        );
    };
    let Some(tool_name) = params.get("name").and_then(JsonValue::as_string) else {
        return RpcResponse::failure(
            Some(request.id),
            RpcError::new(INVALID_PARAMS, "tools/call requires a tool name"),
        );
    };
    if server.catalog.descriptor(tool_name).is_none() {
        return RpcResponse::failure(
            Some(request.id),
            RpcError::new(METHOD_NOT_FOUND, "tool is not in the active catalog"),
        );
    }
    let Some(arguments) = params.get("arguments").and_then(JsonValue::as_object) else {
        return RpcResponse::failure(
            Some(request.id),
            RpcError::new(INVALID_PARAMS, "tools/call arguments must be an object"),
        );
    };
    let id = request.id;
    let correlation_id = id.stable_text();
    if !safe_header_value(&correlation_id) {
        return invalid_params(
            id,
            "request id contains an unsafe or oversized header value",
        );
    }
    match tool_name {
        GET_STATE_TOOL => state_call(server, id, arguments, &correlation_id),
        SUBMIT_ACTION_TOOL if server.catalog.is_runtime_v1() => {
            runtime::runtime_action_call(server, id, arguments, &correlation_id)
        }
        SUBMIT_ACTION_TOOL => action_call(server, id, arguments, &correlation_id),
        _ => RpcResponse::failure(
            Some(id),
            RpcError::new(METHOD_NOT_FOUND, "tool is not in the active catalog"),
        ),
    }
}

fn state_call<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    id: RequestId,
    arguments: &BTreeMap<String, JsonValue>,
    correlation_id: &str,
) -> RpcResponse {
    if !has_only_arguments(arguments, &["instance_id", "mcp_session_id"]) {
        return invalid_params(id, "tools/call arguments contain an unsupported field");
    }
    let (instance_id, session_id) = match request_context(arguments) {
        Ok(context) => context,
        Err(message) => {
            return invalid_params(id, message);
        }
    };
    let gateway_request = GatewayRequest {
        method: GatewayMethod::Get,
        path: format!("/v1/instances/{instance_id}/state"),
        headers: headers(session_id, correlation_id),
        body: None,
        correlation: Correlation {
            mcp_session_id: String::from(session_id),
            mcp_request_id: id.clone(),
        },
    };
    forward(server, id, gateway_request)
}

fn action_call<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    id: RequestId,
    arguments: &BTreeMap<String, JsonValue>,
    correlation_id: &str,
) -> RpcResponse {
    if !has_only_arguments(
        arguments,
        &[
            "instance_id",
            "mcp_session_id",
            "generation",
            "action_id",
            "units",
        ],
    ) {
        return invalid_params(id, "tools/call arguments contain an unsupported field");
    }
    let (instance_id, session_id) = match request_context(arguments) {
        Ok(context) => context,
        Err(message) => {
            return invalid_params(id, message);
        }
    };
    let Some(generation) =
        nonnegative_integer(arguments, "generation").filter(|value| *value <= POC_MAX_GENERATION)
    else {
        return invalid_params(id, "generation exceeds the protocol bound");
    };
    let Some(action_id) = non_empty_string(arguments, "action_id") else {
        return invalid_params(id, "action_id must be a non-empty string");
    };
    if action_id != "use_budget" {
        return invalid_params(id, "action_id must be use_budget");
    }
    let Some(units) =
        nonnegative_integer(arguments, "units").filter(|value| (0..=8).contains(value))
    else {
        return invalid_params(id, "units must be an integer between 0 and 8");
    };
    let gateway_request = GatewayRequest {
        method: GatewayMethod::Post,
        path: format!("/v1/instances/{instance_id}/action"),
        headers: headers(session_id, correlation_id),
        body: Some(poc_action_request(
            correlation_id,
            instance_id,
            generation,
            action_id,
            units,
        )),
        correlation: Correlation {
            mcp_session_id: String::from(session_id),
            mcp_request_id: id.clone(),
        },
    };
    forward(server, id, gateway_request)
}

fn forward<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    id: RequestId,
    request: GatewayRequest,
) -> RpcResponse {
    match server.gateway.forward(request) {
        Ok(response) => response::gateway_success(id, response, server.catalog.is_runtime_v1()),
        Err(error) => gateway_error_result(id, error),
    }
}

fn gateway_error_result(id: RequestId, error: GatewayError) -> RpcResponse {
    let (code, message) = match error {
        GatewayError::Unauthorized => (-32001, "gateway authorization failed"),
        GatewayError::Forbidden => (-32007, "gateway scope authorization failed"),
        GatewayError::NotFound => (-32004, "gateway target was not found"),
        GatewayError::Unavailable => (-32003, "gateway is unavailable"),
        GatewayError::Timeout => (-32008, "gateway request timed out"),
        GatewayError::MalformedResponse => (-32002, "gateway returned an invalid response"),
        GatewayError::Rejected => (-32005, "gateway rejected the request"),
    };
    tool_result(id, format!("gateway error {code}: {message}"), true)
}

fn tool_result(id: RequestId, text: impl Into<String>, is_error: bool) -> RpcResponse {
    RpcResponse::success(
        id,
        JsonValue::object([
            (
                "content".to_owned(),
                JsonValue::Array(vec![JsonValue::object([
                    ("type".to_owned(), JsonValue::string("text")),
                    ("text".to_owned(), JsonValue::string(text)),
                ])]),
            ),
            ("isError".to_owned(), JsonValue::Bool(is_error)),
        ]),
    )
}

fn invalid_params(id: RequestId, message: impl Into<String>) -> RpcResponse {
    RpcResponse::failure(Some(id), RpcError::new(INVALID_PARAMS, message))
}

fn has_only_arguments(arguments: &BTreeMap<String, JsonValue>, allowed: &[&str]) -> bool {
    arguments.keys().all(|key| allowed.contains(&key.as_str()))
}

fn non_empty_string<'a>(arguments: &'a BTreeMap<String, JsonValue>, key: &str) -> Option<&'a str> {
    arguments
        .get(key)
        .and_then(JsonValue::as_string)
        .filter(|value| !value.is_empty())
}

fn request_context(arguments: &BTreeMap<String, JsonValue>) -> Result<(&str, &str), &'static str> {
    let instance_id = non_empty_string(arguments, "instance_id")
        .ok_or("instance_id must be a non-empty string")?;
    let session_id = non_empty_string(arguments, "mcp_session_id")
        .ok_or("mcp_session_id must be a non-empty string")?;
    if !safe_segment(instance_id) || !safe_header_value(session_id) {
        return Err("instance_id or mcp_session_id contains an unsafe or oversized value");
    }
    Ok((instance_id, session_id))
}

fn nonnegative_integer(arguments: &BTreeMap<String, JsonValue>, key: &str) -> Option<i64> {
    match arguments.get(key) {
        Some(JsonValue::Number(value)) if *value >= 0 => Some(*value),
        _ => None,
    }
}

fn headers(session_id: &str, correlation_id: &str) -> BTreeMap<String, String> {
    BTreeMap::from([
        (String::from("x-mcp-session-id"), String::from(session_id)),
        (
            String::from("x-mcp-request-id"),
            String::from(correlation_id),
        ),
    ])
}

fn poc_action_request(
    correlation_id: &str,
    instance_id: &str,
    generation: i64,
    action_id: &str,
    units: i64,
) -> JsonValue {
    JsonValue::object([
        (
            "action".to_owned(),
            JsonValue::object([
                ("action_id".to_owned(), JsonValue::string(action_id)),
                ("units".to_owned(), JsonValue::Number(units)),
            ]),
        ),
        (
            "correlation_id".to_owned(),
            JsonValue::string(correlation_id),
        ),
        ("error_code".to_owned(), JsonValue::Null),
        ("generation".to_owned(), JsonValue::Number(generation)),
        ("instance_id".to_owned(), JsonValue::string(instance_id)),
        ("kind".to_owned(), JsonValue::string("action_request")),
        ("observation".to_owned(), JsonValue::Null),
        (
            "protocol_version".to_owned(),
            JsonValue::string(POC_PROTOCOL_VERSION),
        ),
        (
            "provenance".to_owned(),
            JsonValue::object([
                ("artifact".to_owned(), JsonValue::string(POC_ARTIFACT)),
                ("generator".to_owned(), JsonValue::string(POC_GENERATOR)),
                ("source".to_owned(), JsonValue::string(POC_SCHEMA_SOURCE)),
            ]),
        ),
        (
            "schema_digest".to_owned(),
            JsonValue::string(POC_SCHEMA_DIGEST),
        ),
        ("status".to_owned(), JsonValue::Null),
    ])
}

pub(crate) fn safe_segment(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_IDENTIFIER_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn safe_header_value(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_IDENTIFIER_BYTES
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':' | b'/')
        })
}
