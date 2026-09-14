// SPDX-License-Identifier: MIT

//! Session lifecycle acceptance for the composed mixed-capability session:
//! producer restart and permission revocation must invalidate exactly the
//! capabilities and in-flight snapshot references they affect, and every stale
//! call must be rejected before the gateway boundary (zero forwarded calls).

use super::*;
use crate::fixtures::FRESH_SNAPSHOT;
use sts2_mcp_server::{METHOD_NOT_FOUND, NEGOTIATION_STALE_CODE, SessionEvent};

const STALE: &str = "stale composed call must fail closed before forwarding";

#[test]
fn producer_restart_invalidates_in_flight_snapshots_without_forwarding() {
    let profiles = gameplay_lookup_profiles();
    let mut server = McpServer::with_catalog_and_sessions(
        RecordingGateway::new([
            ok(map_snapshot_response("corr-map-1")),
            ok(map_snapshot_response("corr-map-2")),
            ok(detail_response("corr-detail-2", FRESH_SNAPSHOT)),
        ]),
        composed_catalog(&profiles, CapabilityScope::ALL).expect("full composition"),
        GATEWAY_SESSION,
        MCP_SESSION,
    );
    server
        .register_snapshot_reference(SNAPSHOT)
        .expect("session admits the observed snapshot");
    let first =
        wire(&server.handle_frame(&frame("corr-map-1", MAP_SNAPSHOT_TOOL, map_arguments())));
    assert_eq!(first["result"]["isError"], false, "{first}");
    assert_eq!(server.gateway().forwarded(), 1);
    let epoch_before = server.session_epoch();

    // The producer restarts: the negotiated capabilities and the in-flight
    // snapshot references are invalidated together.
    let update = server
        .apply_session_event(SessionEvent::ProducerRestart)
        .expect("restart applies to the composed session");
    assert!(update.refresh_required);
    assert!(update.invalidated_snapshot_count >= 1);
    assert_eq!(update.session_epoch, epoch_before + 1);
    assert!(server.refresh_required());
    assert!(server.snapshot_is_invalidated(SNAPSHOT));
    assert!(
        server
            .take_notifications()
            .iter()
            .any(|notification| notification.contains("notifications/tools/list_changed")),
        "composed sessions must signal the catalog change"
    );

    // Every composed capability fails closed until the catalog is refreshed,
    // and nothing reaches the gateway.
    for (id, tool, arguments) in [
        ("corr-map-stale", MAP_SNAPSHOT_TOOL, map_arguments()),
        ("corr-observe-stale", OBSERVE_TOOL, observe_arguments()),
        (
            "corr-detail-stale",
            GAME_INFORMATION_DETAIL_TOOL,
            detail_arguments(SNAPSHOT),
        ),
    ] {
        let response = wire(&server.handle_frame(&frame(id, tool, arguments)));
        assert_eq!(
            response["error"]["code"], NEGOTIATION_STALE_CODE,
            "{STALE}: {tool}: {response}"
        );
    }
    assert_eq!(server.gateway().forwarded(), 1, "{STALE}");

    // Refreshing restores the supported capabilities ...
    server
        .refresh_composed_catalog(
            composed_catalog(&profiles, CapabilityScope::ALL).expect("full composition"),
        )
        .expect("refresh after restart");
    assert!(!server.refresh_required());
    let resumed =
        wire(&server.handle_frame(&frame("corr-map-2", MAP_SNAPSHOT_TOOL, map_arguments())));
    assert_eq!(resumed["result"]["isError"], false, "{resumed}");
    assert_eq!(server.gateway().forwarded(), 2);

    // ... but a pre-restart snapshot reference stays invalid, while a freshly
    // registered snapshot in the new epoch is accepted.
    let stale = wire(&server.handle_frame(&frame(
        "corr-detail-stale-2",
        GAME_INFORMATION_DETAIL_TOOL,
        detail_arguments(SNAPSHOT),
    )));
    assert_eq!(
        stale["error"]["code"], NEGOTIATION_STALE_CODE,
        "{STALE}: {stale}"
    );
    assert_eq!(server.gateway().forwarded(), 2, "{STALE}");
    server
        .register_snapshot_reference(FRESH_SNAPSHOT)
        .expect("fresh snapshot registers");
    let fresh = wire(&server.handle_frame(&frame(
        "corr-detail-2",
        GAME_INFORMATION_DETAIL_TOOL,
        detail_arguments(FRESH_SNAPSHOT),
    )));
    assert_eq!(fresh["result"]["isError"], false, "{fresh}");
    assert_eq!(server.gateway().forwarded(), 3);
    assert_consistent_identity(&server.gateway().requests);
}

