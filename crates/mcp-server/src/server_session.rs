// SPDX-License-Identifier: MIT

use crate::catalog::{
    CAPABILITY_DISCOVERY_TOOL, CapabilityLayer, CapabilityScope, NEGOTIATION_STALE_CODE,
    NegotiationError, ToolCatalog,
};
use crate::json::JsonValue;
use crate::protocol::{RpcError, RpcRequest, RpcResponse};

use super::McpServer;
#[path = "server_session_support.rs"]
mod support;
use support::{
    catalog_revision_matches, collect_snapshot_ids, collect_snapshot_ids_from_response,
    tools_changed_notification, valid_revision_identity, valid_snapshot_identity,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SessionEvent {
    ProducerRestart,
    ContentReload,
    PermissionsChanged { caller_scope: CapabilityScope },
    ToolSetRevisionChanged { revision: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SessionRefreshReason {
    ProducerRestart,
    ContentReload,
    PermissionsChanged { caller_scope: CapabilityScope },
    ToolSetRevisionChanged(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionUpdate {
    pub session_epoch: u64,
    pub reason: SessionRefreshReason,
    pub invalidated_snapshot_count: usize,
    pub refresh_required: bool,
}

const MAX_TRACKED_SNAPSHOTS: usize = 1024;

impl<G> McpServer<G> {
    pub fn apply_session_event(&mut self, event: SessionEvent) -> Result<SessionUpdate, String> {
        let reason = match event {
            SessionEvent::ProducerRestart => SessionRefreshReason::ProducerRestart,
            SessionEvent::ContentReload => SessionRefreshReason::ContentReload,
            SessionEvent::PermissionsChanged { caller_scope } => {
                SessionRefreshReason::PermissionsChanged { caller_scope }
            }
            SessionEvent::ToolSetRevisionChanged { revision } => {
                if !valid_revision_identity(&revision) {
                    return Err(String::from(
                        "tool-set revision is empty, unsafe, or oversized",
                    ));
                }
                SessionRefreshReason::ToolSetRevisionChanged(revision)
            }
        };
        let pending_scope = match &reason {
            SessionRefreshReason::PermissionsChanged { caller_scope } => Some(*caller_scope),
            _ => None,
        };
        let pending_revision = match &reason {
            SessionRefreshReason::ToolSetRevisionChanged(revision) => Some(revision.clone()),
            _ => None,
        };
        if self.snapshot_tracking_exhausted {
            return Err(String::from(
                "snapshot tracking capacity exhausted; session event is refused",
            ));
        }
        let before = self.invalidated_snapshots.len();
        let active_snapshots: Vec<String> = self
            .active_snapshots
            .difference(&self.invalidated_snapshots)
            .cloned()
            .collect();
        let new_snapshot_count = active_snapshots.len();
        if self
            .invalidated_snapshots
            .len()
            .checked_add(new_snapshot_count)
            .is_none_or(|count| count > MAX_TRACKED_SNAPSHOTS)
        {
            self.snapshot_tracking_exhausted = true;
            return Err(String::from(
                "snapshot tracking capacity exhausted; refusing to evict invalidated references",
            ));
        }
        self.active_snapshots.clear();
        for snapshot_id in active_snapshots {
            self.invalidated_snapshots.insert(snapshot_id);
        }
        self.session_epoch = self
            .session_epoch
            .checked_add(1)
            .ok_or_else(|| String::from("MCP session epoch exhausted"))?;
        self.refresh_required = Some(reason.clone());
        // Retain an outstanding revision constraint until a compliant refresh.
        if pending_revision.is_some() {
            self.pending_revision = pending_revision;
        }
        if let Some(scope) = pending_scope {
            self.authority_scope = scope;
        }
        if self.catalog.is_negotiated_composition() {
            self.notifications.push_back(tools_changed_notification());
        }
        Ok(SessionUpdate {
            session_epoch: self.session_epoch,
            reason,
            invalidated_snapshot_count: self.invalidated_snapshots.len() - before,
            refresh_required: true,
        })
    }

    pub fn refresh_composed_catalog(&mut self, catalog: ToolCatalog) -> Result<(), String> {
        if !catalog.is_negotiated_composition() {
            return Err(String::from(
                "session refresh requires a negotiated composition catalog",
            ));
        }
        if self.refresh_required.is_none() {
            return Err(String::from("session refresh was not requested"));
        }
        self.validate_refresh_catalog(&catalog)?;
        let changed = self.catalog.to_json() != catalog.to_json();
        let authority_scope = catalog
            .negotiated_capabilities()
            .map_or(CapabilityScope::ALL, |composition| {
                composition.caller_scope()
            });
        self.catalog = catalog;
        self.refresh_required = None;
        self.pending_revision = None;
        self.authority_scope = authority_scope;
        self.session_epoch = self
            .session_epoch
            .checked_add(1)
            .ok_or_else(|| String::from("MCP session epoch exhausted"))?;
        if changed {
            self.notifications.push_back(tools_changed_notification());
        }
        Ok(())
    }

    pub fn renegotiate_composed_catalog(
        &mut self,
        profiles: &[ToolCatalog],
        gateway: CapabilityLayer,
        producer: CapabilityLayer,
        caller_scope: CapabilityScope,
    ) -> Result<(), NegotiationError> {
        let catalog = ToolCatalog::compose_profiles(profiles, gateway, producer, caller_scope)?;
        self.refresh_composed_catalog(catalog)
            .map_err(|_| NegotiationError::NoCompatibleOperations)
    }

    pub fn session_epoch(&self) -> u64 {
        self.session_epoch
    }

    pub fn refresh_required(&self) -> bool {
        self.refresh_required.is_some()
    }

    pub fn take_notifications(&mut self) -> Vec<String> {
        self.notifications.drain(..).collect()
    }

    pub(crate) fn reject_stale_call(&self, request: &RpcRequest) -> Option<RpcResponse> {
        if request.method != "tools/call" {
            return None;
        }
        let local_discovery = request
            .params
            .as_object()
            .and_then(|params| params.get("name"))
            .and_then(JsonValue::as_string)
            == Some(CAPABILITY_DISCOVERY_TOOL);
        if local_discovery {
            return None;
        }
        if self.snapshot_tracking_exhausted
            || self.refresh_required.is_some()
            || self.request_has_stale_snapshot(request)
        {
            return Some(RpcResponse::failure(
                Some(request.id.clone()),
                RpcError::new(
                    NEGOTIATION_STALE_CODE,
                    "capability catalog or snapshot reference is stale; refresh the catalog",
                ),
            ));
        }
        None
    }

    /// Admits a snapshot reference only when this session can still use it:
    /// composition sessions require registration, standalone catalogs do not.
    fn request_has_stale_snapshot(&self, request: &RpcRequest) -> bool {
        let mut references = Vec::new();
        collect_snapshot_ids(&request.params, &mut references).is_ok()
            && references.iter().any(|snapshot_id| {
                self.snapshot_is_invalidated(snapshot_id)
                    || (self.catalog.is_negotiated_composition()
                        && !self.active_snapshots.contains(snapshot_id))
            })
    }

    /// Validates request snapshot identities without mutating tracking state.
    pub(crate) fn validate_snapshot_params(params: &JsonValue) -> Result<(), &'static str> {
        let mut references = Vec::new();
        collect_snapshot_ids(params, &mut references)
    }

    pub(crate) fn remember_snapshot_from_params(
        &mut self,
        params: &JsonValue,
    ) -> Result<(), &'static str> {
        let mut references = Vec::new();
        collect_snapshot_ids(params, &mut references)?;
        self.track_active_snapshots(references)
    }

    pub(crate) fn remember_snapshot_from_response(
        &mut self,
        response: &RpcResponse,
    ) -> Result<(), &'static str> {
        let Some(result) = response.result() else {
            return Ok(());
        };
        let mut references = Vec::new();
        collect_snapshot_ids_from_response(result, &mut references)?;
        self.track_active_snapshots(references)
    }

    pub fn register_snapshot_reference(
        &mut self,
        snapshot_id: impl Into<String>,
    ) -> Result<(), String> {
        let snapshot_id = snapshot_id.into();
        if !valid_snapshot_identity(&snapshot_id) {
            return Err(String::from(
                "snapshot identity is empty, unsafe, or oversized",
            ));
        }
        if self.snapshot_tracking_exhausted {
            return Err(String::from("snapshot tracking capacity exhausted"));
        }
        self.track_active_snapshot(snapshot_id)
    }

    pub fn snapshot_is_invalidated(&self, snapshot_id: &str) -> bool {
        self.invalidated_snapshots.contains(snapshot_id)
    }

    fn track_active_snapshot(&mut self, snapshot_id: String) -> Result<(), String> {
        if self.active_snapshots.contains(&snapshot_id) {
            return Ok(());
        }
        if self.active_snapshots.len() >= MAX_TRACKED_SNAPSHOTS {
            self.snapshot_tracking_exhausted = true;
            return Err(String::from(
                "snapshot tracking capacity exhausted; refusing to evict active references",
            ));
        }
        self.active_snapshots.insert(snapshot_id);
        Ok(())
    }

    fn track_active_snapshots(&mut self, snapshot_ids: Vec<String>) -> Result<(), &'static str> {
        let new_count = snapshot_ids
            .iter()
            .filter(|snapshot_id| !self.active_snapshots.contains(*snapshot_id))
            .count();
        if self
            .active_snapshots
            .len()
            .checked_add(new_count)
            .is_none_or(|count| count > MAX_TRACKED_SNAPSHOTS)
        {
            self.snapshot_tracking_exhausted = true;
            return Err("snapshot tracking capacity exhausted");
        }
        self.active_snapshots.extend(snapshot_ids);
        Ok(())
    }

    fn validate_refresh_catalog(&self, catalog: &ToolCatalog) -> Result<(), String> {
        let Some(composition) = catalog.negotiated_capabilities() else {
            return Err(String::from(
                "refreshed catalog has no negotiation evidence",
            ));
        };
        if composition.revision != crate::catalog::NEGOTIATED_COMPOSITION_REVISION {
            return Err(String::from(
                "refreshed catalog has an incompatible negotiation revision",
            ));
        }
        if !self.authority_scope.contains(composition.caller_scope())
            || composition.operations().any(|operation| {
                !self.authority_scope.contains(operation.required_scope)
                    || !self.authority_scope.contains(operation.effective_scope)
            })
        {
            return Err(String::from(
                "refreshed catalog exceeds the current caller scope",
            ));
        }
        if let Some(revision) = &self.pending_revision
            && !catalog_revision_matches(composition, revision)
        {
            return Err(String::from(
                "refreshed catalog does not satisfy the changed tool-set revision",
            ));
        }
        Ok(())
    }
}
