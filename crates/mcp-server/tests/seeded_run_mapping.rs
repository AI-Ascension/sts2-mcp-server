// SPDX-License-Identifier: MIT
#![allow(clippy::expect_used, clippy::panic)]

use std::collections::VecDeque;

use sts2_mcp_server::{
    GatewayAdapter, GatewayError, GatewayRequest, GatewayResponse, JsonValue, McpServer,
    ToolCatalog, parse_json, verify_seeded_run_artifact,
};

struct RecordingGateway {
    requests: Vec<GatewayRequest>,
    responses: VecDeque<Result<GatewayResponse, GatewayError>>,
}

impl RecordingGateway {
    fn new(responses: impl IntoIterator<Item = Result<GatewayResponse, GatewayError>>) -> Self {
        Self {
            requests: Vec::new(),
            responses: responses.into_iter().collect(),
        }
    }
}

impl GatewayAdapter for RecordingGateway {
    fn forward(&mut self, request: GatewayRequest) -> Result<GatewayResponse, GatewayError> {
        self.requests.push(request);
        self.responses
            .pop_front()
            .unwrap_or(Err(GatewayError::Unavailable))
    }
}

fn golden(name: &str) -> JsonValue {
    let path = format!("../../../protocol-artifact/seeded-run-v1/golden/{name}.json");
    let text = match name {
        "start-request" => {
            include_str!("../../../protocol-artifact/seeded-run-v1/golden/start-request.json")
        }
        "start-settled" => {
            include_str!("../../../protocol-artifact/seeded-run-v1/golden/start-settled.json")
        }
        "reconcile-settled" => {
            include_str!("../../../protocol-artifact/seeded-run-v1/golden/reconcile-settled.json")
        }
        _ => panic!("unsupported fixture {path}"),
    };
    parse_json(text).expect("fixture is valid JSON")
}

fn selected_context() -> JsonValue {
    let JsonValue::Object(object) = golden("start-request") else {
        panic!("start fixture is not an object");
    };
    object
        .get("selected_context")
        .cloned()
        .expect("start fixture selected_context")
}

fn frame(id: &str, name: &str, arguments: JsonValue) -> String {
    JsonValue::object([
        (String::from("jsonrpc"), JsonValue::string("2.0")),
        (String::from("id"), JsonValue::string(id)),
        (String::from("method"), JsonValue::string("tools/call")),
        (
            String::from("params"),
            JsonValue::object([
                (String::from("name"), JsonValue::string(name)),
                (String::from("arguments"), arguments),
            ]),
        ),
    ])
    .to_json()
}

fn start_arguments(id: &str) -> JsonValue {
    JsonValue::object([
        (String::from("instance_id"), JsonValue::string("instance-1")),
        (
            String::from("mcp_session_id"),
            JsonValue::string("session-1"),
        ),
        (String::from("lease_id"), JsonValue::string("lease-1")),
        (String::from("lease_epoch"), JsonValue::Number(1)),
        (String::from("generation"), JsonValue::Number(0)),
        (String::from("operation_id"), JsonValue::string("op-seed-1")),
        (String::from("seed"), JsonValue::string("ironclad-42")),
        (
            String::from("run_mode"),
            JsonValue::string("seeded_training"),
        ),
        (String::from("selected_context"), selected_context()),
        (String::from("correlation_hint"), JsonValue::string(id)),
    ])
}

#[test]
fn catalog_exposes_only_the_two_seeded_run_tools_and_concrete_context_schema() {
    let mut server =
        McpServer::with_catalog(RecordingGateway::new([]), ToolCatalog::seeded_run_v1());
    let output = server
        .handle_frame("{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\",\"params\":{}}");
    assert!(output.contains("seeded-run-v1-mcp"));
    assert!(output.contains("start_seeded_run"));
    assert!(output.contains("reconcile_seeded_run"));
    assert!(output.contains("selected_context"));
    assert!(!output.contains("plan_digest"));
    assert!(output.contains("^(?!.*\\\\.\\\\.)[A-Za-z0-9_.:-]{1,128}$"));
    assert_eq!(output.matches("\"name\"").count(), 2);
}

