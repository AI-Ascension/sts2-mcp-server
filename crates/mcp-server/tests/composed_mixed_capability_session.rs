// SPDX-License-Identifier: MIT
#![allow(clippy::expect_used, clippy::panic)]

//! Composed mixed-capability session acceptance for issue #52.
//!
//! These cases drive the real negotiation (`ToolCatalog::compose_profiles` and
//! `NegotiatedCapabilitySet::negotiate`), the real session lifecycle (`McpServer`),
//! the composed dispatch path, and an in-memory recording gateway. Gameplay
//! observe/legal-actions, map snapshots, and game-information search/detail are
//! exercised in one session against synthetic owned producers and routes. No game
//! host, gateway process, or provider is started, and no host effect is claimed.

use sts2_mcp_server::{
    CAPABILITY_DISCOVERY_TOOL, CapabilityGroup, CapabilityLayer, CapabilityOffer, CapabilityOwner,
    CapabilityScope, DISPATCH_ACTION_TOOL, GAME_INFORMATION_DETAIL_TOOL,
    GAME_INFORMATION_SEARCH_TOOL, JsonValue, LEGAL_ACTIONS_TOOL, MAP_SNAPSHOT_TOOL, McpServer,
    NEGOTIATED_COMPOSITION_REVISION, NegotiatedCapabilitySet, NegotiationError, NegotiationRequest,
    OBSERVE_TOOL, ToolCatalog, ToolLimits,
};

#[path = "composed_mixed_capability_session/fixtures.rs"]
mod fixtures;
#[path = "composed_mixed_capability_session/lifecycle.rs"]
mod lifecycle;

use fixtures::{
    GATEWAY_SESSION, MCP_SESSION, RecordingGateway, SNAPSHOT, assert_consistent_identity,
    composed_catalog, detail_arguments, detail_response, frame, gameplay_lookup_profiles,
    gateway_layer, legal_actions_arguments, legal_actions_response, listed_names, map_arguments,
    map_snapshot_response, observe_arguments, ok, producer_layer, search_response, state_response,
    static_arguments, tools_list_frame, wire,
};

