// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::catalog::{GET_STATE_TOOL, SUBMIT_ACTION_TOOL};
use crate::gateway::{
    Correlation, GatewayAdapter, GatewayError, GatewayMethod, GatewayRequest, GatewayResponse,
};
use crate::json::JsonValue;
use crate::protocol::{
    INVALID_PARAMS, METHOD_NOT_FOUND, RequestId, RpcError, RpcRequest, RpcResponse,
};
use crate::protocol_artifact::{
    POC_ARTIFACT, POC_GENERATOR, POC_PROTOCOL_VERSION, POC_SCHEMA_DIGEST, POC_SCHEMA_SOURCE,
};
use crate::server::McpServer;

const MAX_RESPONSE_BYTES: usize = 16 * 1024;

pub(crate) fn tools_call<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    request: RpcRequest,
) -> RpcResponse {
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
    match tool_name {
        GET_STATE_TOOL => state_call(server, id, arguments),
        SUBMIT_ACTION_TOOL => action_call(server, id, arguments),
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
) -> RpcResponse {
    let (instance_id, session_id) = match request_context(arguments) {
        Ok(context) => context,
        Err(message) => {
            return RpcResponse::failure(Some(id), RpcError::new(INVALID_PARAMS, message));
        }
    };
    let gateway_request = GatewayRequest {
        method: GatewayMethod::Get,
        path: format!("/v1/instances/{instance_id}/state"),
        headers: headers(session_id, &id),
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
) -> RpcResponse {
    let (instance_id, session_id) = match request_context(arguments) {
        Ok(context) => context,
        Err(message) => {
            return RpcResponse::failure(Some(id), RpcError::new(INVALID_PARAMS, message));
        }
    };
    let Some(generation) = nonnegative_integer(arguments, "generation") else {
        return RpcResponse::failure(
            Some(id),
            RpcError::new(INVALID_PARAMS, "generation must be a non-negative integer"),
        );
    };
    let Some(action_id) = non_empty_string(arguments, "action_id") else {
        return RpcResponse::failure(
            Some(id),
            RpcError::new(INVALID_PARAMS, "action_id must be a non-empty string"),
        );
    };
    if action_id != "use_budget" {
        return RpcResponse::failure(
            Some(id),
            RpcError::new(INVALID_PARAMS, "action_id must be use_budget"),
        );
    }
    let Some(units) = bounded_integer(arguments, "units", 0, 8) else {
        return RpcResponse::failure(
            Some(id),
            RpcError::new(INVALID_PARAMS, "units must be an integer between 0 and 8"),
        );
    };
    let gateway_request = GatewayRequest {
        method: GatewayMethod::Post,
        path: format!("/v1/instances/{instance_id}/action"),
        headers: headers(session_id, &id),
        body: Some(poc_action_request(
            &id,
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
        Ok(response) => gateway_success(id, response),
        Err(error) => RpcResponse::failure(Some(id), gateway_error(error)),
    }
}

fn gateway_success(id: RequestId, response: GatewayResponse) -> RpcResponse {
    let body = response.body.to_json();
    if !(200..300).contains(&response.status) || body.len() > MAX_RESPONSE_BYTES {
        return RpcResponse::failure(
            Some(id),
            RpcError::new(-32002, "gateway returned an invalid response"),
        );
    }
    RpcResponse::success(
        id,
        JsonValue::object([
            (
                "content".to_owned(),
                JsonValue::Array(vec![JsonValue::object([
                    ("type".to_owned(), JsonValue::string("text")),
                    ("text".to_owned(), JsonValue::string(body)),
                ])]),
            ),
            ("isError".to_owned(), JsonValue::Bool(false)),
        ]),
    )
}

fn gateway_error(error: GatewayError) -> RpcError {
    let (code, message) = match error {
        GatewayError::Unauthorized => (-32001, "gateway authorization failed"),
        GatewayError::NotFound => (-32004, "gateway target was not found"),
        GatewayError::Unavailable => (-32003, "gateway is unavailable"),
        GatewayError::Timeout => (-32008, "gateway request timed out"),
        GatewayError::MalformedResponse => (-32002, "gateway returned an invalid response"),
        GatewayError::Rejected => (-32005, "gateway rejected the request"),
    };
    RpcError::new(code, message)
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
    if !safe_segment(instance_id) {
        return Err("instance_id contains an unsafe path character");
    }
    Ok((instance_id, session_id))
}

fn nonnegative_integer(arguments: &BTreeMap<String, JsonValue>, key: &str) -> Option<i64> {
    match arguments.get(key) {
        Some(JsonValue::Number(value)) if *value >= 0 => Some(*value),
        _ => None,
    }
}

fn bounded_integer(
    arguments: &BTreeMap<String, JsonValue>,
    key: &str,
    minimum: i64,
    maximum: i64,
) -> Option<i64> {
    nonnegative_integer(arguments, key).filter(|value| (minimum..=maximum).contains(value))
}

fn headers(session_id: &str, id: &RequestId) -> BTreeMap<String, String> {
    BTreeMap::from([
        (String::from("x-mcp-session-id"), String::from(session_id)),
        (String::from("x-mcp-request-id"), id.stable_text()),
    ])
}

fn poc_action_request(
    id: &RequestId,
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
            JsonValue::string(id.stable_text()),
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
    value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}
