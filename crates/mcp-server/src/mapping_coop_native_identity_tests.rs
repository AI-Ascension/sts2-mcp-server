// SPDX-License-Identifier: MIT

use super::*;

fn tool_property(
    catalog: &ToolCatalog,
    tool_name: &str,
    property_name: &str,
) -> Result<JsonValue, String> {
    let listed = catalog.to_json();
    let tools = listed
        .as_object()
        .and_then(|value| value.get("tools"))
        .and_then(JsonValue::as_array)
        .ok_or_else(|| String::from("native catalog has no tools"))?;
    let tool = tools
        .iter()
        .find(|tool| {
            tool.as_object()
                .and_then(|value| value.get("name"))
                .and_then(JsonValue::as_string)
                == Some(tool_name)
        })
        .ok_or_else(|| String::from("native catalog tool is missing"))?;
    Ok(nested_property(
        tool.as_object()
            .and_then(|value| value.get("inputSchema"))
            .ok_or_else(|| String::from("native catalog input schema is missing"))?,
        property_name,
    )?
    .clone())
}

fn nested_property<'a>(value: &'a JsonValue, property_name: &str) -> Result<&'a JsonValue, String> {
    value
        .as_object()
        .and_then(|value| value.get("properties"))
        .and_then(JsonValue::as_object)
        .and_then(|value| value.get(property_name))
        .ok_or_else(|| String::from("native schema property is missing"))
}

fn schema_maximum(value: &JsonValue) -> Option<i64> {
    let direct = value.as_object().and_then(|value| value.get("maxLength"));
    let nullable = value
        .as_object()
        .and_then(|value| value.get("anyOf"))
        .and_then(JsonValue::as_array)
        .and_then(|values| values.first())
        .and_then(JsonValue::as_object)
        .and_then(|value| value.get("maxLength"));
    match direct.or(nullable) {
        Some(JsonValue::Number(value)) => Some(*value),
        _ => None,
    }
}

#[test]
fn catalog_matches_native_body_and_configured_identity_bounds() -> Result<(), String> {
    let catalog = ToolCatalog::coop_native();
    for (tool, property) in [
        (COOP_NATIVE_ACTION_TOOL, "operation_id"),
        (COOP_NATIVE_ACTION_TOOL, "actor_peer"),
        (COOP_NATIVE_VOTE_TOOL, "operation_id"),
        (COOP_NATIVE_VOTE_TOOL, "actor_peer"),
        (COOP_NATIVE_REJOIN_TOOL, "operation_id"),
        (COOP_NATIVE_REJOIN_TOOL, "actor_peer"),
        (COOP_NATIVE_RECOVER_TOOL, "operation_id"),
    ] {
        assert_eq!(
            schema_maximum(&tool_property(&catalog, tool, property)?),
            Some(512)
        );
    }
    let action = tool_property(&catalog, COOP_NATIVE_ACTION_TOOL, "action")?;
    assert_eq!(
        schema_maximum(nested_property(&action, "action_id")?),
        Some(512),
        "{}",
        action.to_json()
    );
    assert_eq!(
        schema_maximum(nested_property(&action, "target_peer")?),
        Some(512)
    );
    let vote = tool_property(&catalog, COOP_NATIVE_VOTE_TOOL, "vote")?;
    assert_eq!(
        schema_maximum(nested_property(&vote, "proposal_id")?),
        Some(512)
    );
    assert_eq!(
        schema_maximum(nested_property(&vote, "voter_peer")?),
        Some(512)
    );
    assert_eq!(schema_maximum(nested_property(&vote, "choice")?), Some(512));
    let envelope = tool_property(&catalog, COOP_NATIVE_EFFECT_TOOL, "envelope")?;
    assert_eq!(
        schema_maximum(nested_property(&envelope, "operation_id")?),
        Some(512)
    );
    assert_eq!(
        schema_maximum(&tool_property(
            &catalog,
            COOP_NATIVE_ACTION_TOOL,
            "instance_id"
        )?),
        Some(128)
    );
    Ok(())
}

