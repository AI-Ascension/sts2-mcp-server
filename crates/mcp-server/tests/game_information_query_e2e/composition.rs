// SPDX-License-Identifier: MIT
//! The same lookup sequence through the negotiated MCP capability composition:
//! the runtime-map and game-information profiles composed against explicit
//! gateway and producer capability layers, with local capability discovery and
//! the composition session's snapshot-admission fence.

use serde_json::{Value, json};
use sts2_mcp_server::{
    CAPABILITY_DISCOVERY_TOOL, CapabilityLayer, CapabilityOwner, CapabilityScope,
    GAME_INFORMATION_DETAIL_TOOL, GAME_INFORMATION_GET_TOOL, GAME_INFORMATION_SEARCH_TOOL,
    MAP_SNAPSHOT_TOOL, McpServer, NEGOTIATED_COMPOSITION_REVISION, NEGOTIATION_STALE_CODE,
    ToolCatalog,
};

use super::producer::{
    GATEWAY_SESSION_ID, LIVE_ENTITY_ID, MCP_SESSION_ID, QUERY_PATH, SNAPSHOT_GENERATION,
    SNAPSHOT_ID, SyntheticProducer,
};
use super::{
    attempt, call, get_arguments, live_arguments, request, search_arguments, tool_envelope,
};

/// Composes the runtime-map and game-information profiles with a producer
/// capability layer that advertises only the game-information operations, so
/// unsupported map features stay explicitly unavailable.
fn composed_server(producer: SyntheticProducer) -> Result<McpServer<SyntheticProducer>, String> {
    let profiles = [
        ToolCatalog::runtime_map_v1(),
        ToolCatalog::game_information_query_v1(),
    ];
    let gateway = CapabilityLayer::from_catalogs(CapabilityOwner::Gateway, &profiles)
        .map_err(|error| error.to_string())?;
    let producer_layer = CapabilityLayer::from_catalog_for(CapabilityOwner::Producer, &profiles[1])
        .map_err(|error| error.to_string())?;
    let catalog =
        ToolCatalog::compose_gameplay_lookup(gateway, producer_layer, CapabilityScope::READ)
            .map_err(|error| error.to_string())?;
    Ok(McpServer::with_catalog_and_sessions(
        producer,
        catalog,
        GATEWAY_SESSION_ID,
        MCP_SESSION_ID,
    ))
}