#[test]
fn one_composed_session_serves_gameplay_map_and_content_with_one_identity() {
    let profiles = gameplay_lookup_profiles();
    let catalog = composed_catalog(&profiles, CapabilityScope::ALL).expect("full composition");
    assert_eq!(catalog.revision, NEGOTIATED_COMPOSITION_REVISION);

    let mut server = McpServer::with_catalog_and_sessions(
        RecordingGateway::new([
            ok(state_response("corr-observe")),
            ok(legal_actions_response("corr-legal")),
            ok(map_snapshot_response("corr-map")),
            ok(search_response("corr-search")),
            ok(detail_response("corr-detail", SNAPSHOT)),
        ]),
        catalog,
        GATEWAY_SESSION,
        MCP_SESSION,
    );

    // The composed catalog advertises each of the three capability groups exactly once.
    let listed = wire(&server.handle_frame(&tools_list_frame()));
    let names = listed_names(&listed);
    for tool in [
        OBSERVE_TOOL,
        LEGAL_ACTIONS_TOOL,
        MAP_SNAPSHOT_TOOL,
        GAME_INFORMATION_SEARCH_TOOL,
        GAME_INFORMATION_DETAIL_TOOL,
        CAPABILITY_DISCOVERY_TOOL,
    ] {
        assert_eq!(
            names.iter().filter(|name| *name == tool).count(),
            1,
            "tool {tool} must be advertised exactly once"
        );
    }
    let features = &listed["result"]["composition"]["features"];
    for (group, tool) in [
        ("gameplay_actions", OBSERVE_TOOL),
        ("gameplay_actions", LEGAL_ACTIONS_TOOL),
        ("maps", MAP_SNAPSHOT_TOOL),
        ("static_reference", GAME_INFORMATION_SEARCH_TOOL),
        ("live_details", GAME_INFORMATION_DETAIL_TOOL),
    ] {
        assert!(
            features[group]["available"].to_string().contains(tool),
            "{tool} is not advertised as available in {group}"
        );
    }

    // Local capability discovery reports the negotiated set without a gateway hand-off.
    let caps = wire(&server.handle_frame(&frame(
        "corr-caps",
        CAPABILITY_DISCOVERY_TOOL,
        JsonValue::object([]),
    )));
    assert_eq!(caps["result"]["isError"], false, "{caps}");
    assert_eq!(server.gateway().forwarded(), 0);
    let caps_body = wire(
        caps["result"]["content"][0]["text"]
            .as_str()
            .expect("capability text"),
    );
    assert_eq!(caps_body["revision"], NEGOTIATED_COMPOSITION_REVISION);
    assert_eq!(caps_body["capability_discovery"]["forwards"], false);

    // Ordinary gameplay, maps, and content lookups share one session identity.
    for (id, tool, arguments) in [
        ("corr-observe", OBSERVE_TOOL, observe_arguments()),
        ("corr-legal", LEGAL_ACTIONS_TOOL, legal_actions_arguments()),
        ("corr-map", MAP_SNAPSHOT_TOOL, map_arguments()),
    ] {
        let response = wire(&server.handle_frame(&frame(id, tool, arguments)));
        assert_eq!(response["result"]["isError"], false, "{tool}: {response}");
    }
    // The adapter records the live snapshot an observation is bound to before a
    // composed live lookup may reference it.
    server
        .register_snapshot_reference(SNAPSHOT)
        .expect("session admits the observed snapshot");
    for (id, tool, arguments) in [
        (
            "corr-search",
            GAME_INFORMATION_SEARCH_TOOL,
            static_arguments(),
        ),
        (
            "corr-detail",
            GAME_INFORMATION_DETAIL_TOOL,
            detail_arguments(SNAPSHOT),
        ),
    ] {
        let response = wire(&server.handle_frame(&frame(id, tool, arguments)));
        assert_eq!(response["result"]["isError"], false, "{tool}: {response}");
    }

    assert_eq!(server.gateway().forwarded(), 5);
    assert_consistent_identity(&server.gateway().requests);
    let paths: Vec<&str> = server
        .gateway()
        .requests
        .iter()
        .map(|request| request.path.as_str())
        .collect();
    for route in [
        "/v3/instances/instance-1/state",
        "/v3/instances/instance-1/legal-actions",
        "/v1/instances/instance-1/map-snapshot",
    ] {
        assert!(paths.contains(&route), "missing route {route}: {paths:?}");
    }
    assert_eq!(
        paths
            .iter()
            .filter(|path| **path == "/v1/instances/instance-1/game-information/query")
            .count(),
        2,
        "search and detail must share one bounded query route"
    );
    assert_eq!(server.session_epoch(), 0);
    assert!(!server.refresh_required());
}

#[test]
fn missing_producer_feature_advertises_the_remaining_supported_set() {
    let profiles = gameplay_lookup_profiles();
    // The producer supports maps and gameplay but not the game-information feature.
    let catalog = ToolCatalog::compose_profiles(
        &profiles,
        gateway_layer(&profiles).expect("gateway advertises both profiles"),
        producer_layer(&[profiles[0].clone()]).expect("producer advertises the map profile"),
        CapabilityScope::ALL,
    )
    .expect("partial composition still negotiates");

    let mut server = McpServer::with_catalog_and_sessions(
        RecordingGateway::new([ok(state_response("corr-observe"))]),
        catalog,
        GATEWAY_SESSION,
        MCP_SESSION,
    );
    let listed = wire(&server.handle_frame(&tools_list_frame()));
    let names = listed_names(&listed);
    for tool in [
        OBSERVE_TOOL,
        LEGAL_ACTIONS_TOOL,
        MAP_SNAPSHOT_TOOL,
        CAPABILITY_DISCOVERY_TOOL,
    ] {
        assert!(
            names.contains(&tool.to_owned()),
            "remaining tool {tool} absent from {names:?}"
        );
    }
    for tool in [GAME_INFORMATION_SEARCH_TOOL, GAME_INFORMATION_DETAIL_TOOL] {
        assert!(
            !names.contains(&tool.to_owned()),
            "unsupported tool {tool} must not be advertised"
        );
    }

    let caps = wire(&server.handle_frame(&frame(
        "corr-caps",
        CAPABILITY_DISCOVERY_TOOL,
        JsonValue::object([]),
    )));
    let caps_body = wire(
        caps["result"]["content"][0]["text"]
            .as_str()
            .expect("capability text"),
    );
    assert!(
        caps_body["features"]["maps"]["available"]
            .to_string()
            .contains(MAP_SNAPSHOT_TOOL)
            && caps_body["features"]["gameplay_actions"]["available"]
                .to_string()
                .contains(OBSERVE_TOOL),
        "remaining supported set is not advertised: {caps_body}"
    );
    let unavailable = caps_body["features"]["static_reference"]["unavailable"].to_string();
    assert!(
        unavailable.contains(GAME_INFORMATION_SEARCH_TOOL)
            && unavailable.contains("producer_missing"),
        "missing feature is not advertised with its reason: {caps_body}"
    );
    assert!(
        caps_body["features"]["live_details"]["unavailable"]
            .to_string()
            .contains("producer_missing"),
        "missing live feature is not advertised with its reason: {caps_body}"
    );

    // The remaining supported capability still forwards in the same session.
    let response =
        wire(&server.handle_frame(&frame("corr-observe", OBSERVE_TOOL, observe_arguments())));
    assert_eq!(response["result"]["isError"], false, "{response}");
    assert_eq!(server.gateway().forwarded(), 1);
    assert_consistent_identity(&server.gateway().requests);
}

