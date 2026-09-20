// SPDX-License-Identifier: MIT
#![allow(clippy::expect_used, clippy::panic)]

use std::collections::VecDeque;

use serde_json::Value;
use sts2_mcp_server::{
    GAME_INFORMATION_AVAILABILITY_TOOL, GAME_INFORMATION_BINDING_TOOL,
    GAME_INFORMATION_CAPABILITIES_TOOL, GAME_INFORMATION_CONTENT_MANIFEST_TOOL,
    GAME_INFORMATION_DETAIL_TOOL, GAME_INFORMATION_GET_TOOL, GAME_INFORMATION_LIST_TOOL,
    GAME_INFORMATION_SEARCH_TOOL, GatewayAdapter, GatewayError, GatewayMethod, GatewayRequest,
    GatewayResponse, JsonValue, McpServer, ToolCatalog, parse_json,
    verify_game_information_artifact,
};

#[path = "support/game_information_query_content_manifest.rs"]
mod content_manifest;

#[path = "support/game_information_query_content_manifest_vectors.rs"]
mod content_manifest_vectors;

#[path = "support/game_information_query_errors.rs"]
mod errors;

#[path = "support/game_information_query_extra.rs"]
mod extra;

#[path = "support/game_information_query_regressions.rs"]
mod regressions;

struct FakeGateway {
    requests: Vec<GatewayRequest>,
    responses: VecDeque<Result<GatewayResponse, GatewayError>>,
}

impl FakeGateway {
    fn new(responses: impl IntoIterator<Item = Result<GatewayResponse, GatewayError>>) -> Self {
        Self {
            requests: Vec::new(),
            responses: responses.into_iter().collect(),
        }
    }
}

impl GatewayAdapter for FakeGateway {
    fn forward(&mut self, request: GatewayRequest) -> Result<GatewayResponse, GatewayError> {
        self.requests.push(request);
        self.responses
            .pop_front()
            .unwrap_or(Err(GatewayError::Unavailable))
    }
}

fn golden(name: &str) -> JsonValue {
    let text = match name {
        "capabilities-response" => include_str!(
            "../../../protocol-artifact/game-information-query-v1/golden/capabilities-response.json"
        ),
        "error-stale-cursor" => include_str!(
            "../../../protocol-artifact/game-information-query-v1/golden/error-stale-cursor.json"
        ),
        "live-detail-request" => include_str!(
            "../../../protocol-artifact/game-information-query-v1/golden/live-detail-request.json"
        ),
        "live-detail-response" => include_str!(
            "../../../protocol-artifact/game-information-query-v1/golden/live-detail-response.json"
        ),
        "static-page-1-request" => include_str!(
            "../../../protocol-artifact/game-information-query-v1/golden/static-page-1-request.json"
        ),
        "static-page-1-response" => include_str!(
            "../../../protocol-artifact/game-information-query-v1/golden/static-page-1-response.json"
        ),
        "static-page-2-request" => include_str!(
            "../../../protocol-artifact/game-information-query-v1/golden/static-page-2-request.json"
        ),
        "static-page-2-response" => include_str!(
            "../../../protocol-artifact/game-information-query-v1/golden/static-page-2-response.json"
        ),
        _ => panic!("unsupported golden {name}"),
    };
    parse_json(text).expect("accepted golden JSON")
}

fn set_correlation(body: &mut JsonValue, correlation: &str) {
    if let JsonValue::Object(object) = body {
        object.insert(
            String::from("correlation_id"),
            JsonValue::string(correlation),
        );
    }
}

fn frame(id: &str, name: &str, arguments: JsonValue) -> String {
    JsonValue::object([
        (String::from("jsonrpc"), JsonValue::string("2.0")),
        (String::from("id"), JsonValue::string(id)),
        (String::from("method"), JsonValue::string("tools/call")),
        (
            String::from("params"),
            JsonValue::object([
                (String::from("name"), JsonValue::string(name)),
                (String::from("arguments"), arguments),
            ]),
        ),
    ])
    .to_json()
}

fn context() -> JsonValue {
    JsonValue::object([
        (String::from("instance_id"), JsonValue::string("instance-1")),
        (
            String::from("mcp_session_id"),
            JsonValue::string("mcp-session-1"),
        ),
        (String::from("lease_id"), JsonValue::string("lease-1")),
        (String::from("lease_epoch"), JsonValue::Number(7)),
    ])
}

