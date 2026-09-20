// SPDX-License-Identifier: MIT

use super::*;
use crate::gateway::{GatewayError, GatewayMethod};
use crate::{GatewayRequest, GatewayResponse, JsonValue, McpServer, ToolCatalog, parse_json};

#[derive(Clone)]
struct RecordingGateway {
    response: GatewayResponse,
    request: Option<GatewayRequest>,
}

impl GatewayAdapter for RecordingGateway {
    fn forward(&mut self, request: GatewayRequest) -> Result<GatewayResponse, GatewayError> {
        self.request = Some(request);
        Ok(self.response.clone())
    }
}

fn response() -> Result<GatewayResponse, String> {
    Ok(GatewayResponse {
        status: 503,
        body: parse_json(
            r#"{"protocol_version":"game-information-lookup-binding-v1","schema_digest":"f10f9af01d6be1de104069ba842e7971971e88f27553e782e81174ee7aa1cd58","provenance":{"artifact":"sts2-protocol/game-information-lookup-binding-v1","source":"schemas/game-information-lookup-binding-v1.schema.json","generator":"hand-authored"},"correlation_id":"corr:17:1","kind":"error_response","binding":null,"discovery":null,"observation":null,"error":{"code":"missing_capability","field":null,"reason":null}}"#,
        )?,
    })
}

fn valid_frame() -> &'static str {
    r#"{"jsonrpc":"2.0","id":"corr:17:1","method":"tools/call","params":{"name":"sts2.game_information_binding","arguments":{"instance_id":"instance-1","mcp_session_id":"mcp-session-1","lease_id":"lease-1","lease_epoch":9,"operation":"discovery","project_id":"project-1","run_id":"run-1","episode_id":"episode-1","agent_id":"agent-1","authority_epoch":3}}}"#
}

#[test]
fn forwards_closed_binding_read_to_exact_gateway_route() -> Result<(), String> {
    let gateway = RecordingGateway {
        response: response()?,
        request: None,
    };
    let mut server = McpServer::with_catalog_and_sessions(
        gateway,
        ToolCatalog::game_information(),
        "gateway-session-1",
        "mcp-session-1",
    );
    let output = server.handle_frame(valid_frame());
    if !output.contains("\"isError\":true") || !output.contains("missing_capability") {
        return Err(format!("unexpected MCP output: {output}"));
    }
    let request = server
        .gateway()
        .request
        .as_ref()
        .ok_or_else(|| String::from("gateway was not called"))?;
    if request.method != GatewayMethod::Post
        || request.path != "/v1/instances/instance-1/game-information/lookup-binding"
        || request.headers.get("x-sts2-session-id").map(String::as_str) != Some("gateway-session-1")
    {
        return Err(String::from("binding route or authority was not exact"));
    }
    if request.body.as_ref().map(JsonValue::to_json).as_deref()
        != Some(
            r#"{"agent_id":"agent-1","authority_epoch":3,"episode_id":"episode-1","operation":"discovery","project_id":"project-1","run_id":"run-1"}"#,
        )
    {
        return Err(String::from(
            "binding request body was not closed and canonical",
        ));
    }
    Ok(())
}

#[test]
fn rejects_unknown_binding_operation_before_gateway() -> Result<(), String> {
    let gateway = RecordingGateway {
        response: response()?,
        request: None,
    };
    let mut server = McpServer::with_catalog_and_sessions(
        gateway,
        ToolCatalog::game_information(),
        "gateway-session-1",
        "mcp-session-1",
    );
    let output = server.handle_frame(
        &valid_frame().replace(r#""operation":"discovery""#, r#""operation":"mutate""#),
    );
    if !output.contains("\"code\":-32602") || server.gateway().request.is_some() {
        return Err(String::from(
            "invalid binding operation reached the gateway",
        ));
    }
    Ok(())
}

#[path = "mapping_game_information_binding_artifact_tests.rs"]
mod artifact_tests;
#[path = "mapping_game_information_binding_boundary_tests.rs"]
mod boundary_tests;
#[path = "mapping_game_information_binding_vector_data.rs"]
mod vector_data;