#[test]
fn conflicting_names_and_versions_fail_negotiation_instead_of_shadowing() {
    // One operation name, two incompatible advertised revisions.
    let conflicts = [ToolCatalog::runtime_v1(), ToolCatalog::runtime_v2()];
    let error = ToolCatalog::compose_profiles(
        &conflicts,
        gateway_layer(&[conflicts[0].clone()]).expect("gateway mirrors the first profile"),
        producer_layer(&[conflicts[0].clone()]).expect("producer mirrors the first profile"),
        CapabilityScope::ALL,
    )
    .expect_err("conflicting operation revisions must not be resolved by shadowing");
    assert!(
        matches!(&error, NegotiationError::RevisionConflict { operation, .. } if operation == "get_state"),
        "unexpected negotiation error: {error}"
    );

    // The published version differs across two profiles that advertise the tool.
    let mut game_information_v999 = ToolCatalog::game_information();
    game_information_v999.revision = String::from("game-information-query-v999-mcp");
    let versions = [ToolCatalog::game_information(), game_information_v999];
    let error = ToolCatalog::compose_profiles(
        &versions,
        gateway_layer(&[versions[0].clone()]).expect("gateway mirrors the first profile"),
        producer_layer(&[versions[0].clone()]).expect("producer mirrors the first profile"),
        CapabilityScope::ALL,
    )
    .expect_err("incompatible published versions must fail negotiation");
    assert!(
        matches!(&error, NegotiationError::RevisionConflict { .. }),
        "unexpected negotiation error: {error}"
    );

    // A producer advertising an incompatible revision for an agreed name fails the
    // negotiated set rather than falling back to another owner's revision.
    let profiles = gameplay_lookup_profiles();
    let producer = CapabilityLayer::new(CapabilityOwner::Producer, "producer-v1")
        .with_offer(CapabilityOffer::supported(
            MAP_SNAPSHOT_TOOL,
            "runtime-map-v2-mcp",
            CapabilityGroup::Maps,
            CapabilityScope::READ,
            ToolLimits::bounded(1024, 4096, 4096, 8),
        ))
        .expect("offer inserts");
    let negotiated = NegotiatedCapabilitySet::negotiate(&NegotiationRequest::new(
        CapabilityLayer::from_catalogs(CapabilityOwner::Mcp, &profiles)
            .expect("mcp mirrors both profiles"),
        gateway_layer(&profiles).expect("gateway mirrors both profiles"),
        producer,
        CapabilityScope::ALL,
    ));
    assert!(
        matches!(&negotiated, Err(NegotiationError::RevisionConflict { operation, .. }) if operation == MAP_SNAPSHOT_TOOL),
        "incompatible producer revision must fail negotiation: {negotiated:?}"
    );
}