fn static_arguments(cursor: Option<&str>) -> JsonValue {
    let mut arguments = context();
    let JsonValue::Object(object) = &mut arguments else {
        return JsonValue::Null;
    };
    for (key, value) in [
        ("content_manifest_id", JsonValue::string("content-1")),
        ("locale", JsonValue::string("en-US")),
        ("visibility_scope", JsonValue::string("public")),
        ("entity_kind", JsonValue::string("card")),
        ("projection", JsonValue::string("summary")),
        ("detail_level", JsonValue::string("summary")),
        (
            "fields",
            JsonValue::Array(vec![JsonValue::string("display_name")]),
        ),
        ("page_items", JsonValue::Number(2)),
        ("item_bytes", JsonValue::Number(4_096)),
        ("page_bytes", JsonValue::Number(65_536)),
        ("text_bytes", JsonValue::Number(1_024)),
    ] {
        object.insert(String::from(key), value);
    }
    object.insert(
        String::from("cursor"),
        cursor.map_or(JsonValue::Null, JsonValue::string),
    );
    arguments
}

fn definition_ref() -> JsonValue {
    JsonValue::object([
        (
            String::from("content_manifest_id"),
            JsonValue::string("content-1"),
        ),
        (String::from("entity_kind"), JsonValue::string("card")),
        (
            String::from("namespaced_id"),
            JsonValue::string("ironclad:strike"),
        ),
        (String::from("variant"), JsonValue::Null),
    ])
}

fn live_arguments() -> JsonValue {
    parse_json(
        r#"{
          "instance_id":"instance-1","mcp_session_id":"mcp-session-1",
          "lease_id":"lease-1","lease_epoch":7,
          "content_manifest_id":"content-1","locale":"en-US",
          "visibility_scope":"player","entity_kind":"card",
          "projection":"full","detail_level":"full","fields":["amount","cost","description","display_name"],"page_items":1,
          "item_bytes":4096,"page_bytes":65536,"text_bytes":4096,
          "definition_ref":{"content_manifest_id":"content-1","entity_kind":"card","namespaced_id":"ironclad:strike","variant":null},
          "instance_ref":{"instance_id":"instance-1","run_id":"run-1","epoch":7,"entity_kind":"card","entity_id":"card-17"},
          "snapshot_ref":{"snapshot_id":"snapshot-42","instance_ref":{"instance_id":"instance-1","run_id":"run-1","epoch":7,"entity_kind":"card","entity_id":"card-17"},"state_generation":42},
          "parent_observation":{"instance_ref":{"instance_id":"instance-1","run_id":"run-1","epoch":7,"entity_kind":"card","entity_id":"card-17"},"snapshot_ref":{"snapshot_id":"snapshot-42","instance_ref":{"instance_id":"instance-1","run_id":"run-1","epoch":7,"entity_kind":"card","entity_id":"card-17"},"state_generation":42},"state_generation":42}
        }"#,
    )
    .expect("live arguments JSON")
}

fn wire_value(output: &str) -> Value {
    serde_json::from_str(output).expect("MCP response JSON")
}

fn successful_response(
    body: &mut JsonValue,
    correlation: &str,
) -> Result<GatewayResponse, GatewayError> {
    set_correlation(body, correlation);
    Ok(GatewayResponse {
        status: 200,
        body: body.clone(),
    })
}

