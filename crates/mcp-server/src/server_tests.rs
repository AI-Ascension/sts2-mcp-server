// SPDX-License-Identifier: MIT

use crate::mapping::safe_segment;
use crate::{
    CAPABILITY_DISCOVERY_TOOL, CapabilityLayer, CapabilityOwner, CapabilityScope,
    GAME_INFORMATION_CAPABILITIES_TOOL, GatewayAdapter, GatewayError, GatewayResponse, JsonValue,
    OBSERVE_TOOL, SessionEvent, ToolCatalog, ToolLimits,
};

#[derive(Default)]
struct CountingGateway {
    requests: usize,
}

impl GatewayAdapter for CountingGateway {
    fn forward(
        &mut self,
        _request: crate::GatewayRequest,
    ) -> Result<GatewayResponse, GatewayError> {
        self.requests += 1;
        Ok(GatewayResponse {
            status: 200,
            body: JsonValue::Null,
        })
    }
}

fn composed_catalog() -> Result<ToolCatalog, String> {
    composed_catalog_with_scope(CapabilityScope::ALL)
}

fn composed_catalog_with_scope(scope: CapabilityScope) -> Result<ToolCatalog, String> {
    let profiles = [
        ToolCatalog::runtime_map_v1(),
        ToolCatalog::game_information(),
    ];
    let gateway = CapabilityLayer::from_catalogs(CapabilityOwner::Gateway, &profiles)
        .map_err(|error| error.to_string())?;
    let producer = CapabilityLayer::from_catalogs(CapabilityOwner::Producer, &profiles)
        .map_err(|error| error.to_string())?;
    ToolCatalog::compose_profiles(&profiles, gateway, producer, scope)
        .map_err(|error| error.to_string())
}

fn composed_catalog_with_limit(operation: &str, limits: ToolLimits) -> Result<ToolCatalog, String> {
    let mut catalog = composed_catalog()?;
    let composition = catalog
        .composition
        .as_mut()
        .ok_or_else(|| String::from("composition evidence is missing"))?;
    let negotiated = composition
        .operations
        .get_mut(operation)
        .ok_or_else(|| format!("{operation} is not negotiated"))?;
    negotiated.limits = limits;
    Ok(catalog)
}

#[test]
fn accepts_only_path_safe_instance_segments() {
    assert!(safe_segment("instance-1_alpha"));
    assert!(!safe_segment("../instance"));
    assert!(!safe_segment("instance/child"));
}

#[test]
fn lifecycle_event_invalidates_snapshots_and_blocks_forwarding_until_refresh() -> Result<(), String>
{
    let catalog = composed_catalog()?;
    let mut server = super::McpServer::with_catalog(CountingGateway::default(), catalog.clone());
    server.register_snapshot_reference("snapshot-1")?;

    let update = server.apply_session_event(SessionEvent::ProducerRestart)?;
    assert_eq!(update.invalidated_snapshot_count, 1);
    assert!(server.refresh_required());
    assert!(
        server
            .take_notifications()
            .iter()
            .any(|notification| notification.contains("notifications/tools/list_changed"))
    );

    let blocked = server.handle_frame(
        "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{\"name\":\"sts2.observe\",\"arguments\":{}}}",
    );
    assert!(blocked.contains("\"code\":-32009"));
    assert_eq!(server.gateway().requests, 0);

    server.refresh_composed_catalog(catalog)?;
    assert!(!server.refresh_required());
    let stale_snapshot = server.handle_frame(
        "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/call\",\"params\":{\"name\":\"sts2.game_information_detail\",\"arguments\":{\"snapshot_ref\":{\"snapshot_id\":\"snapshot-1\"}}}}",
    );
    assert!(stale_snapshot.contains("\"code\":-32009"));
    assert_eq!(server.gateway().requests, 0);
    Ok(())
}

