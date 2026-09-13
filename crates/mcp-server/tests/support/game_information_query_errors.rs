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
    assert_eq!(
        wire["result"]["structuredContent"]["error"]["code"],
        "stale_cursor"
    );
    assert_eq!(
        wire["result"]["structuredContent"]["error"]["category"],
        "stale"
    );

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
    let wire = wire_value(&output);
    assert_eq!(wire["result"]["isError"], true);
    assert_eq!(
        wire["result"]["structuredContent"]["error"]["code"], "game_information_malformed_response",
        "{output}"
    );
    assert_eq!(
        wire["result"]["structuredContent"]["error"]["category"], "malformed_response",
        "{output}"
    );
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
    let wire = wire_value(&output);
    assert_eq!(
        wire["result"]["structuredContent"]["error"]["code"],
        "gateway_timeout"
    );
    assert_eq!(
        wire["result"]["structuredContent"]["error"]["category"],
        "transport"
    );
    assert_eq!(
        wire["result"]["structuredContent"]["error"]["mcp_code"],
        -32008
    );
    assert!(output.contains("-32008"));
    assert!(output.contains("timed out"));
}

#[test]
fn gateway_failures_expose_stable_machine_readable_codes() {
    for (error, expected_code, expected_category, expected_mcp_code) in [
        (
            GatewayError::Unauthorized,
            "gateway_unauthorized",
            "denied",
            -32001,
        ),
        (
            GatewayError::Forbidden,
            "gateway_forbidden",
            "denied",
            -32007,
        ),
        (
            GatewayError::NotFound,
            "gateway_not_found",
            "missing",
            -32004,
        ),
        (
            GatewayError::Unavailable,
            "gateway_unavailable",
            "transport",
            -32003,
        ),
        (
            GatewayError::Timeout,
            "gateway_timeout",
            "transport",
            -32008,
        ),
        (
            GatewayError::MalformedResponse,
            "gateway_malformed_response",
            "malformed_response",
            -32002,
        ),
        (
            GatewayError::ResponseTooLarge,
            "gateway_response_too_large",
            "size",
            -32006,
        ),
        (
            GatewayError::Rejected,
            "gateway_rejected",
            "invalid_input",
            -32005,
        ),
    ] {
        let mut server = McpServer::with_catalog(
            FakeGateway::new([Err(error)]),
            ToolCatalog::game_information_query_v1(),
        );
        let output = server.handle_frame(&frame(
            expected_code,
            GAME_INFORMATION_LIST_TOOL,
            static_arguments(None),
        ));
        let wire = wire_value(&output);
        assert_eq!(
            wire["result"]["structuredContent"]["error"]["code"], expected_code,
            "{output}"
        );
        assert_eq!(
            wire["result"]["structuredContent"]["error"]["category"], expected_category,
            "{output}"
        );
        assert_eq!(
            wire["result"]["structuredContent"]["error"]["mcp_code"], expected_mcp_code,
            "{output}"
        );
    }
}
