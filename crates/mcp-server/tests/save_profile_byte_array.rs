// SPDX-License-Identifier: MIT

//! Byte-array downstream carrier regressions for `save-profile-v1`.
//!
//! The gateway serializes a save-profile result body as a JSON array of byte
//! values, so these regressions keep that decode bounded, single, and closed.
//! The catalog, projection, and reconciliation goldens stay in
//! `save_profile_mapping.rs`.

#![allow(clippy::expect_used, clippy::panic)]

use sts2_mcp_server::{
    GatewayResponse, JsonValue, McpServer, SAVE_PROFILE_CURRENT_TOOL, SAVE_PROFILE_LIST_TOOL,
    SAVE_PROFILE_MAX_BODY_BYTES, SAVE_PROFILE_SELECT_TOOL, ToolCatalog,
};

#[path = "support/save_profile_mapping.rs"]
#[allow(dead_code)]
mod support;
use support::*;

#[test]
fn gateway_byte_array_downstream_projects_for_reads_and_settlement() {
    let body = r#"{"schema_revision":"save-profile-downstream-v1","save_profile_id":"slot-2","instance_id":"instance-1","lease_epoch":7,"status":"settled"}"#;
    let responses = [
        Ok(GatewayResponse {
            status: 200,
            body: with_downstream(result("list-1", "list", "settled"), byte_array(body)),
        }),
        Ok(GatewayResponse {
            status: 200,
            body: with_downstream(result("current-1", "current", "settled"), byte_array(body)),
        }),
        Ok(GatewayResponse {
            status: 200,
            body: with_downstream(result("select-1", "select", "settled"), byte_array(body)),
        }),
    ];
    let mut server = McpServer::with_catalog_and_sessions(
        FakeGateway::new(responses),
        ToolCatalog::save_profile_v1(),
        "gateway-session-1",
        "mcp-session-1",
    );
    let calls = [
        ("list-1", SAVE_PROFILE_LIST_TOOL, context()),
        ("current-1", SAVE_PROFILE_CURRENT_TOOL, context()),
        (
            "select-1",
            SAVE_PROFILE_SELECT_TOOL,
            select_arguments("slot-2"),
        ),
    ];
    for (id, name, arguments) in &calls {
        let output = wire(&server.handle_frame(&frame(id, name, arguments.clone())));
        assert_eq!(output["result"]["isError"], false, "{output}");
        let projected: serde_json::Value = serde_json::from_str(
            output["result"]["content"][0]["text"]
                .as_str()
                .expect("tool result text"),
        )
        .expect("tool result body");
        assert_eq!(projected["operation_id"], *id);
        assert_eq!(
            projected["downstream"],
            serde_json::json!({
                "revision": "save-profile-downstream-v1",
                "status": "settled",
                "instance_id": "instance-1",
                "lease_epoch": 7,
                "profile_id": "slot-2",
            }),
            "{output}"
        );
    }
}

#[test]
fn byte_array_downstream_is_bounded_by_decoded_body_size() {
    let prefix = r#"{"revision":"save-profile-downstream-v1"}"#;
    // Trailing JSON whitespace keeps the decoded body valid at the exact bound.
    let at_limit = format!(
        "{prefix}{}",
        " ".repeat(SAVE_PROFILE_MAX_BODY_BYTES - prefix.len())
    );
    assert_eq!(at_limit.len(), SAVE_PROFILE_MAX_BODY_BYTES);
    let over_limit = format!("{at_limit} ");
    let mut server = McpServer::with_catalog_and_sessions(
        FakeGateway::new([
            Ok(GatewayResponse {
                status: 200,
                body: with_downstream(result("list-1", "list", "settled"), byte_array(&at_limit)),
            }),
            Ok(GatewayResponse {
                status: 200,
                body: with_downstream(
                    result("select-1", "select", "settled"),
                    byte_array(&over_limit),
                ),
            }),
        ]),
        ToolCatalog::save_profile_v1(),
        "gateway-session-1",
        "mcp-session-1",
    );
    let listed = wire(&server.handle_frame(&frame("list-1", SAVE_PROFILE_LIST_TOOL, context())));
    assert_eq!(listed["result"]["isError"], false, "{listed}");
    let projected: serde_json::Value = serde_json::from_str(
        listed["result"]["content"][0]["text"]
            .as_str()
            .expect("tool result text"),
    )
    .expect("tool result body");
    assert_eq!(
        projected["downstream"],
        serde_json::json!({"revision": "save-profile-downstream-v1"})
    );

    let oversized = wire(&server.handle_frame(&frame(
        "select-1",
        SAVE_PROFILE_SELECT_TOOL,
        select_arguments("slot-2"),
    )));
    assert_eq!(oversized["result"]["isError"], true);
    assert_eq!(
        oversized["result"]["structuredContent"]["error"]["code"],
        "save_profile_unknown_after_oversized_response"
    );
    let body: serde_json::Value = serde_json::from_str(
        oversized["result"]["content"][0]["text"]
            .as_str()
            .expect("unknown result text"),
    )
    .expect("unknown result body");
    assert_eq!(body["status"], "unknown");
    assert_eq!(body["guidance"]["code"], "save_profile_reconcile_required");
}

