// SPDX-License-Identifier: MIT
// Nested test module shared by the game-information integration seam.
#![allow(clippy::expect_used, clippy::panic)]

use super::*;

#[test]
fn malformed_inputs_and_responses_fail_closed_without_mutation() {
    let mut server = McpServer::with_catalog(
        FakeGateway::new([Err(GatewayError::Unavailable)]),
        ToolCatalog::game_information_query_v1(),
    );
    let mut unknown = static_arguments(None);
    if let JsonValue::Object(object) = &mut unknown {
        object.insert(String::from("unexpected"), JsonValue::Bool(true));
    }
    let output = server.handle_frame(&frame("unknown", GAME_INFORMATION_LIST_TOOL, unknown));
    assert!(output.contains("\"code\":-32602"));
    assert!(server.gateway().requests.is_empty());

    let mut foreign = live_arguments();
    if let JsonValue::Object(object) = &mut foreign
        && let Some(JsonValue::Object(instance)) = object.get_mut("instance_ref")
    {
        instance.insert(
            String::from("instance_id"),
            JsonValue::string("other-instance"),
        );
    }
    let output = server.handle_frame(&frame("foreign", GAME_INFORMATION_DETAIL_TOOL, foreign));
    assert!(output.contains("\"code\":-32602"));
    assert!(server.gateway().requests.is_empty());

    let mut oversized = static_arguments(None);
    if let JsonValue::Object(object) = &mut oversized {
        object.insert(String::from("cursor"), JsonValue::string("x".repeat(513)));
    }
    let output = server.handle_frame(&frame("oversized", GAME_INFORMATION_LIST_TOOL, oversized));
    assert!(output.contains("\"code\":-32602"));
    assert!(server.gateway().requests.is_empty());
}

#[test]
fn unsupported_fields_and_live_fences_fail_before_gateway() {
    let mut server = McpServer::with_catalog(
        FakeGateway::new([]),
        ToolCatalog::game_information_query_v1(),
    );
    let mut duplicate_fields = static_arguments(None);
    if let JsonValue::Object(object) = &mut duplicate_fields {
        object.insert(
            String::from("fields"),
            JsonValue::Array(vec![
                JsonValue::string("display_name"),
                JsonValue::string("display_name"),
            ]),
        );
    }
    let output = server.handle_frame(&frame(
        "duplicate-fields",
        GAME_INFORMATION_LIST_TOOL,
        duplicate_fields,
    ));
    assert!(output.contains("\"code\":-32602"));

    let mut live_without_parent = live_arguments();
    if let JsonValue::Object(object) = &mut live_without_parent {
        object.remove("parent_observation");
    }
    let output = server.handle_frame(&frame(
        "missing-parent",
        GAME_INFORMATION_DETAIL_TOOL,
        live_without_parent,
    ));
    assert!(output.contains("\"code\":-32602"));
    assert!(server.gateway().requests.is_empty());
}

#[test]
fn malformed_page_accounting_and_state_are_not_projected() {
    fn bad_accounting(body: &mut JsonValue) {
        if let JsonValue::Object(root) = body
            && let Some(JsonValue::Object(result)) = root.get_mut("result")
            && let Some(JsonValue::Object(page)) = result.get_mut("page")
            && let Some(JsonValue::Object(accounting)) = page.get_mut("accounting")
        {
            accounting.insert(String::from("payload_bytes"), JsonValue::Number(0));
        }
    }
    fn bad_page_state(body: &mut JsonValue) {
        if let JsonValue::Object(root) = body
            && let Some(JsonValue::Object(result)) = root.get_mut("result")
            && let Some(JsonValue::Object(page)) = result.get_mut("page")
        {
            page.insert(String::from("final_page"), JsonValue::Bool(true));
        }
    }
    for (id, mutate) in [
        ("bad-accounting", bad_accounting as fn(&mut JsonValue)),
        ("bad-page-state", bad_page_state as fn(&mut JsonValue)),
    ] {
        let mut response = golden("static-page-1-response");
        set_correlation(&mut response, id);
        mutate(&mut response);
        let mut server = McpServer::with_catalog(
            FakeGateway::new([successful_response(&mut response, id)]),
            ToolCatalog::game_information_query_v1(),
        );
        let output = server.handle_frame(&frame(
            id,
            GAME_INFORMATION_LIST_TOOL,
            static_arguments(None),
        ));
        let wire = wire_value(&output);
        assert_eq!(wire["result"]["isError"], true, "{output}");
        assert_eq!(server.gateway().requests.len(), 1);
    }
}

#[test]
fn copied_protocol_artifact_and_checksum_inventory_are_verified() {
    assert_eq!(verify_game_information_artifact(), Ok(()));
}
