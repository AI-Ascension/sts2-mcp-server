// SPDX-License-Identifier: MIT

use super::*;

mod bound_sessions;
mod recovery_ownership;

#[test]
fn every_tool_constructs_a_canonical_request_and_uncertainty_is_schema_valid() -> Result<(), String>
{
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../protocol-artifact/runtime-v3-gameplay/schema.json"
    ))
    .map_err(|error| error.to_string())?;
    let validator = jsonschema::draft202012::options()
        .build(&schema)
        .map_err(|error| error.to_string())?;
    let cases = bound_sessions::tool_cases();
    for (tool, extra, kind, suffix, method, uncertain) in cases {
        let mut server = McpServer::with_catalog(
            RecordingGateway::new([Err(GatewayError::Timeout)]),
            ToolCatalog::runtime_v3_gameplay(),
        );
        let output = server.handle_frame(&call(tool, &context_arguments(extra)));
        assert_eq!(server.gateway().requests.len(), 1, "{output}");
        let request = &server.gateway().requests[0];
        assert_eq!(request.method, method);
        assert_eq!(request.path, format!("/v3/instances/instance-1/{suffix}"));
        let body = request.body.as_ref().ok_or("missing request body")?;
        let value: serde_json::Value =
            serde_json::from_str(&body.to_json()).map_err(|error| error.to_string())?;
        assert_eq!(value["kind"], kind);
        assert!(validator.is_valid(&value), "{tool}: {value}");
        if uncertain {
            let wire: serde_json::Value =
                serde_json::from_str(&output).map_err(|error| error.to_string())?;
            let projected: serde_json::Value = serde_json::from_str(
                wire["result"]["content"][0]["text"]
                    .as_str()
                    .ok_or("missing projection")?,
            )
            .map_err(|error| error.to_string())?;
            assert_eq!(projected["status"], "unknown");
            assert!(validator.is_valid(&projected), "{tool}: {projected}");
        }
    }
    Ok(())
}

fn settled_response(kind: &str, from: i64, to: i64, witness_state: &str) -> JsonValue {
    root(
        kind,
        to,
        JsonValue::string("combat-1"),
        JsonValue::string("operation-1"),
        observation(to),
        legal_actions(),
        JsonValue::Null,
        JsonValue::string("settled"),
        JsonValue::object([
            (String::from("from_generation"), JsonValue::Number(from)),
            (String::from("to_generation"), JsonValue::Number(to)),
            (String::from("state_id"), JsonValue::string(witness_state)),
            (
                String::from("effect_kind"),
                JsonValue::string("turn_end_settled"),
            ),
        ]),
        JsonValue::Null,
        if kind == "wait_response" {
            JsonValue::string("same_state_mutation")
        } else {
            JsonValue::Null
        },
    )
}

fn project_call(tool: &str, arguments: &str, body: JsonValue) -> String {
    let mut server = McpServer::with_catalog(
        RecordingGateway::new([Ok(GatewayResponse { status: 200, body })]),
        ToolCatalog::runtime_v3_gameplay(),
    );
    server.handle_frame(&call(tool, arguments))
}

#[test]
fn settled_dispatch_requires_witness_for_the_requested_generation_and_result_state() {
    let arguments = context_arguments(
        ",\"state_id\":\"combat-1\",\"operation_id\":\"operation-1\",\
         \"action\":{\"action_id\":\"end-turn\",\"action\":{\"kind\":\"end_turn\"}}",
    );
    for (from, state, accepted) in [
        (4, "combat-1", true),
        (3, "combat-1", false),
        (4, "other-state", false),
    ] {
        let output = project_call(
            DISPATCH_ACTION_TOOL,
            &arguments,
            settled_response("dispatch_action_response", from, 5, state),
        );
        assert_eq!(output.contains("\"isError\":false"), accepted, "{output}");
    }
}

