// SPDX-License-Identifier: MIT
#![allow(clippy::expect_used, clippy::panic)]

use std::collections::VecDeque;

use serde_json::Value;
use sts2_mcp_server::{
    CapabilityLayer, CapabilityOwner, CapabilityScope,
    GAME_INFORMATION_LIVE_OBSERVATION_BOOTSTRAP_TOOL, GatewayAdapter, GatewayError, GatewayMethod,
    GatewayRequest, GatewayResponse, JsonValue, McpServer, ToolCatalog, parse_json,
    verify_live_bootstrap_artifact,
};

struct FakeGateway {
    requests: Vec<GatewayRequest>,
    responses: VecDeque<Result<GatewayResponse, GatewayError>>,
}

impl FakeGateway {
    fn new(responses: impl IntoIterator<Item = Result<GatewayResponse, GatewayError>>) -> Self {
        Self {
            requests: Vec::new(),
            responses: responses.into_iter().collect(),
        }
    }
}

impl GatewayAdapter for FakeGateway {
    fn forward(&mut self, request: GatewayRequest) -> Result<GatewayResponse, GatewayError> {
        self.requests.push(request);
        self.responses
            .pop_front()
            .unwrap_or(Err(GatewayError::Unavailable))
    }
}

fn golden(name: &str) -> JsonValue {
    let text = match name {
        "request" => include_str!(
            "../../../protocol-artifact/game-information-live-observation-bootstrap-v1/golden/bootstrap-request.json"
        ),
        "response" => include_str!(
            "../../../protocol-artifact/game-information-live-observation-bootstrap-v1/golden/bootstrap-response.json"
        ),
        "unavailable" => include_str!(
            "../../../protocol-artifact/game-information-live-observation-bootstrap-v1/golden/error-native-unavailable.json"
        ),
        _ => panic!("unsupported golden"),
    };
    parse_json(text).expect("valid bootstrap golden")
}

fn invalid(name: &str) -> JsonValue {
    let text = match name {
        "foreign" => include_str!(
            "../../../conformance/fixtures/game-information-live-observation-bootstrap-v1/invalid/foreign-instance.json"
        ),
        "malformed" => include_str!(
            "../../../conformance/fixtures/game-information-live-observation-bootstrap-v1/invalid/missing-native-ref.json"
        ),
        "stale" => include_str!(
            "../../../conformance/fixtures/game-information-live-observation-bootstrap-v1/invalid/stale-generation.json"
        ),
        "oversized" => include_str!(
            "../../../conformance/fixtures/game-information-live-observation-bootstrap-v1/invalid/duplicate-visible-entity.json"
        ),
        _ => panic!("unsupported fixture"),
    };
    let mut value = parse_json(text).expect("valid fixture JSON");
    if let JsonValue::Object(object) = &mut value {
        object.insert(
            "schema_digest".to_owned(),
            JsonValue::string(sts2_mcp_server::LIVE_BOOTSTRAP_SCHEMA_DIGEST),
        );
    }
    value
}

fn arguments() -> JsonValue {
    parse_json(
        r#"{
          "instance_id":"instance-1",
          "mcp_session_id":"mcp-session-1",
          "lease_id":"lease-1",
          "lease_epoch":7,
          "run_id":"run-42",
          "authority_epoch":7,
          "content_manifest_id":"content-1",
          "locale":"en-US",
          "definition_ref":{"content_manifest_id":"content-1","entity_kind":"card","namespaced_id":"ironclad:strike","variant":null},
          "instance_ref":null,
          "max_visible_entities":2,
          "max_item_bytes":4096,
          "max_message_bytes":262144
        }"#,
    )
    .expect("bootstrap arguments")
}

