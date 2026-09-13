// SPDX-License-Identifier: MIT
// Regression coverage for the game-information MCP boundary.
#![allow(clippy::expect_used, clippy::panic)]

use super::*;

#[test]
fn query_schemas_define_every_required_context_property() {
    let mut server = McpServer::with_catalog(
        FakeGateway::new([]),
        ToolCatalog::game_information_query_v1(),
    );
    let wire = wire_value(
        &server.handle_frame(r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#),
    );
    let tools = wire["result"]["tools"].as_array().expect("tools array");
    for name in [
        GAME_INFORMATION_LIST_TOOL,
        GAME_INFORMATION_SEARCH_TOOL,
        GAME_INFORMATION_GET_TOOL,
        GAME_INFORMATION_DETAIL_TOOL,
        GAME_INFORMATION_AVAILABILITY_TOOL,
    ] {
        let tool = tools
            .iter()
            .find(|tool| tool["name"] == name)
            .expect("game-information tool descriptor");
        let properties = tool["inputSchema"]["properties"]
            .as_object()
            .expect("closed query properties");
        for key in ["instance_id", "mcp_session_id", "lease_id", "lease_epoch"] {
            assert!(properties.contains_key(key), "{name} omits {key}");
        }
    }
}

#[test]
fn emitted_input_schemas_accept_valid_arguments_for_every_tool() {
    let mut server = McpServer::with_catalog(
        FakeGateway::new([]),
        ToolCatalog::game_information_query_v1(),
    );
    let wire = wire_value(
        &server.handle_frame(r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#),
    );
    let tools = wire["result"]["tools"].as_array().expect("tools array");
    let mut get = static_arguments(None);
    if let JsonValue::Object(object) = &mut get {
        object.insert(String::from("definition_ref"), definition_ref());
    }
    let mut availability = static_arguments(None);
    if let JsonValue::Object(object) = &mut availability {
        object.insert(String::from("mode"), JsonValue::string("static"));
    }
    for (name, arguments) in [
        (GAME_INFORMATION_CAPABILITIES_TOOL, context()),
        (GAME_INFORMATION_LIST_TOOL, static_arguments(None)),
        (GAME_INFORMATION_SEARCH_TOOL, static_arguments(None)),
        (GAME_INFORMATION_GET_TOOL, get),
        (GAME_INFORMATION_DETAIL_TOOL, live_arguments()),
        (GAME_INFORMATION_AVAILABILITY_TOOL, availability),
    ] {
        let tool = tools
            .iter()
            .find(|tool| tool["name"] == name)
            .expect("game-information tool descriptor");
        let validator = jsonschema::draft202012::options()
            .build(&tool["inputSchema"])
            .expect("input schema compiles");
        let value: Value = serde_json::from_str(&arguments.to_json()).expect("arguments are JSON");
        assert!(
            validator.is_valid(&value),
            "{name} input schema rejects valid arguments: {value}"
        );
    }
}

#[test]
fn unicode_text_is_accepted_but_c0_and_c1_controls_are_rejected() {
    let mut server = McpServer::with_catalog(
        FakeGateway::new([]),
        ToolCatalog::game_information_query_v1(),
    );
    let mut unicode = static_arguments(None);
    if let JsonValue::Object(object) = &mut unicode {
        object.insert(String::from("display_name"), JsonValue::string("é界😀"));
    }
    let output = server.handle_frame(&frame(
        "unicode-request",
        GAME_INFORMATION_SEARCH_TOOL,
        unicode,
    ));
    assert!(!output.contains("\"code\":-32602"), "{output}");
    assert_eq!(server.gateway().requests.len(), 1);
    let request_body = match server.gateway().requests[0].body.as_ref() {
        Some(JsonValue::Object(body)) => body,
        _ => panic!("query request body"),
    };
    let query = match request_body.get("query") {
        Some(JsonValue::Object(query)) => query,
        _ => panic!("query envelope"),
    };
    let filters = match query.get("filters") {
        Some(JsonValue::Object(filters)) => filters,
        _ => panic!("query filters"),
    };
    assert_eq!(
        filters.get("display_name"),
        Some(&JsonValue::string("é界😀"))
    );

    let mut control = static_arguments(None);
    if let JsonValue::Object(object) = &mut control {
        object.insert(String::from("display_name"), JsonValue::string("\u{0090}"));
    }
    let output = server.handle_frame(&frame("c1-request", GAME_INFORMATION_SEARCH_TOOL, control));
    assert!(output.contains("\"code\":-32602"), "{output}");
    assert_eq!(server.gateway().requests.len(), 1);
}

#[test]
fn unicode_error_reasons_are_projected_and_control_reasons_fail_closed() {
    let mut valid = golden("error-stale-cursor");
    set_correlation(&mut valid, "unicode-error");
    if let JsonValue::Object(root) = &mut valid
        && let Some(JsonValue::Object(error)) = root.get_mut("error")
    {
        error.insert(String::from("reason"), JsonValue::string("界"));
    }
    let mut server = McpServer::with_catalog(
        FakeGateway::new([successful_response(&mut valid, "unicode-error")]),
        ToolCatalog::game_information_query_v1(),
    );
    let output = server.handle_frame(&frame(
        "unicode-error",
        GAME_INFORMATION_SEARCH_TOOL,
        static_arguments(Some("cursor:cards:1")),
    ));
    assert!(output.contains("界"), "{output}");
    assert_eq!(wire_value(&output)["result"]["isError"], true);

    let mut invalid = golden("error-stale-cursor");
    set_correlation(&mut invalid, "control-error");
    if let JsonValue::Object(root) = &mut invalid
        && let Some(JsonValue::Object(error)) = root.get_mut("error")
    {
        error.insert(String::from("reason"), JsonValue::string("\u{0090}"));
    }
    let mut server = McpServer::with_catalog(
        FakeGateway::new([successful_response(&mut invalid, "control-error")]),
        ToolCatalog::game_information_query_v1(),
    );
    let output = server.handle_frame(&frame(
        "control-error",
        GAME_INFORMATION_SEARCH_TOOL,
        static_arguments(Some("cursor:cards:1")),
    ));
    assert!(output.contains("error reason is invalid"), "{output}");
    assert_eq!(wire_value(&output)["result"]["isError"], true);
}

fn replace_first_field_value(body: &mut JsonValue, value: &str) {
    let JsonValue::Object(root) = body else {
        return;
    };
    let Some(JsonValue::Object(result)) = root.get_mut("result") else {
        return;
    };
    let Some(JsonValue::Object(page)) = result.get_mut("page") else {
        return;
    };
    let Some(JsonValue::Array(items)) = page.get_mut("items") else {
        return;
    };
    let Some(JsonValue::Object(item)) = items.first_mut() else {
        return;
    };
    let Some(JsonValue::Array(fields)) = item.get_mut("fields") else {
        return;
    };
    let Some(JsonValue::Object(field)) = fields.first_mut() else {
        return;
    };
    field.insert(String::from("value"), JsonValue::string(value));
}

fn field_text_bytes(field: &JsonValue) -> usize {
    let JsonValue::Object(field) = field else {
        return 0;
    };
    if field.get("availability") != Some(&JsonValue::string("available")) {
        return 0;
    }
    match (field.get("kind"), field.get("value")) {
        (Some(JsonValue::String(kind)), Some(JsonValue::String(value))) if kind == "text" => {
            value.len()
        }
        (Some(JsonValue::String(kind)), Some(JsonValue::Array(values))) if kind == "text_list" => {
            values
                .iter()
                .filter_map(|value| match value {
                    JsonValue::String(value) => Some(value.len()),
                    _ => None,
                })
                .sum()
        }
        _ => 0,
    }
}

fn recount_page(body: &mut JsonValue) {
    let JsonValue::Object(root) = body else {
        return;
    };
    let Some(JsonValue::Object(result)) = root.get_mut("result") else {
        return;
    };
    let Some(JsonValue::Object(page)) = result.get_mut("page") else {
        return;
    };
    let items = match page.get("items") {
        Some(JsonValue::Array(items)) => items.clone(),
        _ => Vec::new(),
    };
    let expected_item_bytes = items
        .iter()
        .map(|item| item.to_json().len())
        .max()
        .unwrap_or(0);
    let expected_payload_bytes = JsonValue::Array(items.clone()).to_json().len();
    let expected_text_bytes = items
        .iter()
        .filter_map(|item| match item {
            JsonValue::Object(item) => item.get("fields"),
            _ => None,
        })
        .filter_map(|fields| match fields {
            JsonValue::Array(fields) => Some(fields),
            _ => None,
        })
        .flatten()
        .map(field_text_bytes)
        .sum::<usize>();
    let mut page_without_accounting = page.clone();
    page_without_accounting.remove("accounting");
    let expected_page_bytes = JsonValue::Object(page_without_accounting).to_json().len();
    page.insert(
        String::from("accounting"),
        JsonValue::object([
            (
                String::from("item_count"),
                JsonValue::Number(items.len() as i64),
            ),
            (
                String::from("item_bytes"),
                JsonValue::Number(expected_item_bytes as i64),
            ),
            (
                String::from("payload_bytes"),
                JsonValue::Number(expected_payload_bytes as i64),
            ),
            (
                String::from("page_bytes"),
                JsonValue::Number(expected_page_bytes as i64),
            ),
            (
                String::from("text_bytes"),
                JsonValue::Number(expected_text_bytes as i64),
            ),
        ]),
    );
}

#[test]
fn returned_unicode_game_text_is_accepted_with_byte_accounting() {
    let mut response = golden("static-page-1-response");
    set_correlation(&mut response, "unicode-result");
    replace_first_field_value(&mut response, "é界😀");
    recount_page(&mut response);
    let mut server = McpServer::with_catalog(
        FakeGateway::new([successful_response(&mut response, "unicode-result")]),
        ToolCatalog::game_information_query_v1(),
    );
    let output = server.handle_frame(&frame(
        "unicode-result",
        GAME_INFORMATION_LIST_TOOL,
        static_arguments(None),
    ));
    let wire = wire_value(&output);
    assert_eq!(wire["result"]["isError"], false, "{output}");
    assert!(output.contains("é界😀"), "{output}");
}

#[test]
fn get_rejects_a_foreign_definition_with_the_same_manifest_and_kind() {
    let mut response = golden("static-page-1-response");
    set_correlation(&mut response, "foreign-definition");
    let mut requested = definition_ref();
    if let JsonValue::Object(definition) = &mut requested {
        definition.insert(
            String::from("namespaced_id"),
            JsonValue::string("other:definition"),
        );
    }
    if let JsonValue::Object(root) = &mut response {
        if let Some(JsonValue::Object(query)) = root.get_mut("query") {
            query.insert(String::from("query_kind"), JsonValue::string("get"));
            if let Some(JsonValue::Object(target)) = query.get_mut("target") {
                target.insert(String::from("definition_ref"), requested);
            }
        }
        if let Some(JsonValue::Object(result)) = root.get_mut("result")
            && let Some(JsonValue::Object(page)) = result.get_mut("page")
            && let Some(JsonValue::Array(items)) = page.get_mut("items")
            && let Some(JsonValue::Object(item)) = items.first_mut()
            && let Some(JsonValue::Object(definition)) = item.get_mut("definition_ref")
        {
            definition.insert(
                String::from("namespaced_id"),
                JsonValue::string("ironclad:defend"),
            );
        }
    }
    recount_page(&mut response);
    let mut arguments = static_arguments(None);
    if let JsonValue::Object(object) = &mut arguments {
        object.insert(String::from("definition_ref"), {
            let mut definition = definition_ref();
            if let JsonValue::Object(definition) = &mut definition {
                definition.insert(
                    String::from("namespaced_id"),
                    JsonValue::string("other:definition"),
                );
            }
            definition
        });
    }
    let mut server = McpServer::with_catalog(
        FakeGateway::new([successful_response(&mut response, "foreign-definition")]),
        ToolCatalog::game_information_query_v1(),
    );
    let output = server.handle_frame(&frame(
        "foreign-definition",
        GAME_INFORMATION_GET_TOOL,
        arguments,
    ));
    let wire = wire_value(&output);
    assert_eq!(wire["result"]["isError"], true, "{output}");
    assert_eq!(
        wire["result"]["structuredContent"]["error"]["code"], "game_information_malformed_response",
        "{output}"
    );
}

#[test]
fn unavailable_page_uses_canonical_empty_payload_accounting() {
    let mut response = golden("static-page-1-response");
    set_correlation(&mut response, "unavailable");
    let page_bytes = if let JsonValue::Object(root) = &mut response {
        let Some(JsonValue::Object(result)) = root.get_mut("result") else {
            return;
        };
        let Some(JsonValue::Object(page)) = result.get_mut("page") else {
            return;
        };
        page.insert(String::from("items"), JsonValue::Array(Vec::new()));
        page.insert(String::from("next_cursor"), JsonValue::Null);
        page.insert(String::from("cursor_binding"), JsonValue::Null);
        page.insert(String::from("final_page"), JsonValue::Bool(true));
        page.insert(String::from("total_count_known"), JsonValue::Bool(false));
        page.insert(String::from("total_count"), JsonValue::Null);
        page.insert(String::from("coverage"), JsonValue::string("unavailable"));
        let mut without_accounting = JsonValue::Object(page.clone());
        if let JsonValue::Object(object) = &mut without_accounting {
            object.remove("accounting");
        }
        let page_bytes = without_accounting.to_json().len() as i64;
        page.insert(
            String::from("accounting"),
            JsonValue::object([
                (String::from("item_count"), JsonValue::Number(0)),
                (String::from("item_bytes"), JsonValue::Number(0)),
                (String::from("payload_bytes"), JsonValue::Number(2)),
                (String::from("page_bytes"), JsonValue::Number(page_bytes)),
                (String::from("text_bytes"), JsonValue::Number(0)),
            ]),
        );
        page_bytes
    } else {
        return;
    };
    assert!(page_bytes > 2);
    let mut server = McpServer::with_catalog(
        FakeGateway::new([successful_response(&mut response, "unavailable")]),
        ToolCatalog::game_information_query_v1(),
    );
    let output = server.handle_frame(&frame(
        "unavailable",
        GAME_INFORMATION_LIST_TOOL,
        static_arguments(None),
    ));
    let wire = wire_value(&output);
    assert_eq!(wire["result"]["isError"], false, "{output}");
    assert!(output.contains("unavailable"), "{output}");
}
