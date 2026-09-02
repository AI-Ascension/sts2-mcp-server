// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::catalog::ToolCatalog;
use crate::gateway::{
    Correlation, GatewayAdapter, GatewayError, GatewayMethod, GatewayRequest, GatewayResponse,
};
use crate::json::JsonValue;
use crate::protocol::{
    INVALID_PARAMS, METHOD_NOT_FOUND, PARSE_ERROR, RequestId, RpcError, RpcRequest, RpcResponse,
};
use crate::transport::{FrameCodec, FrameError};

pub const SERVER_NAME: &str = "sts2-mcp-server";
pub const SERVER_VERSION: &str = "0.0.0";
const MAX_RESPONSE_BYTES: usize = 16 * 1024;

pub struct McpServer<G> {
    gateway: G,
    catalog: ToolCatalog,
}

impl<G: GatewayAdapter> McpServer<G> {
    pub fn new(gateway: G) -> Self {
        Self {
            gateway,
            catalog: ToolCatalog::default(),
        }
    }

    pub fn with_catalog(gateway: G, catalog: ToolCatalog) -> Self {
        Self { gateway, catalog }
    }

    pub fn handle_frame(&mut self, frame: &str) -> String {
        match FrameCodec::decode(frame) {
            Ok(request) => FrameCodec::encode(&self.dispatch(request)),
            Err(error) => FrameCodec::encode(&RpcResponse::failure(None, frame_error(error))),
        }
    }

    pub fn catalog(&self) -> &ToolCatalog {
        &self.catalog
    }

    pub fn gateway(&self) -> &G {
        &self.gateway
    }

    fn dispatch(&mut self, request: RpcRequest) -> RpcResponse {
        match request.method.as_str() {
            "initialize" => self.initialize(request),
            "tools/list" => self.tools_list(request),
            "tools/call" => self.tools_call(request),
            method => RpcResponse::failure(
                Some(request.id),
                RpcError::new(METHOD_NOT_FOUND, unsupported_method(method)),
            ),
        }
    }

    fn initialize(&self, request: RpcRequest) -> RpcResponse {
        if request.params.as_object().is_none() {
            return RpcResponse::failure(
                Some(request.id),
                RpcError::new(INVALID_PARAMS, "initialize params must be an object"),
            );
        }
        let result = JsonValue::object([
            (
                "protocolVersion".to_owned(),
                JsonValue::string(self.catalog.revision.as_str()),
            ),
            (
                "capabilities".to_owned(),
                self.catalog.capabilities.to_json(),
            ),
            (
                "serverInfo".to_owned(),
                JsonValue::object([
                    ("name".to_owned(), JsonValue::string(SERVER_NAME)),
                    ("version".to_owned(), JsonValue::string(SERVER_VERSION)),
                ]),
            ),
        ]);
        RpcResponse::success(request.id, result)
    }

    fn tools_list(&self, request: RpcRequest) -> RpcResponse {
        if request.params.as_object().is_none() {
            return RpcResponse::failure(
                Some(request.id),
                RpcError::new(INVALID_PARAMS, "tools/list params must be an object"),
            );
        }
        RpcResponse::success(request.id, self.catalog.to_json())
    }

    fn tools_call(&mut self, request: RpcRequest) -> RpcResponse {
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
        if self.catalog.descriptor(tool_name).is_none() {
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
        let Some(instance_id) = non_empty_string(arguments, "instance_id") else {
            return RpcResponse::failure(
                Some(request.id),
                RpcError::new(INVALID_PARAMS, "instance_id must be a non-empty string"),
            );
        };
        let Some(session_id) = non_empty_string(arguments, "mcp_session_id") else {
            return RpcResponse::failure(
                Some(request.id),
                RpcError::new(INVALID_PARAMS, "mcp_session_id must be a non-empty string"),
            );
        };
        if !safe_segment(instance_id) {
            return RpcResponse::failure(
                Some(request.id),
                RpcError::new(
                    INVALID_PARAMS,
                    "instance_id contains an unsafe path character",
                ),
            );
        }
        let gateway_request = GatewayRequest {
            method: GatewayMethod::Get,
            path: format!("/v1/instances/{instance_id}/state"),
            headers: BTreeMap::from([
                (String::from("x-mcp-session-id"), String::from(session_id)),
                (String::from("x-mcp-request-id"), request.id.stable_text()),
            ]),
            body: None,
            correlation: Correlation {
                mcp_session_id: String::from(session_id),
                mcp_request_id: request.id.clone(),
            },
        };
        match self.gateway.forward(gateway_request) {
            Ok(response) => self.gateway_success(request.id, response),
            Err(error) => RpcResponse::failure(Some(request.id), gateway_error(error)),
        }
    }

    fn gateway_success(&self, id: RequestId, response: GatewayResponse) -> RpcResponse {
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
}

fn frame_error(error: FrameError) -> RpcError {
    match error {
        FrameError::TooLarge => RpcError::new(PARSE_ERROR, "MCP frame exceeds the byte limit"),
        FrameError::MultipleLines => {
            RpcError::new(PARSE_ERROR, "MCP frame must contain one JSON value")
        }
        FrameError::InvalidJson => RpcError::new(PARSE_ERROR, "MCP frame is not valid JSON"),
        FrameError::InvalidRequest => RpcError::new(-32600, "MCP request shape is invalid"),
    }
}

fn unsupported_method(method: &str) -> String {
    let method: String = method.chars().take(64).collect();
    format!("capability or method is not supported: {method}")
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

fn safe_segment(value: &str) -> bool {
    value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

#[cfg(test)]
mod tests {
    use super::safe_segment;

    #[test]
    fn accepts_only_path_safe_instance_segments() {
        assert!(safe_segment("instance-1_alpha"));
        assert!(!safe_segment("../instance"));
        assert!(!safe_segment("instance/child"));
    }
}
