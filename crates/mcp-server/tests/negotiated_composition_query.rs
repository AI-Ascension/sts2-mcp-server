// SPDX-License-Identifier: MIT
#![allow(clippy::expect_used, clippy::panic)]

use sts2_mcp_server::{
    CapabilityLayer, CapabilityOwner, CapabilityScope, GAME_INFORMATION_AVAILABILITY_TOOL,
    GAME_INFORMATION_CAPABILITIES_TOOL, GAME_INFORMATION_DETAIL_TOOL, GAME_INFORMATION_GET_TOOL,
    GAME_INFORMATION_LIST_TOOL, GAME_INFORMATION_SEARCH_TOOL, GatewayMethod, JsonValue, McpServer,
    ToolCatalog,
};

#[path = "support/negotiated_composition_query_support.rs"]
mod support;

use support::*;

#[test]
fn composed_initialize_list_and_full_query_sequence() {
    let mut server = McpServer::with_catalog_and_sessions(
        ScriptedGateway::new([
            Reply::Capabilities,
            Reply::EmptyPage,
            Reply::EmptyPage,
            Reply::ListPageOne,
            Reply::ListPageTwo,
            Reply::Detail,
        ]),
        composed_catalog(CapabilityScope::ALL).expect("composed catalog"),
        "gateway-session-1",
        "mcp-session-1",
    );
    server
        .register_snapshot_reference("snapshot-42")
        .expect("tracked snapshot");

    let initialize = wire_value(&server.handle_frame(
        r#"{"jsonrpc":"2.0","id":"init","method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test-client","version":"1"}}}"#,
    ));
    assert!(initialize["result"].is_object(), "{initialize}");

    let listed = wire_value(
        &server.handle_frame(r#"{"jsonrpc":"2.0","id":"list","method":"tools/list","params":{}}"#),
    );
    let names: Vec<_> = listed["result"]["tools"]
        .as_array()
        .expect("tools array")
        .iter()
        .filter_map(|tool| tool["name"].as_str())
        .collect();
    for tool in [
        GAME_INFORMATION_CAPABILITIES_TOOL,
        GAME_INFORMATION_LIST_TOOL,
        GAME_INFORMATION_SEARCH_TOOL,
        GAME_INFORMATION_GET_TOOL,
        GAME_INFORMATION_DETAIL_TOOL,
        GAME_INFORMATION_AVAILABILITY_TOOL,
    ] {
        assert!(
            names.contains(&tool),
            "{tool} missing from composed tools/list"
        );
    }

    let calls = [
        (
            "capabilities",
            GAME_INFORMATION_CAPABILITIES_TOOL,
            context(),
        ),
        (
            "search",
            GAME_INFORMATION_SEARCH_TOOL,
            static_arguments(None),
        ),
        ("get", GAME_INFORMATION_GET_TOOL, {
            let mut value = static_arguments(None);
            if let JsonValue::Object(object) = &mut value {
                object.insert(String::from("definition_ref"), definition_ref());
            }
            value
        }),
        (
            "list-one",
            GAME_INFORMATION_LIST_TOOL,
            static_arguments(None),
        ),
        (
            "list-two",
            GAME_INFORMATION_LIST_TOOL,
            static_arguments(Some("cursor:cards:1")),
        ),
        ("detail", GAME_INFORMATION_DETAIL_TOOL, live_arguments()),
    ];
    for (id, name, arguments) in calls {
        let wire = wire_value(&server.handle_frame(&frame(id, name, arguments.clone())));
        assert_eq!(
            wire["result"]["isError"], false,
            "composed call {name} failed: {wire}"
        );
    }

    let requests = &server.gateway().requests;
    assert_eq!(requests.len(), 6, "forwarded call count");
    assert_eq!(requests[0].method, GatewayMethod::Get);
    assert_eq!(
        requests[0].path,
        "/v1/instances/instance-1/game-information/capabilities"
    );
    assert!(requests[0].body.is_none());
    for (index, request) in requests.iter().enumerate().skip(1) {
        assert_query_route(request, index);
    }

    let mut expected_page_two = golden("static-page-2-request");
    set_correlation(&mut expected_page_two, "list-two");
    assert_eq!(
        requests[4].body.as_ref(),
        Some(&expected_page_two),
        "next-page cursor was not preserved through composition"
    );
    let mut expected_detail = golden("live-detail-request");
    set_correlation(&mut expected_detail, "detail");
    assert_eq!(
        requests[5].body.as_ref(),
        Some(&expected_detail),
        "live detail request drifted through composition"
    );
}

#[test]
fn composed_errors_are_typed_without_forwards() {
    // Unknown argument field: the strict schema refuses before any gateway hand-off.
    let mut server = McpServer::with_catalog(
        FakeGateway::new([]),
        composed_catalog(CapabilityScope::ALL).expect("composed catalog"),
    );
    let malformed = wire_value(&server.handle_frame(&frame(
        "malformed",
        GAME_INFORMATION_CAPABILITIES_TOOL,
        JsonValue::object([
            (String::from("instance_id"), JsonValue::string("instance-1")),
            (
                String::from("mcp_session_id"),
                JsonValue::string("mcp-session-1"),
            ),
            (String::from("lease_id"), JsonValue::string("lease-1")),
            (String::from("lease_epoch"), JsonValue::Number(7)),
            (String::from("unknown_field"), JsonValue::Number(1)),
        ]),
    )));
    assert_eq!(server.gateway().requests.len(), 0);
    assert_typed_failure(&malformed, "unknown field");

    // Missing capability: a catalog without the game-information profile neither
    // advertises nor dispatches the lookup tool.
    let map_only = ToolCatalog::compose_profiles(
        &[ToolCatalog::runtime_map_v1()],
        CapabilityLayer::from_catalogs(CapabilityOwner::Gateway, &[ToolCatalog::runtime_map_v1()])
            .expect("gateway layer"),
        CapabilityLayer::from_catalogs(CapabilityOwner::Producer, &[ToolCatalog::runtime_map_v1()])
            .expect("producer layer"),
        CapabilityScope::ALL,
    )
    .expect("map-only composition");
    let mut missing_server = McpServer::with_catalog(FakeGateway::new([]), map_only);
    let missing = wire_value(&missing_server.handle_frame(&frame(
        "missing",
        GAME_INFORMATION_LIST_TOOL,
        static_arguments(None),
    )));
    assert_eq!(missing_server.gateway().requests.len(), 0);
    assert_eq!(missing["error"]["code"], -32601, "{missing}");

    // Stale reference: the producer refusal is surfaced as a typed bounded error
    // and forwarded exactly once.
    let mut stale = golden("error-stale-cursor");
    let mut stale_server = McpServer::with_catalog(
        FakeGateway::new([successful_response(&mut stale, "stale")]),
        composed_catalog(CapabilityScope::ALL).expect("composed catalog"),
    );
    let stale_wire = wire_value(&stale_server.handle_frame(&frame(
        "stale",
        GAME_INFORMATION_SEARCH_TOOL,
        static_arguments(None),
    )));
    assert_eq!(stale_server.gateway().requests.len(), 1);
    assert_typed_failure(&stale_wire, "stale reference");
}