#[test]
fn malformed_byte_array_downstream_fails_closed() {
    let cases = [
        JsonValue::Array(vec![JsonValue::Number(0xff), JsonValue::Number(0xfe)]),
        byte_array("not json"),
        byte_array("[123,125]"),
        byte_array("[]"),
        byte_array("123"),
        JsonValue::Array(vec![JsonValue::Number(256)]),
        JsonValue::Array(vec![JsonValue::Number(0), JsonValue::Bool(true)]),
    ];
    let case_count = cases.len();
    let responses = cases.into_iter().map(|downstream| {
        Ok(GatewayResponse {
            status: 200,
            body: with_downstream(result("list-1", "list", "settled"), downstream),
        })
    });
    let mut server = McpServer::with_catalog_and_sessions(
        FakeGateway::new(responses),
        ToolCatalog::save_profile_v1(),
        "gateway-session-1",
        "mcp-session-1",
    );
    for case in 0..case_count {
        let output =
            wire(&server.handle_frame(&frame("list-1", SAVE_PROFILE_LIST_TOOL, context())));
        assert_eq!(output["result"]["isError"], true, "case {case}: {output}");
        assert_eq!(
            output["result"]["structuredContent"]["error"]["code"],
            "save_profile_malformed_response",
            "case {case}: {output}"
        );
    }
    assert_eq!(server.gateway().requests.len(), case_count);
}

#[test]
fn decoded_array_downstream_is_not_decoded_twice() {
    // `[123,125]` is the byte-encoded form of a downstream body that is itself a
    // JSON array. It must fail closed as an unsupported content shape instead of
    // being decoded a second time into `{}` and reported as settled.
    let body = byte_array("[123,125]");
    let mut server = McpServer::with_catalog_and_sessions(
        FakeGateway::new([
            Ok(GatewayResponse {
                status: 200,
                body: with_downstream(result("list-1", "list", "settled"), body.clone()),
            }),
            Ok(GatewayResponse {
                status: 200,
                body: with_downstream(result("select-1", "select", "settled"), body),
            }),
        ]),
        ToolCatalog::save_profile_v1(),
        "gateway-session-1",
        "mcp-session-1",
    );

    let listed = wire(&server.handle_frame(&frame("list-1", SAVE_PROFILE_LIST_TOOL, context())));
    assert_eq!(listed["result"]["isError"], true, "{listed}");
    assert_eq!(
        listed["result"]["structuredContent"]["error"]["code"],
        "save_profile_malformed_response"
    );
    assert_eq!(
        listed["result"]["structuredContent"]["error"]["message"],
        "save-profile downstream content must be an object or null"
    );

    let selected = wire(&server.handle_frame(&frame(
        "select-1",
        SAVE_PROFILE_SELECT_TOOL,
        select_arguments("slot-2"),
    )));
    assert_eq!(selected["result"]["isError"], true, "{selected}");
    assert_eq!(
        selected["result"]["structuredContent"]["error"]["code"],
        "save_profile_unknown_after_malformed_response"
    );
    let unknown: serde_json::Value = serde_json::from_str(
        selected["result"]["content"][0]["text"]
            .as_str()
            .expect("unknown result text"),
    )
    .expect("unknown result body");
    assert_eq!(unknown["status"], "unknown");
    assert_eq!(unknown["downstream"], serde_json::Value::Null);
    assert_eq!(
        unknown["guidance"]["code"],
        "save_profile_reconcile_required"
    );
}

#[test]
fn oversized_byte_array_carrier_outranks_element_validity() {
    // Size takes precedence over element validity: an over-limit carrier is
    // classified oversized even though its last element is not a byte at all.
    let mut values: Vec<JsonValue> = (0..=SAVE_PROFILE_MAX_BODY_BYTES)
        .map(|_| JsonValue::Number(0))
        .collect();
    values.push(JsonValue::Bool(true));
    assert!(values.len() > SAVE_PROFILE_MAX_BODY_BYTES);
    let carrier = JsonValue::Array(values);
    let mut server = McpServer::with_catalog_and_sessions(
        FakeGateway::new([
            Ok(GatewayResponse {
                status: 200,
                body: with_downstream(result("list-1", "list", "settled"), carrier.clone()),
            }),
            Ok(GatewayResponse {
                status: 200,
                body: with_downstream(result("select-1", "select", "settled"), carrier),
            }),
        ]),
        ToolCatalog::save_profile_v1(),
        "gateway-session-1",
        "mcp-session-1",
    );

    let listed = wire(&server.handle_frame(&frame("list-1", SAVE_PROFILE_LIST_TOOL, context())));
    assert_eq!(listed["result"]["isError"], true, "{listed}");
    assert_eq!(
        listed["result"]["structuredContent"]["error"]["code"],
        "save_profile_response_too_large"
    );

    let selected = wire(&server.handle_frame(&frame(
        "select-1",
        SAVE_PROFILE_SELECT_TOOL,
        select_arguments("slot-2"),
    )));
    assert_eq!(selected["result"]["isError"], true, "{selected}");
    assert_eq!(
        selected["result"]["structuredContent"]["error"]["code"],
        "save_profile_unknown_after_oversized_response"
    );
}
