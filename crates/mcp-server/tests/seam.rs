// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use sts2_mcp_server::{
    Correlation, GatewayAdapter, GatewayError, GatewayMethod, GatewayRequest, GatewayResponse,
    JsonValue, McpServer, RequestId, verify_poc_artifact,
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
         \"params\":{{\"name\":\"get_state\",\"arguments\":{{\
         \"instance_id\":\"{instance}\",\"mcp_session_id\":\"{session}\"}}}}}}"
    )
}

fn action_call(id: &str, session: &str, instance: &str, generation: i64, units: i64) -> String {
    format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":\"{id}\",\"method\":\"tools/call\",\
         \"params\":{{\"name\":\"submit_action\",\"arguments\":{{\
         \"instance_id\":\"{instance}\",\"mcp_session_id\":\"{session}\",\
         \"generation\":{generation},\"action_id\":\"use_budget\",\"units\":{units}}}}}}}"
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
    assert!(catalog.contains("get_state"));
    assert!(catalog.contains("submit_action"));
    assert_eq!(catalog.matches("\"name\"").count(), 2);
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
         {\"name\":\"get_state\",\"arguments\":[]}}",
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

#[test]
fn submit_action_maps_to_post_and_preserves_poc_request_fields() -> Result<(), String> {
    verify_poc_artifact().map_err(|error| error.to_string())?;
    let mut server = McpServer::new(FakeGateway::success(JsonValue::object([(
        String::from("status"),
        JsonValue::string("accepted"),
    )])));

    let response = server.handle_frame(&action_call("request-9", "session-4", "instance-1", 0, 1));

    if !response.contains("\"isError\":false") {
        return Err(format!("unexpected MCP response: {response}"));
    }
    let request = server.gateway().first_request();
    assert_eq!(request.method, GatewayMethod::Post);
    assert_eq!(request.path, "/v1/instances/instance-1/action");
    assert_eq!(
        request.correlation,
        Correlation {
            mcp_session_id: String::from("session-4"),
            mcp_request_id: RequestId::String(String::from("request-9")),
        }
    );
    let Some(JsonValue::Object(body)) = request.body.as_ref() else {
        return Err(String::from("action request did not contain a JSON body"));
    };
    assert_eq!(
        body.get("protocol_version"),
        Some(&JsonValue::string("poc-v1"))
    );
    assert_eq!(
        body.get("schema_digest"),
        Some(&JsonValue::string(
            "adb434d119a51b00d968e71bf0bf774f2a08de7c875a5479900aa34b3c02e027"
        ))
    );
    assert_eq!(body.get("kind"), Some(&JsonValue::string("action_request")));
    assert_eq!(body.get("generation"), Some(&JsonValue::Number(0)));
    assert_eq!(
        body.get("action"),
        Some(&JsonValue::object([
            (String::from("action_id"), JsonValue::string("use_budget")),
            (String::from("units"), JsonValue::Number(1)),
        ]))
    );
    Ok(())
}

#[test]
fn zero_unit_action_is_forwarded_for_core_rejection_identity() {
    let mut server = McpServer::new(FakeGateway::success(JsonValue::object([(
        String::from("error_code"),
        JsonValue::string("sts2.game-core/zero_units"),
    )])));

    let response = server.handle_frame(&action_call("request-10", "session-4", "instance-1", 1, 0));

    assert!(response.contains("sts2.game-core/zero_units"));
    assert_eq!(server.gateway().requests.len(), 1);
}

#[test]
fn action_shape_errors_are_rejected_before_gateway_access() {
    let mut server = McpServer::new(FakeGateway::success(JsonValue::Null));
    let response = server.handle_frame(
        "{\"jsonrpc\":\"2.0\",\"id\":4,\"method\":\"tools/call\",\"params\":\
         {\"name\":\"submit_action\",\"arguments\":{\"instance_id\":\"instance-1\",\
         \"mcp_session_id\":\"session-1\",\"action_id\":\"use_budget\",\"units\":1}}}",
    );

    assert!(response.contains("\"code\":-32602"));
    assert_eq!(server.gateway().requests.len(), 0);
}
