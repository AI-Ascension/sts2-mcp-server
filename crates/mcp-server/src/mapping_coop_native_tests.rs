// SPDX-License-Identifier: MIT

use super::*;
use crate::gateway::GatewayMethod;
use crate::{
    COOP_NATIVE_SCHEMA_DIGEST, GatewayAdapter, GatewayError, GatewayRequest, GatewayResponse,
    McpServer, ToolCatalog, parse_json,
};

#[path = "mapping_coop_native_edge_tests.rs"]
mod edge_tests;
#[path = "mapping_coop_native_identity_tests.rs"]
mod identity_tests;
#[path = "mapping_coop_native_relation_tests.rs"]
mod relation_tests;

#[derive(Clone)]
struct RecordingGateway {
    response: GatewayResponse,
    requests: Vec<GatewayRequest>,
}

impl GatewayAdapter for RecordingGateway {
    fn forward(&mut self, request: GatewayRequest) -> Result<GatewayResponse, GatewayError> {
        self.requests.push(request);
        Ok(self.response.clone())
    }
}

fn response(
    fixture: &str,
    correlation: &str,
    operation: Option<&str>,
) -> Result<GatewayResponse, String> {
    let fixture_wire = match fixture {
        "observation" => include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../protocol-artifact/coop-native-v1/golden/observation-response.json"
        )),
        "effect" => include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../protocol-artifact/coop-native-v1/golden/local-action-settled-response.json"
        )),
        "recovery" => include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../protocol-artifact/coop-native-v1/golden/rejoin-recovered-response.json"
        )),
        "catalog" => include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../protocol-artifact/coop-native-v1/golden/legal-catalog-response.json"
        )),
        _ => return Err(String::from("unknown fixture")),
    };
    let mut wire = fixture_wire
        .replace("corr:native:observation", correlation)
        .replace("corr:native:local-action-settled", correlation)
        .replace("corr:native:rejoin:recovered", correlation)
        .replace("corr:native:legal-catalog", correlation)
        .replace("instance:native-test", "instance-1")
        .replace("session:native-test", "session-1")
        .replace("lease:native-test", "lease-1")
        .replace("\"lease_epoch\":7", "\"lease_epoch\":1")
        .replace("op:native:action:settled", operation.unwrap_or("op-1"))
        .replace("op:native:rejoin", operation.unwrap_or("op-1"));
    if fixture == "effect" {
        wire = wire.replace(
            "effect:op:native:action:settled",
            &format!("effect:{}", operation.unwrap_or("op-1")),
        );
    }
    Ok(GatewayResponse {
        status: 200,
        body: parse_json(&wire)?,
    })
}

fn server(response: GatewayResponse) -> McpServer<RecordingGateway> {
    McpServer::with_catalog_and_sessions(
        RecordingGateway {
            response,
            requests: Vec::new(),
        },
        ToolCatalog::coop_native(),
        "session-1",
        "mcp-1",
    )
}

fn frame(id: &str, name: &str, arguments: &str) -> String {
    format!(
        r##"{{"jsonrpc":"2.0","id":"{id}","method":"tools/call","params":{{"name":"{name}","arguments":{arguments}}}}}"##
    )
}

fn common() -> &'static str {
    r#""instance_id":"instance-1","mcp_session_id":"mcp-1","lease_id":"lease-1","lease_epoch":1"#
}

#[test]
fn catalog_has_seven_native_tools_and_component_frame_bound() -> Result<(), String> {
    let catalog = ToolCatalog::coop_native();
    let names = [
        COOP_NATIVE_OBSERVATION_TOOL,
        COOP_NATIVE_ACTION_TOOL,
        COOP_NATIVE_VOTE_TOOL,
        COOP_NATIVE_REJOIN_TOOL,
        COOP_NATIVE_EFFECT_TOOL,
        COOP_NATIVE_RECOVER_TOOL,
        COOP_NATIVE_LEGAL_CATALOG_TOOL,
    ];
    let listed = catalog.to_json();
    let tools = listed
        .as_object()
        .and_then(|object| object.get("tools"))
        .and_then(JsonValue::as_array)
        .ok_or_else(|| String::from("catalog has no tools array"))?;
    assert_eq!(tools.len(), names.len());
    for name in names {
        assert!(tools.iter().any(|tool| {
            tool.as_object()
                .and_then(|object| object.get("name"))
                .and_then(JsonValue::as_string)
                == Some(name)
        }));
    }
    assert_eq!(catalog.max_frame_bytes(), 256 * 1024);
    Ok(())
}