fn frame(id: &str, arguments: JsonValue) -> String {
    JsonValue::object([
        ("jsonrpc".to_owned(), JsonValue::string("2.0")),
        ("id".to_owned(), JsonValue::string(id)),
        ("method".to_owned(), JsonValue::string("tools/call")),
        (
            "params".to_owned(),
            JsonValue::object([
                (
                    "name".to_owned(),
                    JsonValue::string(GAME_INFORMATION_LIVE_OBSERVATION_BOOTSTRAP_TOOL),
                ),
                ("arguments".to_owned(), arguments),
            ]),
        ),
    ])
    .to_json()
}

fn wire(output: &str) -> Value {
    serde_json::from_str(output).expect("MCP response JSON")
}

fn with_correlation(mut body: JsonValue, correlation: &str) -> JsonValue {
    if let JsonValue::Object(object) = &mut body {
        object.insert("correlation_id".to_owned(), JsonValue::string(correlation));
    }
    body
}

#[test]
fn artifact_and_in_process_http_route_mapping_is_pinned() {
    verify_live_bootstrap_artifact().expect("artifact is pinned");
    let request = golden("request");
    let response = with_correlation(golden("response"), "corr-bootstrap-1");
    let mut server = McpServer::with_catalog_and_sessions(
        FakeGateway::new([Ok(GatewayResponse {
            status: 200,
            body: response,
        })]),
        ToolCatalog::game_information_live_observation_bootstrap(),
        "gateway-session-1",
        "mcp-session-1",
    );
    let output = wire(&server.handle_frame(&frame("corr-bootstrap-1", arguments())));
    assert_eq!(output["result"]["isError"], false, "{output}");
    assert_eq!(server.gateway().requests.len(), 1);
    let forwarded = &server.gateway().requests[0];
    assert_eq!(forwarded.method, GatewayMethod::Post);
    assert_eq!(
        forwarded.path,
        "/v1/instances/instance-1/game-information/live-observation-bootstrap"
    );
    assert_eq!(forwarded.body.as_ref(), Some(&request));
    assert_eq!(
        forwarded.headers.get("x-sts2-instance-id"),
        Some(&String::from("instance-1"))
    );
    assert_eq!(
        forwarded.headers.get("x-sts2-session-id"),
        Some(&String::from("gateway-session-1"))
    );
    assert_eq!(
        forwarded.headers.get("x-sts2-lease-id"),
        Some(&String::from("lease-1"))
    );
    assert_eq!(
        forwarded.headers.get("x-sts2-lease-epoch"),
        Some(&String::from("7"))
    );
}

#[test]
fn stale_foreign_malformed_and_oversized_responses_fail_closed() {
    let mut oversized = golden("response");
    if let JsonValue::Object(object) = &mut oversized {
        let JsonValue::Object(limits) = object.get_mut("limits").expect("golden limits") else {
            panic!("golden limits are not an object");
        };
        limits.insert("max_message_bytes".to_owned(), JsonValue::Number(1));
    }
    let cases = [
        ("foreign", invalid("foreign"), "snapshot is incoherent"),
        ("stale", invalid("stale"), "snapshot is incoherent"),
        (
            "malformed",
            invalid("malformed"),
            "unknown or missing fields",
        ),
        (
            "duplicate",
            duplicate_visible_response(),
            "contain a duplicate instance",
        ),
        ("oversized", oversized, "exceeds max_message_bytes"),
    ];
    let responses = cases.iter().map(|(id, body, _)| {
        Ok(GatewayResponse {
            status: 200,
            body: with_correlation(body.clone(), id),
        })
    });
    let mut server = McpServer::with_catalog(
        FakeGateway::new(responses),
        ToolCatalog::game_information_live_observation_bootstrap(),
    );
    for (index, (id, _, reason)) in cases.into_iter().enumerate() {
        let output = wire(&server.handle_frame(&frame(id, arguments())));
        assert_eq!(output["result"]["isError"], true, "{index}: {output}");
        assert!(
            output["result"]["content"][0]["text"]
                .as_str()
                .is_some_and(|text| text.contains(reason)),
            "{index}: expected {reason}, got {output}"
        );
    }
}

