// SPDX-License-Identifier: MIT

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::catalog::{CapabilityScope, ToolCatalog, ToolLimits};
use crate::gateway::{GatewayAdapter, GatewayError, GatewayRequest, GatewayResponse};
use crate::projection::{RestActionSelectionAdmission, RestActionSelectionKey};

#[path = "server_runtime_v4_expert_rest_action.rs"]
mod runtime_v4_expert_rest_action;
pub(crate) use runtime_v4_expert_rest_action::{
    RestActionOperationContext, RestActionOperationSelection,
};

#[path = "server_runtime_v4_expert_rest_action_capacity.rs"]
mod runtime_v4_expert_rest_action_capacity;
pub(crate) use runtime_v4_expert_rest_action_capacity::REST_ACTION_SELECTOR_CAPACITY_ERROR;
#[path = "server_protocol.rs"]
mod server_protocol;
#[path = "server_session.rs"]
mod session;
pub use session::{SessionEvent, SessionRefreshReason, SessionUpdate};

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
    pub(crate) native_peer_id: Option<String>,
    pub(crate) rest_action_selections: BTreeMap<RestActionSelectionKey, RestActionSelectionContext>,
    pub(crate) rest_action_operations: BTreeMap<String, RestActionOperationContext>,
    pub(crate) rest_action_selector_reservations: BTreeSet<String>,
    pub(crate) session_epoch: u64,
    pub(crate) refresh_required: Option<SessionRefreshReason>,
    pub(crate) active_snapshots: BTreeSet<String>,
    pub(crate) invalidated_snapshots: BTreeSet<String>,
    pub(crate) snapshot_tracking_exhausted: bool,
    pub(crate) notifications: VecDeque<String>,
    pub(crate) dispatch_operation: Option<String>,
    pub(crate) pending_revision: Option<String>,
    pub(crate) authority_scope: CapabilityScope,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RestActionSelectionContext {
    pub(crate) admission: RestActionSelectionAdmission,
    pub(crate) generation: i64,
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
            native_peer_id: None,
            rest_action_selections: BTreeMap::new(),
            rest_action_operations: BTreeMap::new(),
            rest_action_selector_reservations: BTreeSet::new(),
            session_epoch: 0,
            refresh_required: None,
            active_snapshots: BTreeSet::new(),
            invalidated_snapshots: BTreeSet::new(),
            snapshot_tracking_exhausted: false,
            notifications: VecDeque::new(),
            dispatch_operation: None,
            pending_revision: None,
            authority_scope: CapabilityScope::ALL,
        }
    }

    pub fn with_catalog(gateway: G, catalog: ToolCatalog) -> Self {
        let authority_scope = catalog_authority_scope(&catalog);
        Self {
            gateway,
            catalog,
            gateway_session_id: None,
            mcp_session_id: None,
            native_peer_id: None,
            rest_action_selections: BTreeMap::new(),
            rest_action_operations: BTreeMap::new(),
            rest_action_selector_reservations: BTreeSet::new(),
            session_epoch: 0,
            refresh_required: None,
            active_snapshots: BTreeSet::new(),
            invalidated_snapshots: BTreeSet::new(),
            snapshot_tracking_exhausted: false,
            notifications: VecDeque::new(),
            dispatch_operation: None,
            pending_revision: None,
            authority_scope,
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
        let authority_scope = catalog_authority_scope(&catalog);
        Self {
            gateway,
            catalog,
            gateway_session_id: Some(gateway_session_id.into()),
            mcp_session_id: Some(mcp_session_id.into()),
            native_peer_id: None,
            rest_action_selections: BTreeMap::new(),
            rest_action_operations: BTreeMap::new(),
            rest_action_selector_reservations: BTreeSet::new(),
            session_epoch: 0,
            refresh_required: None,
            active_snapshots: BTreeSet::new(),
            invalidated_snapshots: BTreeSet::new(),
            snapshot_tracking_exhausted: false,
            notifications: VecDeque::new(),
            dispatch_operation: None,
            pending_revision: None,
            authority_scope,
        }
    }

    /// Binds native response attribution to the peer selected by gateway
    /// configuration. This value is never a caller-provided tool argument.
    pub fn with_native_peer_id(mut self, peer_id: impl Into<String>) -> Self {
        self.native_peer_id = Some(peer_id.into());
        self
    }

    pub fn catalog(&self) -> &ToolCatalog {
        &self.catalog
    }

    pub fn gateway(&self) -> &G {
        &self.gateway
    }

    pub(crate) fn negotiated_limits(&self, operation: &str) -> Option<ToolLimits> {
        self.catalog
            .negotiated_capabilities()?
            .available(operation)
            .map(|operation| operation.limits)
    }

    pub(crate) fn with_dispatch_operation<R>(
        &mut self,
        operation: &str,
        callback: impl FnOnce(&mut Self) -> R,
    ) -> R {
        let previous = self.dispatch_operation.replace(operation.to_owned());
        let result = callback(self);
        self.dispatch_operation = previous;
        result
    }

    pub(crate) fn forward_gateway(
        &mut self,
        request: GatewayRequest,
    ) -> Result<GatewayResponse, GatewayError> {
        let limits = self
            .dispatch_operation
            .as_deref()
            .and_then(|operation| self.negotiated_limits(operation));
        if limits.is_some_and(|limits| {
            request
                .body
                .as_ref()
                .is_some_and(|body| body.to_json().len() > limits.max_request_bytes)
        }) {
            return Err(GatewayError::ResponseTooLarge);
        }
        let response = self.gateway.forward(request)?;
        if limits.is_some_and(|limits| response.body.to_json().len() > limits.max_response_bytes) {
            return Err(GatewayError::ResponseTooLarge);
        }
        Ok(response)
    }

    pub(crate) fn gateway_session_id(&self) -> Option<&str> {
        self.gateway_session_id.as_deref()
    }

    pub(crate) fn mcp_session_id(&self) -> Option<&str> {
        self.mcp_session_id.as_deref()
    }

    pub(crate) fn native_peer_id(&self) -> Option<&str> {
        self.native_peer_id.as_deref()
    }
}

fn catalog_authority_scope(catalog: &ToolCatalog) -> CapabilityScope {
    catalog
        .negotiated_capabilities()
        .map_or(CapabilityScope::ALL, |composition| {
            composition.caller_scope()
        })
}