#[test]
fn maps_legal_catalog_to_generation_bound_read_route() -> Result<(), String> {
    let response = response("catalog", "corr-catalog", None)?.body;
    let mut server = server(GatewayResponse {
        status: 200,
        body: response,
    });
    let arguments = format!(
        r#"{{{},"actor_peer":"peer:host1","expected_host_generation":1}}"#,
        common()
    );
    let output = server.handle_frame(&frame(
        "corr-catalog",
        COOP_NATIVE_LEGAL_CATALOG_TOOL,
        &arguments,
    ));
    assert!(output.contains(r#""isError":false"#), "{output}");
    let request = server
        .gateway()
        .requests
        .first()
        .ok_or_else(|| String::from("legal catalog did not reach gateway"))?;
    assert_eq!(request.method, GatewayMethod::Post);
    assert_eq!(
        request.path,
        "/v1/instances/instance-1/coop/native/legal-catalog"
    );
    let body = request
        .body
        .as_ref()
        .ok_or_else(|| String::from("legal catalog body is missing"))?;
    assert_eq!(
        body.as_object()
            .and_then(|object| object.get("kind"))
            .and_then(JsonValue::as_string),
        Some("legal_catalog_request")
    );
    assert_eq!(
        body.as_object()
            .and_then(|object| object.get("actor_peer"))
            .and_then(JsonValue::as_string),
        Some("peer:host1")
    );
    assert_eq!(
        body.as_object().and_then(|object| object.get("catalog")),
        Some(&JsonValue::Null)
    );
    assert_eq!(
        body.as_object().and_then(|object| object.get("receipt")),
        Some(&JsonValue::Null)
    );
    assert_eq!(
        request
            .headers
            .get("x-sts2-host-generation")
            .map(String::as_str),
        Some("1")
    );
    Ok(())
}

#[test]
fn maps_observation_action_vote_rejoin_and_recovery_to_fixed_routes() -> Result<(), String> {
    let cases = [
        (
            "corr-observation",
            COOP_NATIVE_OBSERVATION_TOOL,
            format!(r#"{{{}}}"#, common()),
            "observation",
            GatewayMethod::Get,
            "/v1/instances/instance-1/coop/native/observation",
            None,
        ),
        (
            "corr-action",
            COOP_NATIVE_ACTION_TOOL,
            format!(
                r#"{{{},"operation_id":"op-1","actor_peer":"peer:host1","expected_host_generation":1,"action":{{"kind":"end_turn","action_id":"turn:1","target_peer":null}}}}"#,
                common()
            ),
            "effect",
            GatewayMethod::Post,
            "/v1/instances/instance-1/coop/native/action",
            Some("op-1"),
        ),
        (
            "corr-vote",
            COOP_NATIVE_VOTE_TOOL,
            format!(
                r#"{{{},"operation_id":"op-1","actor_peer":"peer:host1","expected_host_generation":1,"vote":{{"proposal_id":"event:campfire","voter_peer":"peer:host1","choice":"index:1"}}}}"#,
                common()
            ),
            "effect",
            GatewayMethod::Post,
            "/v1/instances/instance-1/coop/native/vote",
            Some("op-1"),
        ),
        (
            "corr-rejoin",
            COOP_NATIVE_REJOIN_TOOL,
            format!(
                r#"{{{},"operation_id":"op-1","actor_peer":"peer:client1","expected_host_generation":1,"recovery":{{"kind":"rejoin","rejoin_epoch":3}}}}"#,
                common()
            ),
            "recovery",
            GatewayMethod::Post,
            "/v1/instances/instance-1/coop/native/rejoin",
            Some("op-1"),
        ),
        (
            "corr-recover",
            COOP_NATIVE_RECOVER_TOOL,
            format!(
                r#"{{{},"operation_id":"op-1","recovery":{{"kind":"reconcile","rejoin_epoch":3}}}}"#,
                common()
            ),
            "recovery",
            GatewayMethod::Post,
            "/v1/instances/instance-1/coop/native/recover",
            Some("op-1"),
        ),
    ];
    for (id, name, arguments, fixture, method, path, operation) in cases {
        let gateway_response = response(fixture, id, operation)?;
        let mut server = server(gateway_response);
        let output = server.handle_frame(&frame(id, name, &arguments));
        assert!(output.contains(r#""isError":false"#), "{name}: {output}");
        let request = server
            .gateway()
            .requests
            .first()
            .ok_or_else(|| format!("{name} did not reach gateway"))?;
        assert_eq!(request.method, method, "{name}");
        assert_eq!(request.path, path, "{name}");
        assert_eq!(
            request
                .headers
                .get("x-sts2-schema-digest")
                .map(String::as_str),
            Some(COOP_NATIVE_SCHEMA_DIGEST)
        );
    }
    Ok(())
}

#[test]
fn effect_tool_projects_response_without_gateway_access() -> Result<(), String> {
    let body = response("effect", "corr-effect", Some("op-effect"))?.body;
    let mut server = server(GatewayResponse {
        status: 500,
        body: JsonValue::Null,
    });
    let arguments = format!(r#"{{{},"envelope":{}}}"#, common(), body.to_json());
    let output = server.handle_frame(&frame("corr-effect", COOP_NATIVE_EFFECT_TOOL, &arguments));
    assert!(output.contains(r#""isError":false"#));
    assert!(server.gateway().requests.is_empty());
    Ok(())
}

#[test]
fn rejects_foreign_identity_unknown_fields_and_digest_drift_before_or_at_boundary()
-> Result<(), String> {
    let gateway_response = response("observation", "corr-obs", None)?;
    let mut server_instance = server(gateway_response.clone());
    let foreign = format!(r#"{{{}}}"#, common().replace("mcp-1", "foreign"));
    assert!(
        server_instance
            .handle_frame(&frame("corr-obs", COOP_NATIVE_OBSERVATION_TOOL, &foreign))
            .contains("-32602")
    );
    assert!(server_instance.gateway().requests.is_empty());

    let mut unknown_server = server(gateway_response.clone());
    let unknown = format!(r#"{{{},"unexpected":true}}"#, common());
    assert!(
        unknown_server
            .handle_frame(&frame("corr-obs", COOP_NATIVE_OBSERVATION_TOOL, &unknown))
            .contains("-32602")
    );
    assert!(unknown_server.gateway().requests.is_empty());

    let bad = gateway_response.body.to_json().replace(
        COOP_NATIVE_SCHEMA_DIGEST,
        "0000000000000000000000000000000000000000000000000000000000000000",
    );
    let mut server = server(GatewayResponse {
        status: 200,
        body: parse_json(&bad)?,
    });
    let valid = format!(r#"{{{}}}"#, common());
    let output = server.handle_frame(&frame("corr-obs", COOP_NATIVE_OBSERVATION_TOOL, &valid));
    assert!(output.contains(r#""isError":true"#));
    Ok(())
}

#[test]
fn rejects_a_response_for_a_different_operation_identity() -> Result<(), String> {
    let gateway_response = response("effect", "corr-action", Some("wrong-op"))?;
    let mut server = server(gateway_response);
    let arguments = format!(
        r#"{{{},"operation_id":"op-1","actor_peer":"peer:host1","expected_host_generation":1,"action":{{"kind":"end_turn","action_id":"turn:1","target_peer":null}}}}"#,
        common()
    );
    let output = server.handle_frame(&frame("corr-action", COOP_NATIVE_ACTION_TOOL, &arguments));
    assert!(output.contains(r#""isError":true"#));
    assert_eq!(server.gateway().requests.len(), 1);
    Ok(())
}

#[test]
fn rejects_native_responses_without_the_merged_receipt_member() -> Result<(), String> {
    let valid = response("effect", "corr-effect", Some("op-effect"))?.body;
    let wire = valid.to_json();
    let receipt_start = wire
        .find(",\"receipt\":")
        .ok_or_else(|| String::from("receipt member is missing from the fixture"))?;
    let schema_start = wire
        .find(",\"schema_digest\":")
        .ok_or_else(|| String::from("schema digest member is missing from the fixture"))?;
    let malformed = format!("{}{}", &wire[..receipt_start], &wire[schema_start..]);
    let mut server = server(GatewayResponse {
        status: 200,
        body: parse_json(&malformed)?,
    });
    let arguments = format!(
        r#"{{{} ,"operation_id":"op-effect","actor_peer":"peer:host1","expected_host_generation":1,"action":{{"kind":"end_turn","action_id":"turn:1","target_peer":null}}}}"#,
        common()
    );
    let output = server.handle_frame(&frame("corr-effect", COOP_NATIVE_ACTION_TOOL, &arguments));
    assert!(output.contains(r#""isError":true"#), "{output}");
    Ok(())
}