#[test]
fn start_maps_the_exact_envelope_and_fenced_route_without_harness_metadata() {
    let mut server = McpServer::with_catalog(
        RecordingGateway::new([Ok(GatewayResponse {
            status: 200,
            body: golden("start-settled"),
        })]),
        ToolCatalog::seeded_run_v1(),
    );
    let mut arguments = start_arguments("corr-seed-0001");
    if let JsonValue::Object(object) = &mut arguments {
        object.remove("correlation_hint");
    }
    let output = server.handle_frame(&frame("corr-seed-0001", "start_seeded_run", arguments));
    assert!(output.contains("\"isError\":false"), "{output}");
    assert!(
        output.contains("\\\"status\\\":\\\"settled\\\""),
        "{output}"
    );
    let request = &server.gateway().requests[0];
    assert_eq!(request.path, "/v2/instances/instance-1/seeded-run");
    assert_eq!(request.method, sts2_mcp_server::GatewayMethod::Post);
    assert_eq!(request.body, Some(golden("start-request")));
    assert!(!request.headers.contains_key("x-sts2-seed-plan-digest"));
    assert!(!request.headers.contains_key("x-sts2-seed-entry-ordinal"));
}

#[test]
fn reconcile_is_bodyless_and_preserves_operation_identity_and_authority() {
    let mut server = McpServer::with_catalog(
        RecordingGateway::new([Ok(GatewayResponse {
            status: 200,
            body: golden("reconcile-settled"),
        })]),
        ToolCatalog::seeded_run_v1(),
    );
    let arguments = JsonValue::object([
        (String::from("instance_id"), JsonValue::string("instance-1")),
        (
            String::from("mcp_session_id"),
            JsonValue::string("session-1"),
        ),
        (String::from("lease_id"), JsonValue::string("lease-1")),
        (String::from("lease_epoch"), JsonValue::Number(1)),
        (String::from("generation"), JsonValue::Number(0)),
        (
            String::from("operation_id"),
            JsonValue::string("op-seed-unknown"),
        ),
    ]);
    let output = server.handle_frame(&frame("corr-seed-0003", "reconcile_seeded_run", arguments));
    assert!(output.contains("\"isError\":false"), "{output}");
    let request = &server.gateway().requests[0];
    assert_eq!(
        request.path,
        "/v2/instances/instance-1/seeded-operations/op-seed-unknown"
    );
    assert!(request.body.is_none());
    assert_eq!(
        request
            .headers
            .get("x-sts2-lease-epoch")
            .map(String::as_str),
        Some("1")
    );
}

#[test]
fn timeout_is_unknown_without_mutation_retry_and_invalid_metadata_is_rejected() {
    let mut server = McpServer::with_catalog(
        RecordingGateway::new([Err(GatewayError::Timeout)]),
        ToolCatalog::seeded_run_v1(),
    );
    let mut arguments = start_arguments("corr-timeout");
    if let JsonValue::Object(object) = &mut arguments {
        object.remove("correlation_hint");
    }
    let output = server.handle_frame(&frame("corr-timeout", "start_seeded_run", arguments));
    assert!(
        output.contains("\\\"status\\\":\\\"unknown\\\""),
        "{output}"
    );
    assert_eq!(server.gateway().requests.len(), 1);

    let mut invalid = start_arguments("corr-invalid");
    if let JsonValue::Object(object) = &mut invalid {
        object.remove("correlation_hint");
        object.insert(String::from("plan_digest"), JsonValue::string("deadbeef"));
    }
    let rejected = server.handle_frame(&frame("corr-invalid", "start_seeded_run", invalid));
    assert!(rejected.contains("-32602"), "{rejected}");
    assert_eq!(server.gateway().requests.len(), 1);

    let mut unsafe_operation = start_arguments("corr-operation");
    if let JsonValue::Object(object) = &mut unsafe_operation {
        object.remove("correlation_hint");
        object.insert(String::from("operation_id"), JsonValue::string("op/seed"));
    }
    let rejected = server.handle_frame(&frame(
        "corr-operation",
        "start_seeded_run",
        unsafe_operation,
    ));
    assert!(rejected.contains("-32602"), "{rejected}");
    assert_eq!(server.gateway().requests.len(), 1);

    let mut digest_mismatch = start_arguments("corr-digest");
    if let JsonValue::Object(object) = &mut digest_mismatch {
        object.remove("correlation_hint");
        object.insert(
            String::from("context_digest"),
            JsonValue::string("0000000000000000000000000000000000000000000000000000000000000000"),
        );
    }
    let rejected = server.handle_frame(&frame("corr-digest", "start_seeded_run", digest_mismatch));
    assert!(rejected.contains("-32602"), "{rejected}");
    assert_eq!(server.gateway().requests.len(), 1);
}

