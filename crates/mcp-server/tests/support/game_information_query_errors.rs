// SPDX-License-Identifier: MIT

use super::*;

#[test]
fn response_validation_preserves_typed_stale_and_read_only_errors() {
    let mut stale = golden("error-stale-cursor");
    set_correlation(&mut stale, "stale");
    let mut server = McpServer::with_catalog(
        FakeGateway::new([successful_response(&mut stale, "stale")]),
        ToolCatalog::game_information_query_v1(),
    );
    let output = server.handle_frame(&frame(
        "stale",
        GAME_INFORMATION_SEARCH_TOOL,
        static_arguments(Some("cursor:cards:1")),
    ));
    let wire = wire_value(&output);
    assert_eq!(wire["result"]["isError"], true);
    assert!(output.contains("stale_cursor"));

    let mut malformed = golden("static-page-1-response");
    set_correlation(&mut malformed, "bad-read-only");
    if let JsonValue::Object(root) = &mut malformed
        && let Some(JsonValue::Object(result)) = root.get_mut("result")
    {
        result.insert(String::from("read_only"), JsonValue::Bool(false));
    }
    let mut server = McpServer::with_catalog(
        FakeGateway::new([successful_response(&mut malformed, "bad-read-only")]),
        ToolCatalog::game_information_query_v1(),
    );
    let output = server.handle_frame(&frame(
        "bad-read-only",
        GAME_INFORMATION_LIST_TOOL,
        static_arguments(None),
    ));
    assert_eq!(wire_value(&output)["result"]["isError"], true);
    assert!(output.contains("read-only"));

    let mut timeout_server = McpServer::with_catalog(
        FakeGateway::new([Err(GatewayError::Timeout)]),
        ToolCatalog::game_information_query_v1(),
    );
    let output = timeout_server.handle_frame(&frame(
        "timeout",
        GAME_INFORMATION_LIST_TOOL,
        static_arguments(None),
    ));
    assert!(output.contains("-32008"));
    assert!(output.contains("timed out"));
}
