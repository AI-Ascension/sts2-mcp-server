// SPDX-License-Identifier: MIT
//! Typed-error acceptance for issue #51: malformed input and fields, unknown
//! references, stale cursors, oversized requests and results, missing
//! capability, and unmapped tools all fail closed without reaching the
//! producer with an arbitrary request.

use serde_json::{Value, json};
use sts2_mcp_server::{
    GAME_INFORMATION_DETAIL_TOOL, GAME_INFORMATION_GET_TOOL, GAME_INFORMATION_SEARCH_TOOL,
    METHOD_NOT_FOUND,
};

use super::producer::{LIVE_ENTITY_ID, ProducerMode, QUERY_PATH, SyntheticProducer};
use super::{
    assert_tool_error, attempt, base_query, call, get_arguments, live_arguments, request,
    search_arguments, server, tool_envelope,
};

#[test]
fn malformed_unknown_stale_oversized_and_missing_capability_fail_closed() {
    let mut server = server(SyntheticProducer::contract());

    let mut unknown_field = search_arguments(None, "summary");
    unknown_field["unexpected"] = json!(true);
    let wire = attempt(
        &mut server,
        "malformed-argument",
        GAME_INFORMATION_SEARCH_TOOL,
        unknown_field,
    );
    assert_eq!(wire["error"]["code"], -32602, "{wire}");

    let mut oversized_request = base_query();
    oversized_request["page_items"] = json!(129);
    let wire = attempt(
        &mut server,
        "oversized-request",
        GAME_INFORMATION_SEARCH_TOOL,
        oversized_request,
    );
    assert_eq!(wire["error"]["code"], -32602, "{wire}");

    let mut oversized_cursor = search_arguments(None, "summary");
    oversized_cursor["cursor"] = json!("c".repeat(513));
    let wire = attempt(
        &mut server,
        "oversized-cursor",
        GAME_INFORMATION_SEARCH_TOOL,
        oversized_cursor,
    );
    assert_eq!(wire["error"]["code"], -32602, "{wire}");
    assert!(
        server.gateway().records().is_empty(),
        "refused requests must never reach the producer"
    );

    let wire = call(
        &mut server,
        "unknown",
        GAME_INFORMATION_GET_TOOL,
        get_arguments("synthetic:missing"),
    );
    assert_tool_error(&wire, "unknown_id", "missing");

    let wire = call(
        &mut server,
        "cursor-source",
        GAME_INFORMATION_SEARCH_TOOL,
        search_arguments(None, "summary"),
    );
    let cursor = tool_envelope(&wire)["result"]["page"]["next_cursor"]
        .as_str()
        .expect("continuation cursor")
        .to_owned();
    let wire = call(
        &mut server,
        "stale",
        GAME_INFORMATION_SEARCH_TOOL,
        search_arguments(Some(&cursor), "full"),
    );
    assert_tool_error(&wire, "stale_cursor", "stale");

    let wire = call(
        &mut server,
        "missing-capability",
        GAME_INFORMATION_DETAIL_TOOL,
        live_arguments("relic", LIVE_ENTITY_ID, "synthetic:ember"),
    );
    assert_tool_error(&wire, "missing_capability", "missing");

    let mut unsupported = base_query();
    unsupported["entity_kind"] = json!("relic");
    unsupported["fields"] = json!(["amount"]);
    let wire = call(
        &mut server,
        "unsupported-field",
        GAME_INFORMATION_SEARCH_TOOL,
        unsupported,
    );
    assert_tool_error(&wire, "unsupported_field", "unsupported");

    let wire = request(
        &mut server,
        "unknown-tool",
        "tools/call",
        json!({"name": "sts2.not_a_tool", "arguments": {}}),
    );
    assert_eq!(wire["error"]["code"], METHOD_NOT_FOUND, "{wire}");

    for record in server.gateway().records() {
        assert_eq!(record.path, QUERY_PATH);
    }
    assert!(
        server.gateway().violations().is_empty(),
        "{:?}",
        server.gateway().violations()
    );
}

/// Caller bounds large enough for the producer to build a page whose envelope
/// exceeds the pinned message bound while every declared limit still holds.
fn max_bound_arguments() -> Value {
    let mut arguments = search_arguments(None, "summary");
    arguments["page_items"] = json!(128);
    arguments["item_bytes"] = json!(262144);
    arguments["page_bytes"] = json!(262144);
    arguments["text_bytes"] = json!(65536);
    arguments
}

#[test]
fn oversized_and_corrupted_producer_data_are_rejected_at_the_boundary() {
    let mut oversized = server(SyntheticProducer::with_mode(ProducerMode::OversizedPage));
    let wire = call(
        &mut oversized,
        "oversized",
        GAME_INFORMATION_SEARCH_TOOL,
        max_bound_arguments(),
    );
    assert_tool_error(&wire, "game_information_response_too_large", "size");
    assert_eq!(oversized.gateway().records().len(), 1);
    assert!(oversized.gateway().violations().is_empty());

    let mut corrupt = server(SyntheticProducer::with_mode(ProducerMode::CorruptEnvelope));
    let wire = call(
        &mut corrupt,
        "corrupt",
        GAME_INFORMATION_SEARCH_TOOL,
        search_arguments(None, "summary"),
    );
    assert_tool_error(
        &wire,
        "game_information_malformed_response",
        "malformed_response",
    );
    assert_eq!(corrupt.gateway().records().len(), 1);
    assert!(corrupt.gateway().violations().is_empty());
}
