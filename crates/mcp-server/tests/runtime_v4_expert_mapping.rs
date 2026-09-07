// SPDX-License-Identifier: MIT

use serde_json::{Value, json};
use sts2_mcp_server::{
    GatewayAdapter, GatewayError, GatewayMethod, GatewayRequest, GatewayResponse, JsonValue,
    McpServer, ToolCatalog, parse_json,
};

const GOLDEN: &str =
    include_str!("../../../protocol-artifact/runtime-v4-expert/golden/observation.json");

struct Gateway {
    requests: Vec<GatewayRequest>,
    body: JsonValue,
}

impl GatewayAdapter for Gateway {
    fn forward(&mut self, request: GatewayRequest) -> Result<GatewayResponse, GatewayError> {
        self.requests.push(request);
        Ok(GatewayResponse {
            status: 200,
            body: self.body.clone(),
        })
    }
}

fn server(wire: &str) -> Result<McpServer<Gateway>, Box<dyn std::error::Error>> {
    Ok(McpServer::with_catalog_and_sessions(
        Gateway {
            requests: Vec::new(),
            body: parse_json(wire)?,
        },
        ToolCatalog::runtime_v4_expert(),
        "gateway-session-1",
        "mcp-session-1",
    ))
}

fn arguments() -> Value {
    json!({"instance_id":"instance-1","mcp_session_id":"mcp-session-1"})
}

fn call(arguments: Value) -> String {
    json!({
        "jsonrpc":"2.0",
        "id":1,
        "method":"tools/call",
        "params":{"name":"sts2.expert_state","arguments":arguments}
    })
    .to_string()
}

fn action_call(arguments: Value) -> String {
    json!({
        "jsonrpc":"2.0",
        "id":1,
        "method":"tools/call",
        "params":{"name":"sts2.expert_action","arguments":arguments}
    })
    .to_string()
}

#[test]
fn serialized_gateway_observation_reaches_the_expert_tool() -> Result<(), Box<dyn std::error::Error>>
{
    let mut server = server(GOLDEN)?;
    let result: Value = serde_json::from_str(&server.handle_frame(&call(arguments())))?;
    assert_eq!(result["result"]["isError"], false);
    let body: Value = serde_json::from_str(
        result["result"]["content"][0]["text"]
            .as_str()
            .ok_or("MCP content text absent")?,
    )?;
    assert_eq!(body, serde_json::from_str::<Value>(GOLDEN)?);
    let request = &server.gateway().requests[0];
    assert_eq!(request.method, GatewayMethod::Get);
    assert_eq!(request.path, "/v4/instances/instance-1/expert-state");
    assert!(request.body.is_none());
    assert_eq!(
        request.headers.get("x-mcp-session-id").map(String::as_str),
        Some("mcp-session-1")
    );
    assert_eq!(request.correlation.mcp_session_id, "mcp-session-1");
    Ok(())
}

#[test]
fn expert_projection_rejects_unknown_fields_and_foreign_arguments_before_forwarding()
-> Result<(), Box<dyn std::error::Error>> {
    let mut value: Value = serde_json::from_str(GOLDEN)?;
    value["player"]["private"] = json!("secret");
    let mut server_with_unknown = server(&value.to_string())?;
    let result = server_with_unknown.handle_frame(&call(arguments()));
    assert!(result.contains("\"isError\":true"));

    let mut foreign = arguments();
    foreign["extra"] = json!(true);
    let mut server = server(GOLDEN)?;
    let result = server.handle_frame(&call(foreign));
    assert!(result.contains("-32602"));
    assert!(server.gateway().requests.is_empty());
    Ok(())
}

#[test]
fn expert_catalog_is_read_only_and_profile_scoped() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = server(GOLDEN)?;
    assert_eq!(server.catalog().revision, "runtime-v4-expert-mcp");
    assert_eq!(server.catalog().max_frame_bytes(), 256 * 1024);
    let listed: Value = serde_json::from_str(
        &server.handle_frame(r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#),
    )?;
    let tools = listed["result"]["tools"].as_array().ok_or("tools absent")?;
    assert_eq!(tools.len(), 2);
    assert_eq!(tools[0]["name"], "sts2.expert_state");
    assert_eq!(tools[1]["name"], "sts2.expert_action");
    Ok(())
}

#[test]
fn expert_action_maps_a_fenced_potion_and_preserves_settlement_witness()
-> Result<(), Box<dyn std::error::Error>> {
    let mut settled: Value = serde_json::from_str(include_str!(
        "../../../protocol-artifact/runtime-v4-expert-action/golden/action-settled.json"
    ))?;
    let mut observation: Value = serde_json::from_str(GOLDEN)?;
    observation["generation"] = json!(8);
    observation["state_id"] = json!("live:8");
    settled["correlation_id"] = json!("1");
    settled["observation"] = observation;
    let mut server = server(&settled.to_string())?;
    let arguments = json!({
        "instance_id":"instance-1",
        "mcp_session_id":"mcp-session-1",
        "lease_id":"lease-1",
        "lease_epoch":1,
        "generation":7,
        "state_id":"live:7",
        "operation_id":"potion-op-1",
        "action": {
            "action_id":"potion:7:potion:fire:enemy:1",
            "action":{"kind":"use_potion","potion_id":"potion:fire","target_id":"enemy:1"}
        }
    });
    let result: Value = serde_json::from_str(&server.handle_frame(&action_call(arguments)))?;
    assert_eq!(result["result"]["isError"], false);
    let body: Value = serde_json::from_str(
        result["result"]["content"][0]["text"]
            .as_str()
            .ok_or("MCP content text absent")?,
    )?;
    assert_eq!(body["status"], "settled");
    assert_eq!(body["operation_id"], "potion-op-1");
    assert_eq!(body["transition"]["removed"], true);
    let request = &server.gateway().requests[0];
    assert_eq!(request.method, GatewayMethod::Post);
    assert_eq!(request.path, "/v4/instances/instance-1/expert-action");
    assert!(request.body.is_some());
    assert_eq!(request.correlation.mcp_session_id, "mcp-session-1");
    Ok(())
}
