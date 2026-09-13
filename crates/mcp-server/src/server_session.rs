// SPDX-License-Identifier: MIT

use crate::catalog::{
    CAPABILITY_DISCOVERY_TOOL, CapabilityLayer, CapabilityScope, NEGOTIATION_STALE_CODE,
    NegotiationError, ToolCatalog,
};
use crate::json::JsonValue;
use crate::protocol::{RpcError, RpcRequest, RpcResponse};

use super::McpServer;

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
    PermissionsChanged,
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
            SessionEvent::PermissionsChanged { caller_scope: _ } => {
                SessionRefreshReason::PermissionsChanged
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
        let before = self.invalidated_snapshots.len();
        let active_snapshots: Vec<String> = self.active_snapshots.iter().cloned().collect();
        self.active_snapshots.clear();
        for snapshot_id in active_snapshots {
            if self.invalidated_snapshots.len() >= MAX_TRACKED_SNAPSHOTS
                && !self.invalidated_snapshots.contains(&snapshot_id)
            {
                let _ = self.invalidated_snapshots.pop_first();
            }
            self.invalidated_snapshots.insert(snapshot_id);
        }
        self.session_epoch = self
            .session_epoch
            .checked_add(1)
            .ok_or_else(|| String::from("MCP session epoch exhausted"))?;
        self.refresh_required = Some(reason.clone());
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
        let changed = self.catalog.to_json() != catalog.to_json();
        self.catalog = catalog;
        self.refresh_required = None;
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
        if self.refresh_required.is_some() || self.request_has_invalidated_snapshot(request) {
            return Some(RpcResponse::failure(
                Some(request.id.clone()),
                RpcError::new(
                    NEGOTIATION_STALE_CODE,
                    "capability catalog or snapshot reference is stale; refresh and reinitialize",
                ),
            ));
        }
        None
    }

    fn request_has_invalidated_snapshot(&self, request: &RpcRequest) -> bool {
        request
            .params
            .as_object()
            .and_then(|object| object.get("arguments"))
            .and_then(JsonValue::as_object)
            .and_then(|arguments| arguments.get("snapshot_ref"))
            .and_then(JsonValue::as_object)
            .and_then(|snapshot| snapshot.get("snapshot_id"))
            .and_then(JsonValue::as_string)
            .is_some_and(|snapshot_id| self.snapshot_is_invalidated(snapshot_id))
    }

    pub(crate) fn remember_snapshot_from_params(&mut self, params: &JsonValue) {
        let Some(arguments) = params
            .as_object()
            .and_then(|object| object.get("arguments"))
            .and_then(JsonValue::as_object)
        else {
            return;
        };
        let Some(snapshot_id) = arguments
            .get("snapshot_ref")
            .and_then(JsonValue::as_object)
            .and_then(|snapshot| snapshot.get("snapshot_id"))
            .and_then(JsonValue::as_string)
        else {
            return;
        };
        if valid_snapshot_identity(snapshot_id) {
            self.track_active_snapshot(snapshot_id.to_owned());
        }
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
        self.track_active_snapshot(snapshot_id);
        Ok(())
    }

    pub fn snapshot_is_invalidated(&self, snapshot_id: &str) -> bool {
        self.invalidated_snapshots.contains(snapshot_id)
    }

    fn track_active_snapshot(&mut self, snapshot_id: String) {
        if self.active_snapshots.contains(&snapshot_id) {
            return;
        }
        if self.active_snapshots.len() >= MAX_TRACKED_SNAPSHOTS {
            let _ = self.active_snapshots.pop_first();
        }
        self.active_snapshots.insert(snapshot_id);
    }
}

fn valid_snapshot_identity(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':' | b'/')
        })
}

fn valid_revision_identity(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

pub(crate) fn tools_changed_notification() -> String {
    JsonValue::object([
        ("jsonrpc".into(), JsonValue::string("2.0")),
        (
            "method".into(),
            JsonValue::string("notifications/tools/list_changed"),
        ),
        ("params".into(), JsonValue::object([])),
    ])
    .to_json()
}
