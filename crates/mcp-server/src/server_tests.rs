// SPDX-License-Identifier: MIT

use crate::mapping::safe_segment;
use crate::{
    CapabilityLayer, CapabilityOwner, CapabilityScope, GatewayAdapter, GatewayError,
    GatewayResponse, JsonValue, SessionEvent, ToolCatalog,
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
    let profiles = [
        ToolCatalog::runtime_map_v1(),
        ToolCatalog::game_information(),
    ];
    let gateway = CapabilityLayer::from_catalogs(CapabilityOwner::Gateway, &profiles)
        .map_err(|error| error.to_string())?;
    let producer = CapabilityLayer::from_catalogs(CapabilityOwner::Producer, &profiles)
        .map_err(|error| error.to_string())?;
    ToolCatalog::compose_profiles(&profiles, gateway, producer, CapabilityScope::ALL)
        .map_err(|error| error.to_string())
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
