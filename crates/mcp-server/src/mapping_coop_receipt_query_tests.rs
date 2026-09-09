// SPDX-License-Identifier: MIT

use super::*;
use crate::gateway::GatewayError;
use crate::{GatewayResponse, McpServer, ToolCatalog, canonical_coop_receipt_query, parse_json};

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
    let body = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../protocol-artifact/coop-receipt-query-v1/golden/receipt-query-response-settled.json"
    ));
    Ok(GatewayResponse {
        status: 200,
        body: parse_json(body)?,
    })
}

fn valid_frame() -> &'static str {
    r#"{"jsonrpc":"2.0","id":"corr:17:1","method":"tools/call","params":{"name":"sts2.coop_receipt_query","arguments":{"instance_id":"instance-1","mcp_session_id":"mcp-session-1","lease_id":"lease-1","lease_epoch":9,"operation_id":"op:run-17:0001","action_kind":"play_card","action_fingerprint":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","run_id":"run-17","location":{"act_index":1,"room_id":42,"coord":{"col":3,"row":5}},"actor_id":"peer-1","authority_id":"native-host-a","authority_epoch":"epoch-9","expected_host_generation":17,"before_host_generation":17,"participant_ids":["peer-1","peer-2"]}}}"#
}

#[test]
fn forwards_one_read_only_request_with_frozen_canonical_bytes() -> Result<(), String> {
    let gateway = RecordingGateway {
        response: response()?,
        request: None,
    };
    let mut server = McpServer::with_catalog_and_sessions(
        gateway,
        ToolCatalog::coop_receipt_query(),
        "session-native-17",
        "mcp-session-1",
    );
    let output = server.handle_frame(valid_frame());
    if !output.contains("\"isError\":false") || !output.contains("receipt_query_response") {
        return Err(format!("unexpected MCP output: {output}"));
    }
    let request = server
        .gateway()
        .request
        .as_ref()
        .ok_or_else(|| String::from("gateway was not called"))?;
    if request.method != GatewayMethod::Post
        || request.path != "/v1/instances/instance-1/coop/receipt-query"
        || request.headers.get("x-sts2-session-id").map(String::as_str) != Some("session-native-17")
    {
        return Err(String::from(
            "receipt-query route or authority was not exact",
        ));
    }
    let body = request
        .body
        .as_ref()
        .ok_or_else(|| String::from("receipt-query body was omitted"))?;
    let encoded = canonical_coop_receipt_query(body)
        .ok_or_else(|| String::from("receipt-query body was not canonicalizable"))?;
    let expected = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../protocol-artifact/coop-receipt-query-v1/golden/receipt-query-request.json"
    ));
    if encoded.as_bytes() != expected {
        return Err(String::from(
            "receipt-query body differed from the golden wire bytes",
        ));
    }
    Ok(())
}

#[test]
fn rejects_actor_absence_and_unsorted_participants_before_gateway() -> Result<(), String> {
    for replacement in [
        "\"participant_ids\":[\"peer-2\",\"peer-3\"]",
        "\"participant_ids\":[\"peer-2\",\"peer-1\"]",
    ] {
        let frame =
            valid_frame().replace("\"participant_ids\":[\"peer-1\",\"peer-2\"]", replacement);
        let gateway = RecordingGateway {
            response: response()?,
            request: None,
        };
        let mut server = McpServer::with_catalog_and_sessions(
            gateway,
            ToolCatalog::coop_receipt_query(),
            "session-native-17",
            "mcp-session-1",
        );
        let output = server.handle_frame(&frame);
        if !output.contains("\"code\":-32602") || server.gateway().request.is_some() {
            return Err(String::from(
                "invalid participant identity reached the gateway",
            ));
        }
    }
    Ok(())
}