#[test]
fn negotiated_composition_reaches_discovery_pagination_and_get() -> Result<(), String> {
    let mut server = composed_server(SyntheticProducer::contract())?;
    let wire = request(
        &mut server,
        "init",
        "initialize",
        json!({
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {"name": "issue-51-composition", "version": "1.0.0"},
        }),
    );
    assert_eq!(
        wire["result"]["composition"]["revision"],
        NEGOTIATED_COMPOSITION_REVISION
    );
    assert_eq!(wire["result"]["refresh_required"], false);
    assert_eq!(wire["result"]["session_epoch"], 0);

    let wire = request(&mut server, "list", "tools/list", json!({}));
    assert_eq!(wire["result"]["revision"], NEGOTIATED_COMPOSITION_REVISION);
    let names: Vec<&str> = wire["result"]["tools"]
        .as_array()
        .expect("tools array")
        .iter()
        .filter_map(|tool| tool["name"].as_str())
        .collect();
    for name in [
        CAPABILITY_DISCOVERY_TOOL,
        GAME_INFORMATION_SEARCH_TOOL,
        GAME_INFORMATION_GET_TOOL,
        GAME_INFORMATION_DETAIL_TOOL,
    ] {
        assert!(names.contains(&name), "{name} is not composed: {names:?}");
    }
    assert!(
        !names.contains(&MAP_SNAPSHOT_TOOL),
        "a producer-unsupported feature must not be advertised: {names:?}"
    );

    let wire = call(
        &mut server,
        "discovery",
        CAPABILITY_DISCOVERY_TOOL,
        json!({}),
    );
    let discovery = tool_envelope(&wire);
    assert_eq!(discovery["revision"], NEGOTIATED_COMPOSITION_REVISION);
    assert_eq!(discovery["capability_discovery"]["forwards"], false);
    assert_eq!(discovery["capability_discovery"]["read_only"], true);
    assert_eq!(discovery["refresh_required"], false);
    assert!(
        discovery["features"]["static_reference"]["available"]
            .as_array()
            .is_some_and(|available| available.contains(&json!(GAME_INFORMATION_SEARCH_TOOL))),
        "{discovery}"
    );
    assert!(
        discovery["features"]["live_details"]["available"]
            .as_array()
            .is_some_and(|available| available.contains(&json!(GAME_INFORMATION_DETAIL_TOOL))),
        "{discovery}"
    );
    assert!(
        discovery["features"]["maps"]["unavailable"]
            .as_array()
            .is_some_and(|unavailable| unavailable.contains(&json!({
                "operation": MAP_SNAPSHOT_TOOL,
                "reason": "producer_missing",
            }))),
        "{discovery}"
    );
    assert!(server.gateway().records().is_empty(), "discovery is local");

    let wire = call(
        &mut server,
        "search-1",
        GAME_INFORMATION_SEARCH_TOOL,
        search_arguments(None, "summary"),
    );
    let envelope = tool_envelope(&wire);
    let cursor = envelope["result"]["page"]["next_cursor"]
        .as_str()
        .expect("continuation cursor")
        .to_owned();
    let wire = call(
        &mut server,
        "search-2",
        GAME_INFORMATION_SEARCH_TOOL,
        search_arguments(Some(&cursor), "summary"),
    );
    let envelope = tool_envelope(&wire);
    assert_eq!(envelope["result"]["page"]["final_page"], true);
    assert_eq!(
        envelope["result"]["page"]["items"][0]["definition_ref"]["namespaced_id"],
        "synthetic:strike"
    );

    let wire = call(
        &mut server,
        "get",
        GAME_INFORMATION_GET_TOOL,
        get_arguments("synthetic:bash"),
    );
    let envelope = tool_envelope(&wire);
    assert_eq!(
        envelope["result"]["page"]["items"][0]["definition_ref"]["namespaced_id"],
        "synthetic:bash"
    );

    for record in server.gateway().records() {
        assert_eq!(record.path, QUERY_PATH);
    }
    assert!(
        server.gateway().violations().is_empty(),
        "{:?}",
        server.gateway().violations()
    );
    Ok(())
}

#[test]
fn composition_live_detail_requires_an_admitted_snapshot() -> Result<(), String> {
    let mut server = composed_server(SyntheticProducer::contract())?;
    let wire = attempt(
        &mut server,
        "unregistered",
        GAME_INFORMATION_DETAIL_TOOL,
        live_arguments("card", LIVE_ENTITY_ID, "synthetic:strike"),
    );
    assert_eq!(wire["error"]["code"], NEGOTIATION_STALE_CODE, "{wire}");
    assert!(
        server.gateway().records().is_empty(),
        "a stale snapshot fence must not reach the producer"
    );

    server
        .register_snapshot_reference(SNAPSHOT_ID)
        .map_err(|error| error.to_string())?;
    let wire = call(
        &mut server,
        "admitted",
        GAME_INFORMATION_DETAIL_TOOL,
        live_arguments("card", LIVE_ENTITY_ID, "synthetic:strike"),
    );
    let envelope = tool_envelope(&wire);
    assert_eq!(envelope["result"]["result_generation"], SNAPSHOT_GENERATION);
    assert_eq!(
        envelope["result"]["page"]["items"][0]["instance_ref"]["entity_id"],
        LIVE_ENTITY_ID
    );

    let wire = request(
        &mut server,
        "unsupported-feature",
        "tools/call",
        json!({"name": MAP_SNAPSHOT_TOOL, "arguments": Value::Null}),
    );
    assert_eq!(wire["error"]["code"], -32601, "{wire}");
    assert_eq!(server.gateway().records().len(), 1);
    assert!(server.gateway().violations().is_empty());
    Ok(())
}
