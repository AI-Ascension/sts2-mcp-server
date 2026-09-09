// SPDX-License-Identifier: MIT

use std::collections::{BTreeMap, BTreeSet};

use crate::catalog::ToolCatalog;
use crate::gateway::GatewayAdapter;
use crate::json::JsonValue;
use crate::projection::{RestActionSelectionAdmission, RestActionSelectionKey};
use crate::protocol::{
    INVALID_PARAMS, METHOD_NOT_FOUND, PARSE_ERROR, RpcError, RpcRequest, RpcResponse,
};
use crate::transport::{FrameCodec, FrameError};

#[path = "server_runtime_v4_expert_rest_action.rs"]
mod runtime_v4_expert_rest_action;
pub(crate) use runtime_v4_expert_rest_action::RestActionOperationContext;

#[path = "server_runtime_v4_expert_rest_action_capacity.rs"]
mod runtime_v4_expert_rest_action_capacity;
pub(crate) use runtime_v4_expert_rest_action_capacity::REST_ACTION_SELECTOR_CAPACITY_ERROR;

#[cfg(test)]
#[path = "server_tests.rs"]
mod tests;

pub const SERVER_NAME: &str = "sts2-mcp-server";
pub const SERVER_VERSION: &str = "0.0.0";
pub const MCP_PROTOCOL_VERSION: &str = "2025-06-18";

pub struct McpServer<G> {
    pub(crate) gateway: G,
    pub(crate) catalog: ToolCatalog,
    pub(crate) gateway_session_id: Option<String>,
    pub(crate) mcp_session_id: Option<String>,
    pub(crate) rest_action_selections: BTreeMap<RestActionSelectionKey, RestActionSelectionContext>,
    pub(crate) rest_action_operations: BTreeMap<String, RestActionOperationContext>,
    pub(crate) rest_action_selector_reservations: BTreeSet<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RestActionSelectionContext {
    pub(crate) admission: RestActionSelectionAdmission,
    pub(crate) terminal: bool,
}

const MAX_REST_ACTION_SELECTIONS: usize = 128;

impl<G: GatewayAdapter> McpServer<G> {
    pub fn new(gateway: G) -> Self {
        Self {
            gateway,
            catalog: ToolCatalog::default(),
            gateway_session_id: None,
            mcp_session_id: None,
            rest_action_selections: BTreeMap::new(),
            rest_action_operations: BTreeMap::new(),
            rest_action_selector_reservations: BTreeSet::new(),
        }
    }

    pub fn with_catalog(gateway: G, catalog: ToolCatalog) -> Self {
        Self {
            gateway,
            catalog,
            gateway_session_id: None,
            mcp_session_id: None,
            rest_action_selections: BTreeMap::new(),
            rest_action_operations: BTreeMap::new(),
            rest_action_selector_reservations: BTreeSet::new(),
        }
    }

    /// Binds the process to one gateway session and one MCP session.
    ///
    /// The two values intentionally remain separate: the gateway session is
    /// placed in the Runtime-v2 envelope, while the MCP session is carried in
    /// the adapter correlation/header seam. Callers must validate both values
    /// before constructing the server.
    pub fn with_catalog_and_sessions(
        gateway: G,
        catalog: ToolCatalog,
        gateway_session_id: impl Into<String>,
        mcp_session_id: impl Into<String>,
    ) -> Self {
        Self {
            gateway,
            catalog,
            gateway_session_id: Some(gateway_session_id.into()),
            mcp_session_id: Some(mcp_session_id.into()),
            rest_action_selections: BTreeMap::new(),
            rest_action_operations: BTreeMap::new(),
            rest_action_selector_reservations: BTreeSet::new(),
        }
    }

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

    pub fn catalog(&self) -> &ToolCatalog {
        &self.catalog
    }

    pub fn gateway(&self) -> &G {
        &self.gateway
    }

    pub(crate) fn gateway_session_id(&self) -> Option<&str> {
        self.gateway_session_id.as_deref()
    }

    pub(crate) fn mcp_session_id(&self) -> Option<&str> {
        self.mcp_session_id.as_deref()
    }

    pub(crate) fn rest_action_selection_admission(
        &self,
        instance_id: &str,
        session_id: &str,
        lease_id: &str,
        lease_epoch: i64,
        body: &JsonValue,
    ) -> Option<&RestActionSelectionAdmission> {
        let selection_id = crate::projection::rest_action_selection_id(body)?;
        self.rest_action_selections
            .get(&RestActionSelectionKey {
                instance_id: instance_id.to_owned(),
                session_id: session_id.to_owned(),
                lease_id: lease_id.to_owned(),
                lease_epoch,
                selection_id: selection_id.to_owned(),
            })
            .map(|context| &context.admission)
    }

    pub(crate) fn remember_rest_action_selection(
        &mut self,
        instance_id: &str,
        session_id: &str,
        lease_id: &str,
        lease_epoch: i64,
        body: &JsonValue,
    ) -> bool {
        let Some(selection_id) = crate::projection::rest_action_selection_id(body) else {
            return true;
        };
        let key = RestActionSelectionKey {
            instance_id: instance_id.to_owned(),
            session_id: session_id.to_owned(),
            lease_id: lease_id.to_owned(),
            lease_epoch,
            selection_id: selection_id.to_owned(),
        };
        let terminal = body
            .as_object()
            .and_then(|root| root.get("transition"))
            .and_then(JsonValue::as_object)
            .and_then(|transition| transition.get("kind"))
            .and_then(JsonValue::as_string)
            == Some("rest_option_selection_completed");
        if terminal {
            let Some(context) = self.rest_action_selections.get_mut(&key) else {
                return false;
            };
            context.terminal = true;
            return true;
        }
        let Some((_, admission)) = crate::projection::rest_action_selection_admission(body) else {
            return true;
        };
        if self.rest_action_selections.len() >= MAX_REST_ACTION_SELECTIONS
            && !self.rest_action_selections.contains_key(&key)
        {
            let Some(eviction_key) = self
                .rest_action_selections
                .iter()
                .find_map(|(key, context)| context.terminal.then(|| key.clone()))
            else {
                // Never discard an active catalog. The caller must fail closed
                // rather than surface a selector it cannot later reconcile.
                return false;
            };
            self.rest_action_selections.remove(&eviction_key);
        }
        self.rest_action_selections.insert(
            key,
            RestActionSelectionContext {
                admission,
                terminal: false,
            },
        );
        true
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

    fn initialize(&self, request: RpcRequest) -> RpcResponse {
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
        let result = JsonValue::object([
            (
                "protocolVersion".to_owned(),
                JsonValue::string(MCP_PROTOCOL_VERSION),
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