#[test]
fn waiting_after_reobservation_preserves_the_existing_settlement_receipt() {
    let arguments = context_arguments(",\"operation_id\":\"operation-1\",\"wait_for_millis\":1000");
    // Request generation 4 is a refreshed observation; the operation settled from 3 to 4.
    let output = project_call(
        WAIT_FOR_TRANSITION_TOOL,
        &arguments,
        settled_response("wait_response", 3, 4, "combat-1"),
    );
    assert!(output.contains("\"isError\":false"), "{output}");
}

#[test]
fn catalog_response_must_match_the_requested_state_and_generation() {
    for (generation, state, accepted) in [
        (4, "combat-1", true),
        (5, "combat-1", false),
        (4, "other-state", false),
    ] {
        let body = root(
            "legal_actions_response",
            generation,
            JsonValue::string(state),
            JsonValue::Null,
            JsonValue::Null,
            legal_actions(),
            JsonValue::Null,
            JsonValue::Null,
            JsonValue::Null,
            JsonValue::Null,
            JsonValue::Null,
        );
        let output = project_call(
            LEGAL_ACTIONS_TOOL,
            &context_arguments(",\"state_id\":\"combat-1\""),
            body,
        );
        assert_eq!(output.contains("\"isError\":false"), accepted, "{output}");
    }
}

#[test]
fn observation_reads_can_discover_a_newer_generation() {
    let output = project_call(OBSERVE_TOOL, &context_arguments(""), state_response(5));
    assert!(output.contains("\"isError\":false"), "{output}");
}

#[test]
fn malformed_dispatch_response_preserves_unknown_outcome_without_retry() {
    let mut server = McpServer::with_catalog(
        RecordingGateway::new([Err(GatewayError::MalformedResponse)]),
        ToolCatalog::runtime_v3_gameplay(),
    );
    let arguments = context_arguments(
        ",\"state_id\":\"combat-1\",\"operation_id\":\"operation-1\",\
         \"action\":{\"action_id\":\"end-turn\",\"action\":{\"kind\":\"end_turn\"}}",
    );
    let output = server.handle_frame(&call(DISPATCH_ACTION_TOOL, &arguments));
    assert!(
        output.contains("\\\"status\\\":\\\"unknown\\\""),
        "{output}"
    );
    assert_eq!(server.gateway().requests.len(), 1);
}

#[test]
fn body_only_identifiers_use_the_protocol_bound_not_the_http_header_bound() {
    let id = "a".repeat(512);
    let extra = format!(
        ",\"state_id\":\"{id}\",\"operation_id\":\"{id}\",\"action\":{{\"action_id\":\"{id}\",\"action\":{{\"kind\":\"end_turn\"}}}}"
    );
    let mut server = McpServer::with_catalog(
        RecordingGateway::new([]),
        ToolCatalog::runtime_v3_gameplay(),
    );
    let output = server.handle_frame(&call(DISPATCH_ACTION_TOOL, &context_arguments(&extra)));
    assert_eq!(server.gateway().requests.len(), 1, "{output}");
    assert!(output.contains("unknown_after_disconnect"));
    let invalid = extra.replace(&id, &"a".repeat(513));
    let output = server.handle_frame(&call(DISPATCH_ACTION_TOOL, &context_arguments(&invalid)));
    assert!(output.contains("-32602"), "{output}");
    assert_eq!(server.gateway().requests.len(), 1);
}

#[test]
fn unknown_receipt_cannot_smuggle_unvalidated_state_payload() {
    let body = root(
        "dispatch_action_response",
        4,
        JsonValue::object([(
            String::from("raw_memory"),
            JsonValue::string("private-payload"),
        )]),
        JsonValue::string("operation-1"),
        JsonValue::Null,
        JsonValue::Null,
        JsonValue::Null,
        JsonValue::string("unknown"),
        JsonValue::Null,
        JsonValue::string("sts2.runtime/unknown"),
        JsonValue::Null,
    );
    let arguments = context_arguments(
        ",\"state_id\":\"combat-1\",\"operation_id\":\"operation-1\",\
         \"action\":{\"action_id\":\"end-turn\",\"action\":{\"kind\":\"end_turn\"}}",
    );
    let output = project_call(DISPATCH_ACTION_TOOL, &arguments, body);
    assert!(!output.contains("private-payload"), "{output}");
    assert!(
        output.contains("unknown_after_invalid_response"),
        "{output}"
    );
}

