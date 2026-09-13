// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::catalog::MAX_IDENTIFIER_BYTES;
use crate::gateway::{GatewayAdapter, GatewayError, GatewayRequest};
use crate::json::JsonValue;
use crate::protocol::{INVALID_PARAMS, RequestId, RpcError, RpcResponse};
use crate::protocol_artifact::{
    POC_ARTIFACT, POC_GENERATOR, POC_PROTOCOL_VERSION, POC_SCHEMA_DIGEST, POC_SCHEMA_SOURCE,
};
use crate::server::McpServer;

pub(super) fn forward<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    id: RequestId,
    request: GatewayRequest,
) -> RpcResponse {
    match server.gateway.forward(request) {
        Ok(response) => {
            super::response::gateway_success(id, response, server.catalog.is_runtime_v1())
        }
        Err(error) => gateway_error_result(id, error),
    }
}

pub(super) fn gateway_error_result(id: RequestId, error: GatewayError) -> RpcResponse {
    let (mcp_code, code, category, message) = match error {
        GatewayError::Unauthorized => (
            -32001,
            "gateway_unauthorized",
            "denied",
            "gateway authorization failed",
        ),
        GatewayError::Forbidden => (
            -32007,
            "gateway_forbidden",
            "denied",
            "gateway scope authorization failed",
        ),
        GatewayError::NotFound => (
            -32004,
            "gateway_not_found",
            "missing",
            "gateway target was not found",
        ),
        GatewayError::Unavailable => (
            -32003,
            "gateway_unavailable",
            "transport",
            "gateway is unavailable",
        ),
        GatewayError::Timeout => (
            -32008,
            "gateway_timeout",
            "transport",
            "gateway request timed out",
        ),
        GatewayError::MalformedResponse => (
            -32002,
            "gateway_malformed_response",
            "malformed_response",
            "gateway returned an invalid response",
        ),
        GatewayError::ResponseTooLarge => (
            -32006,
            "gateway_response_too_large",
            "size",
            "gateway response exceeded the byte limit",
        ),
        GatewayError::Rejected => (
            -32005,
            "gateway_rejected",
            "invalid_input",
            "gateway rejected the request",
        ),
    };
    tool_error_result_with_metadata(
        id,
        code,
        category,
        format!("gateway error {mcp_code}: {message}"),
        Some(mcp_code),
        None,
    )
}

pub(super) fn tool_result(id: RequestId, text: impl Into<String>, is_error: bool) -> RpcResponse {
    let text = text.into();
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

/// Returns a tool error with a machine-readable error object in
/// MCP `structuredContent`, while retaining the bounded text content for
/// clients that only consume the textual channel.
pub(super) fn tool_error_result(
    id: RequestId,
    code: impl Into<String>,
    category: &'static str,
    text: impl Into<String>,
) -> RpcResponse {
    tool_error_result_with_metadata(id, code, category, text, None, None)
}

pub(super) fn tool_error_result_with_metadata(
    id: RequestId,
    code: impl Into<String>,
    category: &'static str,
    text: impl Into<String>,
    mcp_code: Option<i32>,
    http_status: Option<u16>,
) -> RpcResponse {
    let code = code.into();
    let text = text.into();
    RpcResponse::success(
        id,
        JsonValue::object([
            (
                "content".to_owned(),
                JsonValue::Array(vec![JsonValue::object([
                    ("type".to_owned(), JsonValue::string("text")),
                    ("text".to_owned(), JsonValue::string(text.as_str())),
                ])]),
            ),
            ("isError".to_owned(), JsonValue::Bool(true)),
            (
                "structuredContent".to_owned(),
                JsonValue::object([(
                    "error".to_owned(),
                    JsonValue::object([
                        ("category".to_owned(), JsonValue::string(category)),
                        ("code".to_owned(), JsonValue::string(code.as_str())),
                        (
                            "http_status".to_owned(),
                            http_status.map_or(JsonValue::Null, |status| {
                                JsonValue::Number(i64::from(status))
                            }),
                        ),
                        (
                            "mcp_code".to_owned(),
                            mcp_code
                                .map_or(JsonValue::Null, |code| JsonValue::Number(i64::from(code))),
                        ),
                        ("message".to_owned(), JsonValue::string(text)),
                    ]),
                )]),
            ),
        ]),
    )
}

pub(super) fn invalid_params(id: RequestId, message: impl Into<String>) -> RpcResponse {
    RpcResponse::failure(Some(id), RpcError::new(INVALID_PARAMS, message))
}

pub(super) fn has_only_arguments(
    arguments: &BTreeMap<String, JsonValue>,
    allowed: &[&str],
) -> bool {
    arguments.keys().all(|key| allowed.contains(&key.as_str()))
}

pub(super) fn non_empty_string<'a>(
    arguments: &'a BTreeMap<String, JsonValue>,
    key: &str,
) -> Option<&'a str> {
    arguments
        .get(key)
        .and_then(JsonValue::as_string)
        .filter(|value| !value.is_empty())
}

pub(super) fn request_context(
    arguments: &BTreeMap<String, JsonValue>,
) -> Result<(&str, &str), &'static str> {
    let instance_id = non_empty_string(arguments, "instance_id")
        .ok_or("instance_id must be a non-empty string")?;
    let session_id = non_empty_string(arguments, "mcp_session_id")
        .ok_or("mcp_session_id must be a non-empty string")?;
    if !safe_segment(instance_id) || !safe_header_value(session_id) {
        return Err("instance_id or mcp_session_id contains an unsafe or oversized value");
    }
    Ok((instance_id, session_id))
}

pub(super) fn nonnegative_integer(
    arguments: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Option<i64> {
    match arguments.get(key) {
        Some(JsonValue::Number(value)) if *value >= 0 => Some(*value),
        _ => None,
    }
}

pub(super) fn headers(session_id: &str, correlation_id: &str) -> BTreeMap<String, String> {
    BTreeMap::from([
        (String::from("x-mcp-session-id"), String::from(session_id)),
        (
            String::from("x-mcp-request-id"),
            String::from(correlation_id),
        ),
    ])
}

pub(super) fn poc_action_request(
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

pub(super) fn safe_header_value(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_IDENTIFIER_BYTES
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':' | b'/')
        })
}