#[test]
fn catalog_is_versioned_strict_and_read_only() {
    let mut server = McpServer::with_catalog(
        FakeGateway::new([]),
        ToolCatalog::game_information_query_v1(),
    );
    let wire = wire_value(
        &server.handle_frame(r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#),
    );
    assert_eq!(wire["result"]["revision"], "game-information-query-v1-mcp");
    let tools = wire["result"]["tools"].as_array().expect("tools array");
    assert_eq!(tools.len(), 8);
    for tool in tools {
        assert_eq!(tool["annotations"]["readOnlyHint"], true);
        assert_eq!(tool["annotations"]["destructiveHint"], false);
        assert_eq!(tool["annotations"]["idempotentHint"], true);
        assert_eq!(tool["inputSchema"]["additionalProperties"], false);
    }
    let names: Vec<_> = tools
        .iter()
        .filter_map(|tool| tool["name"].as_str())
        .collect();
    assert!(names.contains(&GAME_INFORMATION_CAPABILITIES_TOOL));
    assert!(names.contains(&GAME_INFORMATION_LIST_TOOL));
    assert!(names.contains(&GAME_INFORMATION_SEARCH_TOOL));
    assert!(names.contains(&GAME_INFORMATION_GET_TOOL));
    assert!(names.contains(&GAME_INFORMATION_DETAIL_TOOL));
    assert!(names.contains(&GAME_INFORMATION_AVAILABILITY_TOOL));
    assert!(names.contains(&GAME_INFORMATION_BINDING_TOOL));
    assert!(names.contains(&GAME_INFORMATION_CONTENT_MANIFEST_TOOL));
}

#[test]
fn fake_gateway_proves_capabilities_search_get_detail_and_cursor_routes() {
    let mut capabilities = golden("capabilities-response");
    set_correlation(&mut capabilities, "capabilities");
    let mut page_one = golden("static-page-1-response");
    set_correlation(&mut page_one, "list-one");
    let mut page_two = golden("static-page-2-response");
    set_correlation(&mut page_two, "list-two");
    let mut stale_search = golden("error-stale-cursor");
    set_correlation(&mut stale_search, "search");
    let mut stale_get = golden("error-stale-cursor");
    set_correlation(&mut stale_get, "get");
    let mut detail = golden("live-detail-response");
    set_correlation(&mut detail, "detail");
    let mut stale_availability = golden("error-stale-cursor");
    set_correlation(&mut stale_availability, "availability");
    let mut server = McpServer::with_catalog_and_sessions(
        FakeGateway::new([
            successful_response(&mut capabilities, "capabilities"),
            successful_response(&mut page_one, "list-one"),
            successful_response(&mut page_two, "list-two"),
            successful_response(&mut stale_search, "search"),
            successful_response(&mut stale_get, "get"),
            successful_response(&mut detail, "detail"),
            successful_response(&mut stale_availability, "availability"),
        ]),
        ToolCatalog::game_information_query_v1(),
        "gateway-session-1",
        "mcp-session-1",
    );

    let calls = [
        (
            "capabilities",
            GAME_INFORMATION_CAPABILITIES_TOOL,
            context(),
        ),
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
        ("detail", GAME_INFORMATION_DETAIL_TOOL, live_arguments()),
        ("availability", GAME_INFORMATION_AVAILABILITY_TOOL, {
            let mut value = static_arguments(None);
            if let JsonValue::Object(object) = &mut value {
                object.insert(String::from("mode"), JsonValue::string("static"));
            }
            value
        }),
    ];
    for (index, (id, name, arguments)) in calls.iter().enumerate() {
        let wire = wire_value(&server.handle_frame(&frame(id, name, arguments.clone())));
        assert!(wire["result"].is_object(), "{wire}");
        let expected_error = matches!(index, 3 | 4 | 6);
        assert_eq!(
            wire["result"]["isError"], expected_error,
            "unexpected query outcome for {name}: {wire}"
        );
    }
    assert_eq!(server.gateway().requests.len(), calls.len());
    assert_eq!(server.gateway().requests[0].method, GatewayMethod::Get);
    assert_eq!(
        server.gateway().requests[0].path,
        "/v1/instances/instance-1/game-information/capabilities"
    );
    assert!(server.gateway().requests[0].body.is_none());
    for (name, expected) in [
        ("x-sts2-instance-id", "instance-1"),
        ("x-sts2-session-id", "gateway-session-1"),
        ("x-sts2-lease-id", "lease-1"),
        ("x-sts2-lease-epoch", "7"),
        ("x-mcp-session-id", "mcp-session-1"),
    ] {
        assert_eq!(
            server.gateway().requests[0]
                .headers
                .get(name)
                .map(String::as_str),
            Some(expected),
            "capabilities header {name}"
        );
    }
    let mut expected_page_one = golden("static-page-1-request");
    set_correlation(&mut expected_page_one, "list-one");
    assert_eq!(
        server.gateway().requests[1].body.as_ref(),
        Some(&expected_page_one)
    );
    let mut expected_page_two = golden("static-page-2-request");
    set_correlation(&mut expected_page_two, "list-two");
    assert_eq!(
        server.gateway().requests[2].body.as_ref(),
        Some(&expected_page_two)
    );
    let mut expected_detail = golden("live-detail-request");
    set_correlation(&mut expected_detail, "detail");
    assert_eq!(
        server.gateway().requests[5].body.as_ref(),
        Some(&expected_detail)
    );
    for (index, request) in server.gateway().requests.iter().enumerate().skip(1) {
        assert_eq!(request.method, GatewayMethod::Post);
        assert_eq!(
            request.path,
            "/v1/instances/instance-1/game-information/query"
        );
        assert!(request.body.is_some(), "query body {index}");
        assert_eq!(
            request.headers.get("x-sts2-session-id").map(String::as_str),
            Some("gateway-session-1")
        );
        assert_eq!(
            request.headers.get("x-mcp-session-id").map(String::as_str),
            Some("mcp-session-1")
        );
        for (name, expected) in [
            ("x-sts2-instance-id", "instance-1"),
            ("x-sts2-session-id", "gateway-session-1"),
            ("x-sts2-lease-id", "lease-1"),
            ("x-sts2-lease-epoch", "7"),
        ] {
            assert_eq!(
                request.headers.get(name).map(String::as_str),
                Some(expected),
                "query header {name}"
            );
        }
        assert!(
            request
                .body
                .as_ref()
                .is_some_and(|body| body.to_json().len() <= 16_384),
            "query body {index} exceeds the adapter bound"
        );
    }
}
