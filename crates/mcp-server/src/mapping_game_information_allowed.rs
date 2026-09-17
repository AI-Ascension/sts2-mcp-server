// SPDX-License-Identifier: MIT

use super::CallKind;
use crate::catalog::{
    GAME_INFORMATION_AVAILABILITY_TOOL, GAME_INFORMATION_BINDING_TOOL,
    GAME_INFORMATION_CAPABILITIES_TOOL, GAME_INFORMATION_DETAIL_TOOL, GAME_INFORMATION_GET_TOOL,
    GAME_INFORMATION_LIST_TOOL, GAME_INFORMATION_SEARCH_TOOL,
};

pub(super) fn arguments(tool_name: &str) -> &'static [&'static str] {
    match tool_name {
        GAME_INFORMATION_BINDING_TOOL => &[
            "instance_id",
            "mcp_session_id",
            "lease_id",
            "lease_epoch",
            "operation",
            "project_id",
            "run_id",
            "episode_id",
            "agent_id",
            "authority_epoch",
        ],
        GAME_INFORMATION_CAPABILITIES_TOOL => {
            &["instance_id", "mcp_session_id", "lease_id", "lease_epoch"]
        }
        GAME_INFORMATION_LIST_TOOL | GAME_INFORMATION_SEARCH_TOOL => &[
            "instance_id",
            "mcp_session_id",
            "lease_id",
            "lease_epoch",
            "content_manifest_id",
            "locale",
            "visibility_scope",
            "entity_kind",
            "projection",
            "detail_level",
            "fields",
            "page_items",
            "item_bytes",
            "page_bytes",
            "text_bytes",
            "cursor",
            "display_name",
            "namespaced_ids",
            "definition_refs",
            "instance_ids",
        ],
        GAME_INFORMATION_GET_TOOL => &[
            "instance_id",
            "mcp_session_id",
            "lease_id",
            "lease_epoch",
            "content_manifest_id",
            "locale",
            "visibility_scope",
            "entity_kind",
            "projection",
            "detail_level",
            "fields",
            "page_items",
            "item_bytes",
            "page_bytes",
            "text_bytes",
            "cursor",
            "display_name",
            "namespaced_ids",
            "definition_refs",
            "instance_ids",
            "definition_ref",
        ],
        GAME_INFORMATION_DETAIL_TOOL => &[
            "instance_id",
            "mcp_session_id",
            "lease_id",
            "lease_epoch",
            "content_manifest_id",
            "locale",
            "visibility_scope",
            "entity_kind",
            "projection",
            "detail_level",
            "fields",
            "page_items",
            "item_bytes",
            "page_bytes",
            "text_bytes",
            "cursor",
            "display_name",
            "namespaced_ids",
            "definition_refs",
            "instance_ids",
            "definition_ref",
            "instance_ref",
            "snapshot_ref",
            "parent_observation",
        ],
        GAME_INFORMATION_AVAILABILITY_TOOL => &[
            "instance_id",
            "mcp_session_id",
            "lease_id",
            "lease_epoch",
            "content_manifest_id",
            "locale",
            "visibility_scope",
            "entity_kind",
            "projection",
            "detail_level",
            "fields",
            "page_items",
            "item_bytes",
            "page_bytes",
            "text_bytes",
            "cursor",
            "display_name",
            "namespaced_ids",
            "definition_refs",
            "instance_ids",
            "definition_ref",
            "instance_ref",
            "snapshot_ref",
            "parent_observation",
            "mode",
        ],
        _ => &[],
    }
}

pub(super) fn kind_for(name: &str) -> Option<CallKind> {
    match name {
        GAME_INFORMATION_LIST_TOOL => Some(CallKind::List),
        GAME_INFORMATION_SEARCH_TOOL => Some(CallKind::Search),
        GAME_INFORMATION_GET_TOOL => Some(CallKind::Get),
        GAME_INFORMATION_DETAIL_TOOL => Some(CallKind::Detail),
        GAME_INFORMATION_AVAILABILITY_TOOL => Some(CallKind::Availability),
        GAME_INFORMATION_CAPABILITIES_TOOL | GAME_INFORMATION_BINDING_TOOL => None,
        _ => None,
    }
}

pub(crate) fn is_tool(name: &str) -> bool {
    matches!(
        name,
        GAME_INFORMATION_CAPABILITIES_TOOL
            | GAME_INFORMATION_LIST_TOOL
            | GAME_INFORMATION_SEARCH_TOOL
            | GAME_INFORMATION_GET_TOOL
            | GAME_INFORMATION_DETAIL_TOOL
            | GAME_INFORMATION_AVAILABILITY_TOOL
            | GAME_INFORMATION_BINDING_TOOL
    )
}
