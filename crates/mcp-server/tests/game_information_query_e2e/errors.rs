// SPDX-License-Identifier: MIT
//! Typed-error acceptance for issue #51: malformed input and fields, unknown
//! references, stale cursors, oversized requests and results, missing
//! capability, and unmapped tools all fail closed without reaching the
//! producer with an arbitrary request.

use serde_json::{Value, json};
use sts2_mcp_server::{
    Correlation, GAME_INFORMATION_DETAIL_TOOL, GAME_INFORMATION_GET_TOOL,
    GAME_INFORMATION_SEARCH_TOOL, GatewayAdapter, GatewayError, GatewayMethod, GatewayRequest,
    METHOD_NOT_FOUND, RequestId, parse_json,
};

use super::producer::{
    GATEWAY_SESSION_ID, INSTANCE_ID, LEASE_ID, LIVE_ENTITY_ID, MCP_SESSION_ID, ProducerMode,
    QUERY_PATH, SyntheticProducer,
};
use super::schema;
use super::{
    assert_tool_error, attempt, base_query, call, get_arguments, live_arguments, request,
    search_arguments, server, tool_envelope,
};

const GOLDEN_STATIC_REQUEST: &str = include_str!(
    "../../../../protocol-artifact/game-information-query-v1/golden/static-page-1-request.json"
);
const GOLDEN_STATIC_RESPONSE: &str = include_str!(
    "../../../../protocol-artifact/game-information-query-v1/golden/static-page-1-response.json"
);
const GOLDEN_CAPABILITIES_RESPONSE: &str = include_str!(
    "../../../../protocol-artifact/game-information-query-v1/golden/capabilities-response.json"
);
const GOLDEN_ERROR_RESPONSE: &str = include_str!(
    "../../../../protocol-artifact/game-information-query-v1/golden/error-stale-cursor.json"
);

fn golden(text: &str) -> Value {
    serde_json::from_str(text).expect("golden contract JSON is valid")
}

/// Builds the mapped gateway request one query envelope is delivered on, so the
/// controls drive the producer's own schema validation through `forward`.
fn mapped_query_request(body: &Value) -> GatewayRequest {
    let correlation = body
        .get("correlation_id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let mut headers = std::collections::BTreeMap::new();
    for (name, value) in [
        ("x-sts2-instance-id", INSTANCE_ID),
        ("x-sts2-session-id", GATEWAY_SESSION_ID),
        ("x-sts2-lease-id", LEASE_ID),
        ("x-sts2-lease-epoch", "7"),
        ("x-mcp-session-id", MCP_SESSION_ID),
        ("x-mcp-request-id", correlation.as_str()),
    ] {
        headers.insert(String::from(name), String::from(value));
    }
    GatewayRequest {
        method: GatewayMethod::Post,
        path: String::from(QUERY_PATH),
        headers,
        body: Some(parse_json(&body.to_string()).expect("mapped request body is JSON")),
        correlation: Correlation {
            mcp_session_id: String::from(MCP_SESSION_ID),
            mcp_request_id: RequestId::String(correlation),
        },
    }
}

fn without_member(value: &Value, key: &str) -> Value {
    let mut value = value.clone();
    value
        .as_object_mut()
        .expect("envelope is an object")
        .remove(key);
    value
}

fn with_extra_member(value: &Value, key: &str) -> Value {
    let mut value = value.clone();
    value
        .as_object_mut()
        .expect("envelope is an object")
        .insert(String::from(key), json!(true));
    value
}

/// The producer independently validates every mapped request against the
/// pinned schema, so an envelope that drops a required member or grows an
/// extra root member is refused instead of reaching the query handler.
#[test]
fn producer_rejects_malformed_mapped_requests_against_the_pinned_schema() {
    let mut producer = SyntheticProducer::contract();
    let conforming = golden(GOLDEN_STATIC_REQUEST);

    let accepted = producer
        .forward(mapped_query_request(&conforming))
        .expect("a conforming mapped request is accepted");
    assert_eq!(accepted.status, 200);
    assert_eq!(producer.records().len(), 1);
    assert!(
        schema::errors(producer.validator(), &conforming).is_empty(),
        "the pinned request golden must satisfy the pinned schema"
    );
    assert!(
        producer.violations().is_empty(),
        "a conforming request must not be a violation: {:?}",
        producer.violations()
    );

    for (name, malformed) in [
        (
            "missing required result member",
            without_member(&conforming, "result"),
        ),
        (
            "missing required capabilities member",
            without_member(&conforming, "capabilities"),
        ),
        (
            "missing required error member",
            without_member(&conforming, "error"),
        ),
        (
            "missing required correlation member",
            without_member(&conforming, "correlation_id"),
        ),
        (
            "extra root member",
            with_extra_member(&conforming, "unexpected"),
        ),
    ] {
        let before = producer.violations().len();
        let refusal = producer.forward(mapped_query_request(&malformed));
        assert_eq!(refusal, Err(GatewayError::MalformedResponse), "{name}");
        assert!(
            producer.violations().len() > before,
            "{name} did not record a schema violation"
        );
    }
    assert_eq!(
        producer.records().len(),
        1,
        "a schema-invalid request must not be recorded as a processed query"
    );
}

/// The same validator proves the response direction: pinned outbound goldens
/// conform, while dropping a required member or adding an extra root member is
/// reported instead of silently projected.
#[test]
fn producer_rejects_schema_violating_responses_against_the_pinned_schema() {
    let producer = SyntheticProducer::contract();
    for (name, response) in [
        ("capabilities response", GOLDEN_CAPABILITIES_RESPONSE),
        ("static page response", GOLDEN_STATIC_RESPONSE),
        ("error response", GOLDEN_ERROR_RESPONSE),
    ] {
        let response = golden(response);
        let errors = schema::errors(producer.validator(), &response);
        assert!(errors.is_empty(), "{name} is not schema-valid: {errors:?}");
    }

    let response = golden(GOLDEN_STATIC_RESPONSE);
    assert!(
        !schema::errors(producer.validator(), &without_member(&response, "result")).is_empty(),
        "a response without its required result member must be rejected"
    );
    assert!(
        !schema::errors(
            producer.validator(),
            &with_extra_member(&response, "unexpected")
        )
        .is_empty(),
        "a response with an extra root member must be rejected"
    );
}

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