#[test]
fn preserves_protocol_sized_body_identities_without_widening_header_or_path_identities()
-> Result<(), String> {
    let operation_id = format!("op/{}", "o".repeat(509));
    let peer = format!("peer:{}", "p".repeat(507));
    let action_id = format!("action/{}", "a".repeat(505));
    let mut server = server(GatewayResponse {
        status: 500,
        body: JsonValue::Null,
    });
    let arguments = format!(
        r#"{{{},"operation_id":"{operation_id}","actor_peer":"{peer}","expected_host_generation":1,"action":{{"kind":"end_turn","action_id":"{action_id}","target_peer":"{peer}"}}}}"#,
        common()
    );
    let output = server.handle_frame(&frame(
        "corr-body-identities",
        COOP_NATIVE_ACTION_TOOL,
        &arguments,
    ));
    assert!(output.contains(r#""isError":true"#), "{output}");
    let request = server
        .gateway()
        .requests
        .first()
        .ok_or_else(|| String::from("body identities did not reach the gateway"))?;
    assert_eq!(request.path, "/v1/instances/instance-1/coop/native/action");
    assert_eq!(
        request.headers.get("x-sts2-instance-id").map(String::len),
        Some(10)
    );
    assert!(
        request
            .headers
            .values()
            .all(|value| { value != &operation_id && value != &peer && value != &action_id }),
        "closed envelope identities must not be copied to headers: {:?}",
        request.headers
    );
    let body = request
        .body
        .as_ref()
        .and_then(JsonValue::as_object)
        .ok_or_else(|| String::from("native action did not carry an envelope"))?;
    assert_eq!(
        body.get("operation_id").and_then(JsonValue::as_string),
        Some(operation_id.as_str())
    );
    assert_eq!(
        body.get("actor_peer").and_then(JsonValue::as_string),
        Some(peer.as_str())
    );
    let action = body
        .get("action")
        .and_then(JsonValue::as_object)
        .ok_or_else(|| String::from("native action envelope is missing its action"))?;
    assert_eq!(
        action.get("action_id").and_then(JsonValue::as_string),
        Some(action_id.as_str())
    );
    assert_eq!(
        action.get("target_peer").and_then(JsonValue::as_string),
        Some(peer.as_str())
    );
    Ok(())
}

#[test]
fn rejoin_reconciles_a_protocol_sized_original_operation_identity() -> Result<(), String> {
    let operation_id = format!("op/..{}", "o".repeat(507));
    let peer = format!("peer:{}", "p".repeat(507));
    let mut server = server(response(
        "recovery",
        "corr-long-rejoin",
        Some(&operation_id),
    )?);
    let arguments = format!(
        r#"{{{},"operation_id":"{operation_id}","actor_peer":"{peer}","expected_host_generation":1,"recovery":{{"kind":"rejoin","rejoin_epoch":3}}}}"#,
        common()
    );
    let output = server.handle_frame(&frame(
        "corr-long-rejoin",
        COOP_NATIVE_REJOIN_TOOL,
        &arguments,
    ));
    assert!(output.contains(r#""isError":false"#), "{output}");
    assert!(output.contains(&operation_id), "{output}");
    let request = server
        .gateway()
        .requests
        .first()
        .ok_or_else(|| String::from("rejoin did not reach the gateway"))?;
    assert_eq!(request.path, "/v1/instances/instance-1/coop/native/rejoin");
    let body = request
        .body
        .as_ref()
        .and_then(JsonValue::as_object)
        .ok_or_else(|| String::from("rejoin did not carry an envelope"))?;
    assert_eq!(
        body.get("operation_id").and_then(JsonValue::as_string),
        Some(operation_id.as_str())
    );
    assert_eq!(
        body.get("actor_peer").and_then(JsonValue::as_string),
        Some(peer.as_str())
    );
    Ok(())
}

#[test]
fn recover_retains_a_protocol_sized_original_operation_identity() -> Result<(), String> {
    let operation_id = format!("op/..{}", "o".repeat(507));
    let mut server = server(response(
        "recovery",
        "corr-long-recover",
        Some(&operation_id),
    )?);
    let arguments = format!(
        r#"{{{},"operation_id":"{operation_id}","recovery":{{"kind":"reconcile","rejoin_epoch":3}}}}"#,
        common()
    );
    let output = server.handle_frame(&frame(
        "corr-long-recover",
        COOP_NATIVE_RECOVER_TOOL,
        &arguments,
    ));
    assert!(output.contains(r#""isError":false"#), "{output}");
    assert!(output.contains(&operation_id), "{output}");
    let request = server
        .gateway()
        .requests
        .first()
        .ok_or_else(|| String::from("recovery did not reach the gateway"))?;
    assert_eq!(request.path, "/v1/instances/instance-1/coop/native/recover");
    assert!(
        request.headers.values().all(|value| value != &operation_id),
        "original operation identity must not be copied to headers: {:?}",
        request.headers
    );
    assert_eq!(
        request
            .body
            .as_ref()
            .and_then(JsonValue::as_object)
            .and_then(|body| body.get("operation_id"))
            .and_then(JsonValue::as_string),
        Some(operation_id.as_str())
    );
    Ok(())
}

#[test]
fn rejects_129_byte_configured_path_identity_before_gateway_access() -> Result<(), String> {
    let mut server = server(response("observation", "corr-oversized-instance", None)?);
    let oversized_instance = format!("i{}", "a".repeat(128));
    let arguments = format!(
        r#"{{"instance_id":"{oversized_instance}","mcp_session_id":"mcp-1","lease_id":"lease-1","lease_epoch":1}}"#
    );
    let output = server.handle_frame(&frame(
        "corr-oversized-instance",
        COOP_NATIVE_OBSERVATION_TOOL,
        &arguments,
    ));
    assert!(output.contains("-32602"), "{output}");
    assert!(server.gateway().requests.is_empty());
    Ok(())
}