#[test]
fn permission_revocation_is_enforced_before_refresh_gate_clears() -> Result<(), String> {
    let mut server =
        super::McpServer::with_catalog(CountingGateway::default(), composed_catalog()?);
    let update = server.apply_session_event(SessionEvent::PermissionsChanged {
        caller_scope: CapabilityScope::READ,
    })?;
    assert_eq!(
        update.reason,
        super::SessionRefreshReason::PermissionsChanged {
            caller_scope: CapabilityScope::READ
        }
    );

    assert!(
        server
            .refresh_composed_catalog(composed_catalog()?)
            .is_err()
    );
    assert!(server.refresh_required());

    server.refresh_composed_catalog(composed_catalog_with_scope(CapabilityScope::READ)?)?;
    assert!(!server.refresh_required());
    let listed = server
        .handle_frame("{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\",\"params\":{}}");
    assert!(!listed.contains("\"name\":\"sts2.dispatch_action\""));
    let dispatch = server.handle_frame(
        "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/call\",\"params\":{\"name\":\"sts2.dispatch_action\",\"arguments\":{}}}",
    );
    assert!(dispatch.contains("\"code\":-32601"));
    assert_eq!(server.gateway().requests, 0);
    Ok(())
}

#[test]
fn snapshot_capacity_and_restart_invalidation_fail_closed() -> Result<(), String> {
    let mut server =
        super::McpServer::with_catalog(CountingGateway::default(), composed_catalog()?);
    for index in 0..1024 {
        server.register_snapshot_reference(format!("snapshot-{index}"))?;
    }
    assert!(
        server
            .register_snapshot_reference("snapshot-over-capacity")
            .is_err()
    );
    let blocked = server.handle_frame(
        "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{\"name\":\"sts2.observe\",\"arguments\":{}}}",
    );
    assert!(blocked.contains("\"code\":-32009"));
    assert_eq!(server.gateway().requests, 0);

    let mut server =
        super::McpServer::with_catalog(CountingGateway::default(), composed_catalog()?);
    for index in 0..1024 {
        server.register_snapshot_reference(format!("snapshot-{index}"))?;
    }
    let update = server.apply_session_event(SessionEvent::ProducerRestart)?;
    assert_eq!(update.invalidated_snapshot_count, 1024);
    assert!(server.snapshot_is_invalidated("snapshot-0"));
    server.register_snapshot_reference("snapshot-after-restart")?;
    assert!(
        server
            .apply_session_event(SessionEvent::ProducerRestart)
            .is_err()
    );
    assert!(server.snapshot_is_invalidated("snapshot-0"));
    let blocked = server.handle_frame(
        "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/call\",\"params\":{\"name\":\"sts2.observe\",\"arguments\":{}}}",
    );
    assert!(blocked.contains("\"code\":-32009"));
    Ok(())
}

#[test]
fn composed_limits_bound_requests_and_projected_responses() -> Result<(), String> {
    let request_catalog =
        composed_catalog_with_limit(OBSERVE_TOOL, ToolLimits::bounded(1, 1024, 1024, 1))?;
    let mut request_server =
        super::McpServer::with_catalog(CountingGateway::default(), request_catalog);
    let request = request_server.handle_frame(
        "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{\"name\":\"sts2.observe\",\"arguments\":{}}}",
    );
    assert!(request.contains("negotiated_request_limit_exceeded"));
    assert_eq!(request_server.gateway().requests, 0);

    let response_catalog = composed_catalog_with_limit(
        GAME_INFORMATION_CAPABILITIES_TOOL,
        ToolLimits::bounded(16 * 1024, 1, 1024, 1),
    )?;
    let mut response_server =
        super::McpServer::with_catalog(CountingGateway::default(), response_catalog);
    let response = response_server.handle_frame(
        "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/call\",\"params\":{\"name\":\"sts2.game_information_capabilities\",\"arguments\":{\"instance_id\":\"instance\",\"mcp_session_id\":\"session\",\"lease_id\":\"lease\",\"lease_epoch\":1}}}",
    );
    assert!(response.contains("gateway_response_too_large"));
    assert_eq!(response_server.gateway().requests, 1);

    let content_catalog = composed_catalog_with_limit(
        CAPABILITY_DISCOVERY_TOOL,
        ToolLimits::bounded(16 * 1024, 16 * 1024, 1, 1),
    )?;
    let mut content_server =
        super::McpServer::with_catalog(CountingGateway::default(), content_catalog);
    let content = content_server.handle_frame(
        "{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"tools/call\",\"params\":{\"name\":\"sts2.capabilities\",\"arguments\":{}}}",
    );
    assert!(content.contains("negotiated_content_limit_exceeded"));
    assert_eq!(content_server.gateway().requests, 0);

    let page_catalog = composed_catalog_with_limit(
        CAPABILITY_DISCOVERY_TOOL,
        ToolLimits::bounded(16 * 1024, 16 * 1024, 16 * 1024, 1),
    )?;
    let mut page_server = super::McpServer::with_catalog(CountingGateway::default(), page_catalog);
    let page = page_server.handle_frame(
        "{\"jsonrpc\":\"2.0\",\"id\":4,\"method\":\"tools/call\",\"params\":{\"name\":\"sts2.capabilities\",\"arguments\":{\"page_items\":2}}}",
    );
    assert!(page.contains("negotiated_pagination_limit_exceeded"));
    assert_eq!(page_server.gateway().requests, 0);
    Ok(())
}

