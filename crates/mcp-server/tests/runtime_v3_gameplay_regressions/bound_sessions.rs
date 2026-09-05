// SPDX-License-Identifier: MIT

use super::*;

type ToolCase = (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    GatewayMethod,
    bool,
);

pub(super) fn tool_cases() -> [ToolCase; 9] {
    [
        (
            OBSERVE_TOOL,
            "",
            "state_request",
            "state",
            GatewayMethod::Get,
            false,
        ),
        (
            LEGAL_ACTIONS_TOOL,
            ",\"state_id\":\"combat-1\"",
            "legal_actions_request",
            "legal-actions",
            GatewayMethod::Get,
            false,
        ),
        (
            DISPATCH_ACTION_TOOL,
            ",\"state_id\":\"combat-1\",\"operation_id\":\"operation-1\",\"action\":{\"action_id\":\"end-turn\",\"action\":{\"kind\":\"end_turn\"}}",
            "dispatch_action_request",
            "action",
            GatewayMethod::Post,
            true,
        ),
        (
            WAIT_FOR_TRANSITION_TOOL,
            ",\"operation_id\":\"operation-1\",\"wait_for_millis\":120000",
            "wait_request",
            "wait",
            GatewayMethod::Post,
            true,
        ),
        (
            REOBSERVE_TOOL,
            "",
            "reobserve_request",
            "reobserve",
            GatewayMethod::Get,
            false,
        ),
        (
            RECOVER_TOOL,
            ",\"recovery_kind\":\"reobserve\"",
            "recover_request",
            "recover",
            GatewayMethod::Post,
            true,
        ),
        (
            RECOVER_TOOL,
            ",\"recovery_kind\":\"reconcile\",\"operation_id\":\"operation-1\"",
            "recover_request",
            "recover",
            GatewayMethod::Post,
            true,
        ),
        (
            RECOVER_TOOL,
            ",\"recovery_kind\":\"release_lease\"",
            "recover_request",
            "recover",
            GatewayMethod::Post,
            true,
        ),
        (
            RECOVER_TOOL,
            ",\"recovery_kind\":\"stop_episode\"",
            "recover_request",
            "recover",
            GatewayMethod::Post,
            true,
        ),
    ]
}

fn bound_arguments(extra: &str) -> String {
    context_arguments(extra).replace(
        "\"mcp_session_id\":\"session-1\"",
        "\"mcp_session_id\":\"mcp-session-1\"",
    )
}

fn successful_response(kind: &str, extra: &str) -> Result<JsonValue, String> {
    let response_kind = kind.replace("_request", "_response");
    let mut body = match kind {
        "state_request" | "reobserve_request" => state_response(5),
        "legal_actions_request" => root(
            &response_kind,
            4,
            JsonValue::string("combat-1"),
            JsonValue::Null,
            JsonValue::Null,
            legal_actions(),
            JsonValue::Null,
            JsonValue::Null,
            JsonValue::Null,
            JsonValue::Null,
            JsonValue::Null,
        ),
        _ => settled_response(&response_kind, 4, 5, "combat-1"),
    };
    let JsonValue::Object(fields) = &mut body else {
        return Err("missing root".into());
    };
    fields.insert(String::from("kind"), JsonValue::string(response_kind));
    if kind == "recover_request" && !extra.contains("operation_id") {
        fields.insert(String::from("operation_id"), JsonValue::string("request-1"));
    }
    Ok(body)
}

#[test]
fn all_semantic_tools_preserve_separate_bound_sessions() -> Result<(), String> {
    for (tool, extra, kind, _, _, _) in tool_cases() {
        let gateway = RecordingGateway::new([Ok(GatewayResponse {
            status: 200,
            body: successful_response(kind, extra)?,
        })]);
        let mut server = McpServer::with_catalog_and_sessions(
            gateway,
            ToolCatalog::runtime_v3_gameplay(),
            "session-1",
            "mcp-session-1",
        );
        let output = server.handle_frame(&call(tool, &bound_arguments(extra)));
        assert!(output.contains("\"isError\":false"), "{tool}: {output}");
        assert_eq!(server.gateway().requests.len(), 1);
        let request = &server.gateway().requests[0];
        assert_eq!(
            request.headers.get("x-mcp-session-id").map(String::as_str),
            Some("mcp-session-1")
        );
        assert_eq!(request.correlation.mcp_session_id, "mcp-session-1");
        assert_eq!(
            request.correlation.mcp_request_id.stable_text(),
            "request-1"
        );
        let body = request.body.as_ref().ok_or("missing request body")?;
        let value: serde_json::Value =
            serde_json::from_str(&body.to_json()).map_err(|error| error.to_string())?;
        assert_eq!(value["session_id"], "session-1");
        assert_eq!(value["correlation_id"], "request-1");
        assert_eq!(value["kind"], kind);
    }
    Ok(())
}

#[test]
fn all_semantic_tools_reject_foreign_bound_mcp_sessions_before_forwarding() {
    for (tool, extra, _, _, _, _) in tool_cases() {
        let mut server = McpServer::with_catalog_and_sessions(
            RecordingGateway::new([]),
            ToolCatalog::runtime_v3_gameplay(),
            "session-1",
            "mcp-session-1",
        );
        let arguments = bound_arguments(extra).replace("mcp-session-1", "foreign-session");
        let output = server.handle_frame(&call(tool, &arguments));
        assert!(output.contains("-32602"), "{tool}: {output}");
        assert!(server.gateway().requests.is_empty(), "{tool}");
    }
}

#[test]
fn all_semantic_tools_preserve_typed_scope_denials_without_unknown() -> Result<(), String> {
    for (tool, extra, _, _, _, _) in tool_cases() {
        let mut server = McpServer::with_catalog_and_sessions(
            RecordingGateway::new([Err(GatewayError::Forbidden)]),
            ToolCatalog::runtime_v3_gameplay(),
            "session-1",
            "mcp-session-1",
        );
        let output = server.handle_frame(&call(tool, &bound_arguments(extra)));
        assert_eq!(server.gateway().requests.len(), 1, "{tool}");
        let wire: serde_json::Value =
            serde_json::from_str(&output).map_err(|error| error.to_string())?;
        assert_eq!(wire["result"]["isError"], true);
        assert_eq!(
            wire["result"]["content"][0]["text"],
            "gateway error -32007: gateway scope authorization failed"
        );
        assert!(!output.contains("unknown"), "{tool}: {output}");
    }
    Ok(())
}