#[test]
fn permission_revocation_narrows_capabilities_and_blocks_before_forwarding() {
    let profiles = gameplay_lookup_profiles();
    let mut server = McpServer::with_catalog_and_sessions(
        RecordingGateway::new([ok(state_response("corr-observe-1"))]),
        composed_catalog(&profiles, CapabilityScope::ALL).expect("full composition"),
        GATEWAY_SESSION,
        MCP_SESSION,
    );

    // Revocation to read-only scope invalidates the mutate capability and blocks
    // the whole composed session until a compliant refresh.
    let update = server
        .apply_session_event(SessionEvent::PermissionsChanged {
            caller_scope: CapabilityScope::READ,
        })
        .expect("revocation applies");
    assert!(update.refresh_required);
    let blocked = wire(&server.handle_frame(&frame(
        "corr-dispatch",
        DISPATCH_ACTION_TOOL,
        JsonValue::object([]),
    )));
    assert_eq!(
        blocked["error"]["code"], NEGOTIATION_STALE_CODE,
        "{blocked}"
    );
    assert_eq!(server.gateway().forwarded(), 0, "{STALE}");

    // A refresh that still claims the revoked scope is refused.
    let overreach = server.refresh_composed_catalog(
        composed_catalog(&profiles, CapabilityScope::ALL).expect("full composition"),
    );
    assert!(overreach.is_err(), "revoked scope must not be restored");
    assert!(server.refresh_required());

    // The narrowed catalog keeps every read capability and drops gameplay mutation.
    let narrowed = composed_catalog(&profiles, CapabilityScope::READ).expect("read composition");
    assert!(
        narrowed
            .tools()
            .iter()
            .all(|tool| tool.name != DISPATCH_ACTION_TOOL)
    );
    server
        .refresh_composed_catalog(narrowed)
        .expect("refresh within the revoked scope");
    assert!(!server.refresh_required());

    let listed = wire(&server.handle_frame(&tools_list_frame()));
    let names = listed_names(&listed);
    for tool in [
        OBSERVE_TOOL,
        MAP_SNAPSHOT_TOOL,
        GAME_INFORMATION_SEARCH_TOOL,
    ] {
        assert!(
            names.contains(&tool.to_owned()),
            "read capability {tool} lost"
        );
    }
    assert!(
        !names.contains(&DISPATCH_ACTION_TOOL.to_owned()),
        "revoked mutate capability is still advertised"
    );

    // The revoked capability is rejected at the descriptor boundary, not forwarded.
    let denied = wire(&server.handle_frame(&frame(
        "corr-dispatch-2",
        DISPATCH_ACTION_TOOL,
        JsonValue::object([]),
    )));
    assert_eq!(denied["error"]["code"], METHOD_NOT_FOUND, "{denied}");
    assert_eq!(server.gateway().forwarded(), 0, "{STALE}");

    let allowed =
        wire(&server.handle_frame(&frame("corr-observe-1", OBSERVE_TOOL, observe_arguments())));
    assert_eq!(allowed["result"]["isError"], false, "{allowed}");
    assert_eq!(server.gateway().forwarded(), 1);
    assert_consistent_identity(&server.gateway().requests);
}