#[test]
fn configured_mcp_and_gateway_sessions_are_distinct_namespaces() {
    let mut server = McpServer::with_catalog_and_sessions(
        RecordingGateway::new([Ok(GatewayResponse {
            status: 200,
            body: state_response(4),
        })]),
        ToolCatalog::runtime_v3_gameplay(),
        "session-1",
        "mcp-session-1",
    );
    let arguments = context_arguments("").replace(
        "\"mcp_session_id\":\"session-1\"",
        "\"mcp_session_id\":\"mcp-session-1\"",
    );
    let output = server.handle_frame(&call(OBSERVE_TOOL, &arguments));
    assert!(output.contains("\"isError\":false"), "{output}");
    let request = &server.gateway().requests[0];
    assert_eq!(request.correlation.mcp_session_id, "mcp-session-1");
    assert_eq!(
        request.headers.get("x-mcp-session-id").map(String::as_str),
        Some("mcp-session-1")
    );
    assert_eq!(
        request
            .body
            .as_ref()
            .and_then(|value| match value {
                JsonValue::Object(object) => Some(object),
                _ => None,
            })
            .and_then(|object| object.get("session_id")),
        Some(&JsonValue::string("session-1"))
    );
    let wrong = server.handle_frame(&call(OBSERVE_TOOL, &context_arguments("")));
    assert!(wrong.contains("-32602"), "{wrong}");
    assert_eq!(server.gateway().requests.len(), 1);
}

#[test]
fn oversized_valid_settlement_preserves_unknown_operation_without_retry() -> Result<(), String> {
    let mut body = settled_response("dispatch_action_response", 4, 5, "combat-1");
    let JsonValue::Object(root) = &mut body else {
        return Err("missing root".into());
    };
    let actions = (0..256)
        .map(|index| {
            JsonValue::object([
                (
                    String::from("action_id"),
                    JsonValue::string(format!("action-{index}")),
                ),
                (
                    String::from("action"),
                    JsonValue::object([
                        (String::from("kind"), JsonValue::string("select_card")),
                        (String::from("card_id"), JsonValue::string("a".repeat(512))),
                    ]),
                ),
            ])
        })
        .collect();
    root.insert(String::from("legal_actions"), JsonValue::Array(actions));
    assert!(body.to_json().len() > 128 * 1024);
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../protocol-artifact/runtime-v3-gameplay/schema.json"
    ))
    .map_err(|error| error.to_string())?;
    let validator = jsonschema::draft202012::options()
        .build(&schema)
        .map_err(|error| error.to_string())?;
    let value: serde_json::Value =
        serde_json::from_str(&body.to_json()).map_err(|error| error.to_string())?;
    assert!(validator.is_valid(&value));
    let mut server = McpServer::with_catalog(
        RecordingGateway::new([Ok(GatewayResponse { status: 200, body })]),
        ToolCatalog::runtime_v3_gameplay(),
    );
    let arguments = context_arguments(
        r#", "state_id":"combat-1","operation_id":"operation-1","action":{"action_id":"end-turn","action":{"kind":"end_turn"}}"#,
    );
    let output = server.handle_frame(&call(DISPATCH_ACTION_TOOL, &arguments));
    assert!(
        output.contains("unknown_after_invalid_response"),
        "{output}"
    );
    assert!(output.contains("operation-1"), "{output}");
    assert_eq!(server.gateway().requests.len(), 1);
    Ok(())
}
