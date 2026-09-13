// SPDX-License-Identifier: MIT

use crate::json::JsonValue;
use crate::protocol::{
    INVALID_PARAMS, METHOD_NOT_FOUND, PARSE_ERROR, RpcError, RpcRequest, RpcResponse,
};
use crate::transport::{FrameCodec, FrameError};

use super::McpServer;

impl<G: crate::gateway::GatewayAdapter> McpServer<G> {
    /// Compatibility entry point: an empty string means no notification response.
    /// Transports should use `handle_message` and emit no bytes for `None`.
    pub fn handle_frame(&mut self, frame: &str) -> String {
        self.handle_message(frame).unwrap_or_default()
    }

    /// Notifications produce no response and never dispatch request-only tools.
    pub fn handle_message(&mut self, frame: &str) -> Option<String> {
        match FrameCodec::decode(frame, self.catalog.max_frame_bytes()) {
            Ok(Some(request)) => Some(FrameCodec::encode(&self.dispatch(request))),
            Ok(None) => None,
            Err(error) => Some(FrameCodec::encode(&RpcResponse::failure(
                None,
                frame_error(error),
            ))),
        }
    }

    fn dispatch(&mut self, request: RpcRequest) -> RpcResponse {
        match request.method.as_str() {
            "initialize" => self.initialize(request),
            "tools/list" => self.tools_list(request),
            "tools/call" => crate::mapping::tools_call(self, request),
            method => RpcResponse::failure(
                Some(request.id),
                RpcError::new(METHOD_NOT_FOUND, unsupported_method(method)),
            ),
        }
    }

    fn initialize(&mut self, request: RpcRequest) -> RpcResponse {
        let Some(params) = request.params.as_object() else {
            return RpcResponse::failure(
                Some(request.id),
                RpcError::new(INVALID_PARAMS, "initialize params must be an object"),
            );
        };
        if !matches!(
            params
                .get("protocolVersion")
                .and_then(JsonValue::as_string),
            Some(value) if !value.is_empty()
        ) {
            return RpcResponse::failure(
                Some(request.id),
                RpcError::new(
                    INVALID_PARAMS,
                    "initialize requires a non-empty protocolVersion",
                ),
            );
        }
        if params
            .get("capabilities")
            .and_then(JsonValue::as_object)
            .is_none()
        {
            return RpcResponse::failure(
                Some(request.id),
                RpcError::new(INVALID_PARAMS, "initialize capabilities must be an object"),
            );
        }
        let Some(client_info) = params.get("clientInfo").and_then(JsonValue::as_object) else {
            return RpcResponse::failure(
                Some(request.id),
                RpcError::new(INVALID_PARAMS, "initialize clientInfo must be an object"),
            );
        };
        let client_info_is_valid = matches!(
            client_info.get("name").and_then(JsonValue::as_string),
            Some(value) if !value.is_empty()
        ) && matches!(
            client_info.get("version").and_then(JsonValue::as_string),
            Some(value) if !value.is_empty()
        );
        if !client_info_is_valid {
            return RpcResponse::failure(
                Some(request.id),
                RpcError::new(
                    INVALID_PARAMS,
                    "initialize clientInfo requires non-empty name and version",
                ),
            );
        }
        let mut capabilities = self.catalog.capabilities.to_json();
        if self.catalog.is_negotiated_composition()
            && let JsonValue::Object(object) = &mut capabilities
        {
            object.insert(
                String::from("tools"),
                JsonValue::object([("listChanged".into(), JsonValue::Bool(true))]),
            );
        }
        let mut result = JsonValue::object([
            (
                "protocolVersion".to_owned(),
                JsonValue::string(super::MCP_PROTOCOL_VERSION),
            ),
            ("capabilities".to_owned(), capabilities),
            (
                "serverInfo".to_owned(),
                JsonValue::object([
                    ("name".to_owned(), JsonValue::string(super::SERVER_NAME)),
                    (
                        "version".to_owned(),
                        JsonValue::string(super::SERVER_VERSION),
                    ),
                ]),
            ),
        ]);
        if self.catalog.is_negotiated_composition()
            && let JsonValue::Object(object) = &mut result
            && let Some(composition) = self.catalog.composition()
        {
            object.insert(String::from("composition"), composition.to_json());
            object.insert(
                String::from("refresh_required"),
                JsonValue::Bool(self.refresh_required()),
            );
            object.insert(
                String::from("session_epoch"),
                JsonValue::Number(i64::try_from(self.session_epoch).unwrap_or(i64::MAX)),
            );
        }
        RpcResponse::success(request.id, result)
    }

    fn tools_list(&self, request: RpcRequest) -> RpcResponse {
        if request.params.as_object().is_none() {
            return RpcResponse::failure(
                Some(request.id),
                RpcError::new(INVALID_PARAMS, "tools/list params must be an object"),
            );
        }
        let mut result = self.catalog.to_json();
        if self.catalog.is_negotiated_composition()
            && let JsonValue::Object(object) = &mut result
        {
            object.insert(
                String::from("refresh_required"),
                JsonValue::Bool(self.refresh_required()),
            );
            object.insert(
                String::from("session_epoch"),
                JsonValue::Number(i64::try_from(self.session_epoch).unwrap_or(i64::MAX)),
            );
        }
        RpcResponse::success(request.id, result)
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