#[test]
fn local_capability_discovery_never_forwards() -> Result<(), String> {
    let mut server =
        super::McpServer::with_catalog(CountingGateway::default(), composed_catalog()?);
    let response = server.handle_frame(
        "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{\"name\":\"sts2.capabilities\",\"arguments\":{}}}",
    );
    assert!(response.contains("\"isError\":false"));
    assert!(response.contains("caller_intersection"));
    assert_eq!(server.gateway().requests, 0);
    Ok(())
}

#[test]
fn composed_initialize_and_list_publish_scope_limits_and_epoch() -> Result<(), String> {
    let mut server =
        super::McpServer::with_catalog(CountingGateway::default(), composed_catalog()?);
    let initialize = server.handle_frame(
        "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{\"protocolVersion\":\"2025-06-18\",\"capabilities\":{},\"clientInfo\":{\"name\":\"test-client\",\"version\":\"1\"}}}",
    );
    assert!(initialize.contains("\"listChanged\":true"));
    assert!(initialize.contains("\"session_epoch\":0"));
    assert!(initialize.contains("\"refresh_required\":false"));

    let listed = server
        .handle_frame("{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/list\",\"params\":{}}");
    assert!(listed.contains("\"composition\""));
    assert!(listed.contains("\"max_request_bytes\""));
    assert!(listed.contains("\"_meta\""));
    assert!(listed.contains("\"readOnlyHint\":true"));
    assert!(listed.contains("\"static_reference\""));
    assert!(listed.contains("\"live_details\""));
    assert!(listed.contains("\"gameplay_actions\""));
    assert!(listed.contains("\"maps\""));
    assert!(listed.contains("\"profile_reads\""));
    assert!(listed.contains("\"research_reads\""));
    Ok(())
}

#[test]
fn untracked_snapshot_references_fail_closed_until_registered() -> Result<(), String> {
    let catalog = composed_catalog()?;
    let mut server = super::McpServer::with_catalog(CountingGateway::default(), catalog.clone());
    server.register_snapshot_reference("snapshot-1")?;
    server.apply_session_event(SessionEvent::ProducerRestart)?;
    server.refresh_composed_catalog(catalog)?;

    let untracked = server.handle_frame(
        "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{\"name\":\"sts2.game_information_detail\",\"arguments\":{\"snapshot_ref\":{\"snapshot_id\":\"snapshot-untracked\"}}}}",
    );
    assert!(untracked.contains("\"code\":-32009"), "{untracked}");
    assert_eq!(server.gateway().requests, 0);

    server.register_snapshot_reference("snapshot-2")?;
    let tracked = server.handle_frame(
        "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/call\",\"params\":{\"name\":\"sts2.game_information_detail\",\"arguments\":{\"snapshot_ref\":{\"snapshot_id\":\"snapshot-2\"}}}}",
    );
    assert!(tracked.contains("\"code\":-32602"), "{tracked}");
    assert_eq!(server.gateway().requests, 0);

    // Re-registering a snapshot invalidated by the restart does not revive it.
    server.register_snapshot_reference("snapshot-1")?;
    let revived = server.handle_frame(
        "{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"tools/call\",\"params\":{\"name\":\"sts2.game_information_detail\",\"arguments\":{\"snapshot_ref\":{\"snapshot_id\":\"snapshot-1\"}}}}",
    );
    assert!(revived.contains("\"code\":-32009"), "{revived}");
    assert_eq!(server.gateway().requests, 0);
    Ok(())
}