#[test]
fn copied_seeded_run_artifact_is_verified() {
    assert_eq!(verify_seeded_run_artifact(), Ok(()));
}

#[test]
fn distinct_mcp_and_gateway_sessions_bind_start_and_reconcile_separately() {
    let mut start_response = golden("start-settled");
    if let JsonValue::Object(object) = &mut start_response {
        object.insert(
            String::from("correlation_id"),
            JsonValue::string("distinct-start"),
        );
        object.insert(
            String::from("session_id"),
            JsonValue::string("gateway-session-1"),
        );
    }
    let mut server = McpServer::with_catalog_and_sessions(
        RecordingGateway::new([Ok(GatewayResponse {
            status: 200,
            body: start_response,
        })]),
        ToolCatalog::seeded_run_v1(),
        "gateway-session-1",
        "mcp-session-1",
    );
    let mut arguments = start_arguments("distinct-start");
    if let JsonValue::Object(object) = &mut arguments {
        object.insert(
            String::from("mcp_session_id"),
            JsonValue::string("mcp-session-1"),
        );
        object.remove("correlation_hint");
    }
    let output = server.handle_frame(&frame("distinct-start", "start_seeded_run", arguments));
    assert!(
        output.contains("\\\"status\\\":\\\"settled\\\""),
        "{output}"
    );
    let request = &server.gateway().requests[0];
    assert_eq!(request.correlation.mcp_session_id, "mcp-session-1");
    assert_eq!(
        request.headers.get("x-mcp-session-id").map(String::as_str),
        Some("mcp-session-1")
    );
    assert_eq!(
        request.headers.get("x-sts2-session-id").map(String::as_str),
        Some("gateway-session-1")
    );
    let body_session = match request.body.as_ref() {
        Some(JsonValue::Object(body)) => body.get("session_id").and_then(|value| match value {
            JsonValue::String(value) => Some(value.as_str()),
            _ => None,
        }),
        _ => None,
    };
    assert_eq!(body_session, Some("gateway-session-1"));

    let mut reconcile_response = golden("reconcile-settled");
    if let JsonValue::Object(object) = &mut reconcile_response {
        object.insert(
            String::from("correlation_id"),
            JsonValue::string("distinct-reconcile"),
        );
        object.insert(
            String::from("session_id"),
            JsonValue::string("gateway-session-1"),
        );
    }
    let mut reconcile_server = McpServer::with_catalog_and_sessions(
        RecordingGateway::new([Ok(GatewayResponse {
            status: 200,
            body: reconcile_response,
        })]),
        ToolCatalog::seeded_run_v1(),
        "gateway-session-1",
        "mcp-session-1",
    );
    let arguments = JsonValue::object([
        (String::from("instance_id"), JsonValue::string("instance-1")),
        (
            String::from("mcp_session_id"),
            JsonValue::string("mcp-session-1"),
        ),
        (String::from("lease_id"), JsonValue::string("lease-1")),
        (String::from("lease_epoch"), JsonValue::Number(1)),
        (String::from("generation"), JsonValue::Number(0)),
        (
            String::from("operation_id"),
            JsonValue::string("op-seed-unknown"),
        ),
    ]);
    let output = reconcile_server.handle_frame(&frame(
        "distinct-reconcile",
        "reconcile_seeded_run",
        arguments,
    ));
    assert!(
        output.contains("\\\"status\\\":\\\"settled\\\""),
        "{output}"
    );
    let request = &reconcile_server.gateway().requests[0];
    assert_eq!(request.correlation.mcp_session_id, "mcp-session-1");
    assert!(request.body.is_none());
    assert_eq!(
        request.headers.get("x-mcp-session-id").map(String::as_str),
        Some("mcp-session-1")
    );
    assert_eq!(
        request.headers.get("x-sts2-session-id").map(String::as_str),
        Some("gateway-session-1")
    );
}