fn duplicate_visible_response() -> JsonValue {
    let mut response = golden("response");
    if let JsonValue::Object(object) = &mut response {
        let JsonValue::Array(entities) = object
            .get_mut("visible_entities")
            .expect("golden visible entities")
        else {
            panic!("golden visible entities are not an array");
        };
        entities[1] = entities[0].clone();
    }
    response
}

#[test]
fn native_unavailable_error_is_a_structured_tool_error() {
    let mut body = golden("unavailable");
    body = with_correlation(body, "unavailable");
    let mut server = McpServer::with_catalog(
        FakeGateway::new([Ok(GatewayResponse { status: 409, body })]),
        ToolCatalog::game_information_live_observation_bootstrap(),
    );
    let output = wire(&server.handle_frame(&frame("unavailable", arguments())));
    assert_eq!(output["result"]["isError"], true, "{output}");
    assert_eq!(
        output["result"]["structuredContent"]["error"]["code"],
        "not_observable"
    );
}

#[test]
fn tool_is_read_only_and_requires_both_remote_layers() {
    let mut standalone = McpServer::with_catalog(
        FakeGateway::new([]),
        ToolCatalog::game_information_live_observation_bootstrap(),
    );
    let listed: Value = serde_json::from_str(
        &standalone.handle_frame(r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#),
    )
    .expect("catalog JSON");
    let tool = &listed["result"]["tools"][0];
    assert_eq!(
        tool["name"],
        GAME_INFORMATION_LIVE_OBSERVATION_BOOTSTRAP_TOOL
    );
    assert_eq!(tool["annotations"]["readOnlyHint"], true);
    assert_eq!(tool["annotations"]["destructiveHint"], false);
    let profiles = [ToolCatalog::game_information_live_observation_bootstrap()];
    let gateway =
        CapabilityLayer::from_catalogs(CapabilityOwner::Gateway, &profiles).expect("gateway");
    let producer =
        CapabilityLayer::from_catalogs(CapabilityOwner::Producer, &profiles).expect("producer");
    let composed =
        ToolCatalog::compose_profiles(&profiles, gateway, producer, CapabilityScope::READ)
            .expect("composed profile");
    assert!(
        composed
            .tools()
            .iter()
            .any(|tool| tool.name == GAME_INFORMATION_LIVE_OBSERVATION_BOOTSTRAP_TOOL)
    );
    let missing_gateway =
        CapabilityLayer::from_catalogs(CapabilityOwner::Producer, &profiles).expect("producer");
    let no_gateway = ToolCatalog::compose_profiles(
        &profiles,
        CapabilityLayer::new(CapabilityOwner::Gateway, "missing-gateway"),
        missing_gateway,
        CapabilityScope::READ,
    )
    .expect("composition without gateway");
    assert!(
        !no_gateway
            .tools()
            .iter()
            .any(|tool| tool.name == GAME_INFORMATION_LIVE_OBSERVATION_BOOTSTRAP_TOOL)
    );
    let missing_producer =
        CapabilityLayer::from_catalogs(CapabilityOwner::Gateway, &profiles).expect("gateway");
    let no_producer = ToolCatalog::compose_profiles(
        &profiles,
        missing_producer,
        CapabilityLayer::new(CapabilityOwner::Producer, "missing-producer"),
        CapabilityScope::READ,
    )
    .expect("composition without producer");
    assert!(
        !no_producer
            .tools()
            .iter()
            .any(|tool| tool.name == GAME_INFORMATION_LIVE_OBSERVATION_BOOTSTRAP_TOOL)
    );
    assert!(
        !ToolCatalog::game_information()
            .tools()
            .iter()
            .any(|tool| tool.name == GAME_INFORMATION_LIVE_OBSERVATION_BOOTSTRAP_TOOL)
    );
}