#[test]
fn pending_revision_survives_unrelated_events_until_satisfied() -> Result<(), String> {
    let catalog = composed_catalog()?;
    let mut server = super::McpServer::with_catalog(CountingGateway::default(), catalog.clone());
    server.apply_session_event(SessionEvent::ToolSetRevisionChanged {
        revision: String::from("never-satisfied-v1-mcp"),
    })?;
    server.apply_session_event(SessionEvent::ContentReload)?;
    assert!(server.refresh_required());

    assert!(server.refresh_composed_catalog(catalog.clone()).is_err());
    assert!(server.refresh_required());

    server.apply_session_event(SessionEvent::ToolSetRevisionChanged {
        revision: String::from("runtime-v3-gameplay-mcp"),
    })?;
    server.apply_session_event(SessionEvent::ContentReload)?;
    server.refresh_composed_catalog(catalog)?;
    assert!(!server.refresh_required());
    Ok(())
}

#[test]
fn rejected_calls_do_not_disable_a_legacy_session() -> Result<(), String> {
    let mut server =
        super::McpServer::with_catalog(CountingGateway::default(), ToolCatalog::default());
    for index in 0..1025 {
        let frame = format!(
            "{{\"jsonrpc\":\"2.0\",\"id\":{index},\"method\":\"tools/call\",\"params\":{{\"name\":\"sts2.unknown\",\"arguments\":{{\"snapshot_id\":\"snapshot-{index}\"}}}}}}"
        );
        let _ = server.handle_frame(&frame);
    }
    let legacy = server.handle_frame(
        "{\"jsonrpc\":\"2.0\",\"id\":9001,\"method\":\"tools/call\",\"params\":{\"name\":\"get_state\",\"arguments\":{\"instance_id\":\"instance-1\",\"mcp_session_id\":\"session-1\"}}}",
    );
    assert!(
        !legacy.contains("\"code\":-32009"),
        "legacy session was disabled by rejected calls: {legacy}"
    );
    Ok(())
}

#[test]
fn pre_dispatch_refusals_do_not_consume_snapshot_tracking() -> Result<(), String> {
    let mut server =
        super::McpServer::with_catalog(CountingGateway::default(), composed_catalog()?);
    for index in 0..1025 {
        let frame = format!(
            "{{\"jsonrpc\":\"2.0\",\"id\":{index},\"method\":\"tools/call\",\"params\":{{\"name\":\"sts2.capabilities\",\"arguments\":{{\"snapshot_id\":\"snapshot-{index}\",\"page_items\":4096}}}}}}"
        );
        let refusal = server.handle_frame(&frame);
        assert!(
            refusal.contains("negotiated_pagination_limit_exceeded"),
            "unexpected refusal: {refusal}"
        );
    }
    assert!(
        server.active_snapshots.is_empty(),
        "refused calls changed snapshot tracking state"
    );
    assert_eq!(server.gateway().requests, 0);

    let ordinary = server.handle_frame(
        "{\"jsonrpc\":\"2.0\",\"id\":9001,\"method\":\"tools/call\",\"params\":{\"name\":\"sts2.observe\",\"arguments\":{}}}",
    );
    assert!(
        !ordinary.contains("\"code\":-32009"),
        "refused calls exhausted snapshot tracking: {ordinary}"
    );
    Ok(())
}
