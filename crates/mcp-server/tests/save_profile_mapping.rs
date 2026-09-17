// SPDX-License-Identifier: MIT
#![allow(clippy::expect_used, clippy::panic)]

use sts2_mcp_server::{
    GatewayError, GatewayMethod, GatewayResponse, JsonValue, McpServer, SAVE_PROFILE_CONTRACT,
    SAVE_PROFILE_CREATE_DISPOSABLE_TOOL, SAVE_PROFILE_CURRENT_TOOL, SAVE_PROFILE_LIST_TOOL,
    SAVE_PROFILE_MAX_BODY_BYTES, SAVE_PROFILE_MAX_OPERATION_BYTES, SAVE_PROFILE_SELECT_TOOL,
    SAVE_PROFILE_STATUS_TOOL, ToolCatalog,
};

#[path = "support/save_profile_mapping.rs"]
#[allow(dead_code)]
mod support;
use support::*;

#[test]
fn golden_catalog_and_initialize_gate_read_and_mutation_capabilities() {
    let mut server = McpServer::with_catalog(FakeGateway::new([]), ToolCatalog::save_profile_v1());
    let list =
        wire(&server.handle_frame(r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#));
    assert_eq!(list["result"]["revision"], "save-profile-v1-mcp");
    let tools = list["result"]["tools"].as_array().expect("tool array");
    let names: Vec<_> = tools
        .iter()
        .map(|tool| tool["name"].as_str().expect("tool name"))
        .collect();
    assert_eq!(
        names,
        vec![
            SAVE_PROFILE_LIST_TOOL,
            SAVE_PROFILE_CURRENT_TOOL,
            SAVE_PROFILE_STATUS_TOOL,
            SAVE_PROFILE_SELECT_TOOL,
            SAVE_PROFILE_CREATE_DISPOSABLE_TOOL,
        ]
    );
    for tool in tools {
        assert_eq!(tool["inputSchema"]["additionalProperties"], false);
        assert_eq!(tool["annotations"]["destructiveHint"], false);
        let name = tool["name"].as_str().expect("tool name");
        assert_eq!(
            tool["annotations"]["idempotentHint"],
            name != SAVE_PROFILE_SELECT_TOOL && name != SAVE_PROFILE_CREATE_DISPOSABLE_TOOL
        );
        assert_eq!(tool["annotations"]["openWorldHint"], false);
    }
    assert_eq!(tools[0]["annotations"]["readOnlyHint"], true);
    assert_eq!(tools[2]["annotations"]["readOnlyHint"], true);
    assert_eq!(tools[3]["annotations"]["readOnlyHint"], false);
    assert_eq!(tools[4]["annotations"]["readOnlyHint"], false);

    let initialized = wire(&server.handle_frame(
        r#"{"jsonrpc":"2.0","id":"init","method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1"}}}"#,
    ));
    assert_eq!(
        initialized["result"]["capabilities"]["save_profile"]["contract"],
        SAVE_PROFILE_CONTRACT
    );
    assert_eq!(
        initialized["result"]["capabilities"]["save_profile"]["supported"],
        true
    );
    assert_eq!(
        initialized["result"]["capabilities"]["save_profile"]["read"],
        true
    );
    assert_eq!(
        initialized["result"]["capabilities"]["save_profile"]["mutate"],
        true
    );
}

#[test]
fn every_tool_maps_to_one_fixed_route_and_bounded_body() {
    let responses = [
        Ok(GatewayResponse {
            status: 200,
            body: result("list-1", "list", "settled"),
        }),
        Ok(GatewayResponse {
            status: 200,
            body: result("current-1", "current", "settled"),
        }),
        Ok(GatewayResponse {
            status: 200,
            body: result("select-1", "select", "settled"),
        }),
        Ok(GatewayResponse {
            status: 200,
            body: result("create-1", "createdisposable", "settled"),
        }),
        Ok(GatewayResponse {
            status: 200,
            body: result("select-1", "select", "settled"),
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
        ("create-1", SAVE_PROFILE_CREATE_DISPOSABLE_TOOL, context()),
        (
            "status-1",
            SAVE_PROFILE_STATUS_TOOL,
            status_arguments("select-1"),
        ),
    ];
    for (id, name, arguments) in &calls {
        let output = wire(&server.handle_frame(&frame(id, name, arguments.clone())));
        assert_eq!(output["result"]["isError"], false, "{output}");
    }
    let requests = &server.gateway().requests;
    assert_eq!(requests.len(), calls.len());
    assert_eq!(requests[0].method, GatewayMethod::Get);
    assert_eq!(requests[0].path, "/v1/instances/instance-1/save-profiles");
    assert!(requests[0].body.is_none());
    assert_eq!(requests[1].method, GatewayMethod::Get);
    assert_eq!(
        requests[1].path,
        "/v1/instances/instance-1/save-profile/current"
    );
    assert!(requests[1].body.is_none());
    assert_eq!(requests[2].method, GatewayMethod::Post);
    assert_eq!(
        requests[2].path,
        "/v1/instances/instance-1/save-profile/select"
    );
    assert_eq!(
        requests[2].body,
        Some(JsonValue::object([
            (String::from("baseline"), baseline("before")),
            (String::from("profile_id"), JsonValue::string("slot-2")),
        ]))
    );
    assert_eq!(requests[3].method, GatewayMethod::Post);
    assert_eq!(
        requests[3].path,
        "/v1/instances/instance-1/save-profile/create-disposable"
    );
    assert_eq!(requests[3].body, Some(JsonValue::object([])));
    assert_eq!(requests[4].method, GatewayMethod::Get);
    assert_eq!(
        requests[4].path,
        "/v1/instances/instance-1/save-profile/operations/select-1"
    );
    assert!(requests[4].body.is_none());
    for request in requests {
        assert_eq!(
            request.headers.get("x-mcp-session-id").map(String::as_str),
            Some("mcp-session-1")
        );
        assert_eq!(
            request.headers.get("x-mcp-request-id").map(String::as_str),
            Some(request.correlation.mcp_request_id.stable_text().as_str())
        );
        assert_eq!(
            request.headers.get("x-sts2-session-id").map(String::as_str),
            Some("gateway-session-1")
        );
        assert_eq!(
            request
                .headers
                .get("x-sts2-lease-epoch")
                .map(String::as_str),
            Some("7")
        );
        assert!(
            request
                .body
                .as_ref()
                .is_none_or(|body| body.to_json().len() <= SAVE_PROFILE_MAX_BODY_BYTES)
        );
    }
    assert_eq!(SAVE_PROFILE_MAX_OPERATION_BYTES, 128);
}

#[test]
fn malformed_oversized_and_foreign_shape_inputs_fail_before_forwarding() {
    let mut server = McpServer::with_catalog(FakeGateway::new([]), ToolCatalog::save_profile_v1());
    let mut malformed = select_arguments("slot-2");
    if let JsonValue::Object(object) = &mut malformed {
        object.insert(
            String::from("baseline"),
            JsonValue::object([
                (String::from("identity"), JsonValue::string("before")),
                (String::from("digest"), JsonValue::string("short")),
            ]),
        );
    }
    assert_eq!(
        wire(&server.handle_frame(&frame("bad-baseline", SAVE_PROFILE_SELECT_TOOL, malformed)))["error"]
            ["code"],
        -32602
    );
    assert_eq!(
        wire(&server.handle_frame(&frame(
            "oversized-profile",
            SAVE_PROFILE_SELECT_TOOL,
            select_arguments(&"x".repeat(129))
        )))["error"]["code"],
        -32602
    );
    let mut unknown_field = context();
    if let JsonValue::Object(object) = &mut unknown_field {
        object.insert(String::from("schema_revision"), JsonValue::string("future"));
    }
    assert_eq!(
        wire(&server.handle_frame(&frame(
            "unknown-version",
            SAVE_PROFILE_LIST_TOOL,
            unknown_field
        )))["error"]["code"],
        -32602
    );
    let mut foreign = context();
    if let JsonValue::Object(object) = &mut foreign {
        object.insert(
            String::from("instance_id"),
            JsonValue::string("foreign/instance"),
        );
    }
    assert_eq!(
        wire(&server.handle_frame(&frame("foreign-target", SAVE_PROFILE_CURRENT_TOOL, foreign)))["error"]
            ["code"],
        -32602
    );
    assert!(server.gateway().requests.is_empty());
}

#[test]
fn response_contract_revision_and_stale_baseline_fail_closed_without_leaking_payload() {
    let mut wrong_contract = result("current-1", "current", "settled");
    if let JsonValue::Object(object) = &mut wrong_contract {
        object.insert(
            String::from("contract"),
            JsonValue::string("gateway-save-profile-v99"),
        );
    }
    let stale = JsonValue::object([(
        String::from("error_code"),
        JsonValue::string("save_profile_fence_rejected"),
    )]);
    let mut server = McpServer::with_catalog(
        FakeGateway::new([
            Ok(GatewayResponse {
                status: 200,
                body: wrong_contract,
            }),
            Ok(GatewayResponse {
                status: 409,
                body: stale,
            }),
        ]),
        ToolCatalog::save_profile_v1(),
    );
    let malformed =
        wire(&server.handle_frame(&frame("current-1", SAVE_PROFILE_CURRENT_TOOL, context())));
    assert_eq!(malformed["result"]["isError"], true);
    assert_eq!(
        malformed["result"]["structuredContent"]["error"]["code"],
        "save_profile_malformed_response"
    );
    let stale_output = wire(&server.handle_frame(&frame(
        "select-1",
        SAVE_PROFILE_SELECT_TOOL,
        select_arguments("slot-2"),
    )));
    assert_eq!(stale_output["result"]["isError"], true);
    assert_eq!(
        stale_output["result"]["structuredContent"]["error"]["code"],
        "save_profile_fence_rejected"
    );
    assert_eq!(
        stale_output["result"]["structuredContent"]["error"]["category"],
        "stale"
    );
}

#[test]
fn disposable_provisioning_outcome_preserves_created_descriptor_without_fabricating_settlement() {
    let created = JsonValue::object([
        (
            String::from("contract"),
            JsonValue::string(SAVE_PROFILE_CONTRACT),
        ),
        (String::from("operation_id"), JsonValue::string("create-1")),
        (String::from("status"), JsonValue::string("created")),
        (String::from("user_data"), user_data("create-1")),
        (String::from("guidance"), JsonValue::Null),
    ]);
    let mut server = McpServer::with_catalog(
        FakeGateway::new([Ok(GatewayResponse {
            status: 200,
            body: created,
        })]),
        ToolCatalog::save_profile_v1(),
    );
    let output = wire(&server.handle_frame(&frame(
        "create-1",
        SAVE_PROFILE_CREATE_DISPOSABLE_TOOL,
        context(),
    )));
    assert_eq!(output["result"]["isError"], false, "{output}");
    assert_eq!(server.gateway().requests.len(), 1);
    assert_eq!(
        server.gateway().requests[0].body,
        Some(JsonValue::object([]))
    );
}

#[test]
fn read_only_capability_does_not_advertise_or_dispatch_mutations() {
    let mut server = McpServer::with_catalog(
        FakeGateway::new([]),
        ToolCatalog::save_profile_v1_read_only(),
    );
    let list =
        wire(&server.handle_frame(r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#));
    assert_eq!(list["result"]["tools"].as_array().expect("tools").len(), 3);
    assert_eq!(
        list["result"]["tools"]
            .as_array()
            .expect("tools")
            .iter()
            .filter(|tool| {
                tool["name"] == SAVE_PROFILE_SELECT_TOOL
                    || tool["name"] == SAVE_PROFILE_CREATE_DISPOSABLE_TOOL
            })
            .count(),
        0
    );
    let output = wire(&server.handle_frame(&frame(
        "select-1",
        SAVE_PROFILE_SELECT_TOOL,
        select_arguments("slot-2"),
    )));
    assert_eq!(output["error"]["code"], -32601);
    assert!(server.gateway().requests.is_empty());

    let mut unsupported = McpServer::with_catalog(
        FakeGateway::new([]),
        ToolCatalog::save_profile_v1_unsupported(),
    );
    let capabilities = wire(&unsupported.handle_frame(
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1"}}}"#,
    ));
    assert_eq!(
        capabilities["result"]["capabilities"]["save_profile"]["supported"],
        false
    );
    assert!(capabilities["result"]["capabilities"]["tools"].is_null());
    assert_eq!(
        wire(&unsupported.handle_frame(&frame("list-1", SAVE_PROFILE_LIST_TOOL, context())))["error"]
            ["code"],
        -32601
    );
}

#[test]
fn lost_mutation_response_preserves_operation_identity_and_reconciles_once() {
    let mut server = McpServer::with_catalog(
        FakeGateway::new([
            Err(GatewayError::Timeout),
            Ok(GatewayResponse {
                status: 200,
                body: result("select-unknown", "select", "settled"),
            }),
        ]),
        ToolCatalog::save_profile_v1(),
    );
    let unknown = wire(&server.handle_frame(&frame(
        "select-unknown",
        SAVE_PROFILE_SELECT_TOOL,
        select_arguments("slot-2"),
    )));
    assert_eq!(unknown["result"]["isError"], true);
    assert_eq!(
        unknown["result"]["structuredContent"]["error"]["code"],
        "save_profile_unknown_after_timeout"
    );
    assert!(unknown.to_string().contains("select-unknown"));
    let reconciled = wire(&server.handle_frame(&frame(
        "status-1",
        SAVE_PROFILE_STATUS_TOOL,
        status_arguments("select-unknown"),
    )));
    assert_eq!(reconciled["result"]["isError"], false, "{reconciled}");
    assert_eq!(server.gateway().requests.len(), 2);
    assert_eq!(
        server.gateway().requests[0].path,
        "/v1/instances/instance-1/save-profile/select"
    );
    assert_eq!(
        server.gateway().requests[1].path,
        "/v1/instances/instance-1/save-profile/operations/select-unknown"
    );
}

#[test]
fn malformed_mutation_response_becomes_unknown_with_reconciliation_metadata() {
    let mut malformed = result("select-malformed", "select", "settled");
    if let JsonValue::Object(object) = &mut malformed {
        object.remove("baseline");
    }
    let mut server = McpServer::with_catalog(
        FakeGateway::new([Ok(GatewayResponse {
            status: 200,
            body: malformed,
        })]),
        ToolCatalog::save_profile_v1(),
    );

    let output = wire(&server.handle_frame(&frame(
        "select-malformed",
        SAVE_PROFILE_SELECT_TOOL,
        select_arguments("slot-2"),
    )));

    assert_eq!(output["result"]["isError"], true);
    assert_eq!(
        output["result"]["structuredContent"]["error"]["code"],
        "save_profile_unknown_after_malformed_response"
    );
    let body: serde_json::Value = serde_json::from_str(
        output["result"]["content"][0]["text"]
            .as_str()
            .expect("unknown result text"),
    )
    .expect("unknown result body");
    assert_eq!(body["status"], "unknown");
    assert_eq!(body["operation_id"], "select-malformed");
    assert_eq!(body["guidance"]["code"], "save_profile_reconcile_required");
    assert_eq!(server.gateway().requests.len(), 1);
}

#[test]
fn oversized_mutation_projection_becomes_unknown_with_reconciliation_metadata() {
    let mut oversized = result("select-oversized", "select", "rejected");
    if let JsonValue::Object(object) = &mut oversized {
        object.insert(
            String::from("downstream"),
            JsonValue::string("x".repeat(SAVE_PROFILE_MAX_BODY_BYTES)),
        );
    }
    let mut server = McpServer::with_catalog(
        FakeGateway::new([Ok(GatewayResponse {
            status: 200,
            body: oversized,
        })]),
        ToolCatalog::save_profile_v1(),
    );

    let output = wire(&server.handle_frame(&frame(
        "select-oversized",
        SAVE_PROFILE_SELECT_TOOL,
        select_arguments("slot-2"),
    )));

    assert_eq!(output["result"]["isError"], true);
    assert_eq!(
        output["result"]["structuredContent"]["error"]["code"],
        "save_profile_unknown_after_oversized_response"
    );
    let body: serde_json::Value = serde_json::from_str(
        output["result"]["content"][0]["text"]
            .as_str()
            .expect("unknown result text"),
    )
    .expect("unknown result body");
    assert_eq!(body["status"], "unknown");
    assert_eq!(body["operation_id"], "select-oversized");
    assert_eq!(body["guidance"]["code"], "save_profile_reconcile_required");
    assert_eq!(server.gateway().requests.len(), 1);
}

#[test]
fn disposable_creation_with_different_request_ids_is_not_idempotent() {
    let mut server = McpServer::with_catalog(
        FakeGateway::new([
            Ok(GatewayResponse {
                status: 200,
                body: result("create-1", "createdisposable", "settled"),
            }),
            Ok(GatewayResponse {
                status: 200,
                body: result("create-2", "createdisposable", "settled"),
            }),
        ]),
        ToolCatalog::save_profile_v1(),
    );
    let listed =
        wire(&server.handle_frame(r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#));
    let create = listed["result"]["tools"]
        .as_array()
        .expect("tool array")
        .iter()
        .find(|tool| tool["name"] == SAVE_PROFILE_CREATE_DISPOSABLE_TOOL)
        .expect("create-disposable descriptor");
    assert_eq!(create["annotations"]["idempotentHint"], false);

    for id in ["create-1", "create-2"] {
        let output =
            wire(&server.handle_frame(&frame(id, SAVE_PROFILE_CREATE_DISPOSABLE_TOOL, context())));
        assert_eq!(output["result"]["isError"], false, "{output}");
    }

    let requests = &server.gateway().requests;
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].path, requests[1].path);
    assert_eq!(requests[0].body, requests[1].body);
    assert_ne!(
        requests[0].correlation.mcp_request_id,
        requests[1].correlation.mcp_request_id
    );
}
