// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use sts2_mcp_server::{
    Correlation, GatewayAdapter, GatewayError, GatewayMethod, GatewayRequest, GatewayResponse,
    JsonValue, McpServer, RequestId,
};

struct FakeGateway {
    requests: Vec<GatewayRequest>,
    outcome: Result<JsonValue, GatewayError>,
}

impl FakeGateway {
    fn success(body: JsonValue) -> Self {
        Self {
            requests: Vec::new(),
            outcome: Ok(body),
        }
    }

    fn failure(error: GatewayError) -> Self {
        Self {
            requests: Vec::new(),
            outcome: Err(error),
        }
    }

    fn first_request(&self) -> &GatewayRequest {
        &self.requests[0]
    }
}

impl GatewayAdapter for FakeGateway {
    fn forward(&mut self, request: GatewayRequest) -> Result<GatewayResponse, GatewayError> {
        self.requests.push(request);
        match &self.outcome {
            Ok(body) => Ok(GatewayResponse {
                status: 200,
                body: body.clone(),
            }),
            Err(error) => Err(*error),
        }
    }
}

fn state_call(id: &str, session: &str, instance: &str) -> String {
    format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":\"{id}\",\"method\":\"tools/call\",\
         \"params\":{{\"name\":\"sts2_get_state\",\"arguments\":{{\
         \"instance_id\":\"{instance}\",\"mcp_session_id\":\"{session}\"}}}}}}"
    )
}

#[test]
fn valid_call_maps_to_one_owned_gateway_request() {
    let mut server = McpServer::new(FakeGateway::success(JsonValue::object([(
        String::from("phase"),
        JsonValue::string("ready"),
    )])));

    let response = server.handle_frame(&state_call("request-7", "session-1", "instance-1"));

    assert!(response.contains("\"isError\":false"));
    assert!(response.contains("ready"));
    let request = server.gateway().first_request();
    assert_eq!(request.method, GatewayMethod::Get);
    assert_eq!(request.path, "/v1/instances/instance-1/state");
    assert!(request.body.is_none());
}

#[test]
fn catalog_and_initialize_advertise_only_the_local_tools_capability() {
    let mut server = McpServer::new(FakeGateway::success(JsonValue::Null));

    let initialize = server.handle_frame(
        "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{\"capabilities\":{}}}",
    );
    let catalog = server
        .handle_frame("{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/list\",\"params\":{}}");

    assert!(initialize.contains("\"capabilities\":{\"tools\":{}}"));
    assert!(catalog.contains("sts2_get_state"));
    assert!(!catalog.contains("combat_play_card"));
    assert_eq!(server.gateway().requests.len(), 0);
}

#[test]
fn malformed_json_is_rejected_before_gateway_access() {
    let mut server = McpServer::new(FakeGateway::success(JsonValue::Null));

    let response =
        server.handle_frame("{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":");

    assert!(response.contains("\"code\":-32700"));
    assert!(response.contains("\"id\":null"));
    assert_eq!(server.gateway().requests.len(), 0);
}

#[test]
fn malformed_tool_arguments_are_invalid_params() {
    let mut server = McpServer::new(FakeGateway::success(JsonValue::Null));

    let response = server.handle_frame(
        "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/call\",\"params\":\
         {\"name\":\"sts2_get_state\",\"arguments\":[]}}",
    );

    assert!(response.contains("\"code\":-32602"));
    assert_eq!(server.gateway().requests.len(), 0);
}

#[test]
fn unsupported_capability_is_rejected_without_forwarding() {
    let mut server = McpServer::new(FakeGateway::success(JsonValue::Null));

    let response = server
        .handle_frame("{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"resources/list\",\"params\":{}}");

    assert!(response.contains("\"code\":-32601"));
    assert!(response.contains("not supported"));
    assert_eq!(server.gateway().requests.len(), 0);
}

#[test]
fn correlation_preserves_mcp_request_and_session_namespaces() {
    let mut server = McpServer::new(FakeGateway::success(JsonValue::object([])));

    let response =
        server.handle_frame(&state_call("mcp-request-42", "mcp-session-9", "instance-2"));

    assert!(response.contains("\"id\":\"mcp-request-42\""));
    let request = server.gateway().first_request();
    assert_eq!(
        request.correlation,
        Correlation {
            mcp_session_id: String::from("mcp-session-9"),
            mcp_request_id: RequestId::String(String::from("mcp-request-42")),
        }
    );
    assert_eq!(
        request.headers,
        BTreeMap::from([
            (
                String::from("x-mcp-request-id"),
                String::from("mcp-request-42"),
            ),
            (
                String::from("x-mcp-session-id"),
                String::from("mcp-session-9"),
            ),
        ])
    );
}

#[test]
fn gateway_authorization_error_maps_to_stable_rpc_error() {
    let mut server = McpServer::new(FakeGateway::failure(GatewayError::Unauthorized));

    let response = server.handle_frame(&state_call("request-8", "session-2", "instance-3"));

    assert!(response.contains("\"code\":-32001"));
    assert!(response.contains("gateway authorization failed"));
    assert!(response.contains("\"id\":\"request-8\""));
    assert_eq!(server.gateway().requests.len(), 1);
}
