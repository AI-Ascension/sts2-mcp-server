// SPDX-License-Identifier: MIT

use std::collections::VecDeque;

#[path = "../src/protocol_artifact_runtime_v2_hash.rs"]
mod hash;

use sts2_mcp_server::{
    COOP_SYNCHRONIZATION_TOOL, GatewayAdapter, GatewayError, GatewayMethod, GatewayRequest,
    GatewayResponse, JsonValue, McpServer, ToolCatalog,
};

struct FakeGateway {
    requests: Vec<GatewayRequest>,
    responses: VecDeque<Result<GatewayResponse, GatewayError>>,
}

impl GatewayAdapter for FakeGateway {
    fn forward(&mut self, request: GatewayRequest) -> Result<GatewayResponse, GatewayError> {
        self.requests.push(request);
        self.responses
            .pop_front()
            .unwrap_or(Err(GatewayError::Unavailable))
    }
}

fn response() -> JsonValue {
    JsonValue::object([
        (
            String::from("protocol_version"),
            JsonValue::string("coop-gameplay-v1"),
        ),
        (
            String::from("schema_digest"),
            JsonValue::string("85e0028c1ae20e49542791da165eeabaaea0cc2023626b5094b6660ebcc0cc81"),
        ),
        (
            String::from("provenance"),
            JsonValue::object([
                (
                    String::from("artifact"),
                    JsonValue::string("sts2-protocol/coop-gameplay-v1"),
                ),
                (
                    String::from("source"),
                    JsonValue::string("schemas/coop-gameplay-v1.schema.json"),
                ),
                (
                    String::from("generator"),
                    JsonValue::string("hand-authored"),
                ),
            ]),
        ),
        (String::from("correlation_id"), JsonValue::string("1")),
        (String::from("instance_id"), JsonValue::string("instance-1")),
        (String::from("session_id"), JsonValue::string("session-1")),
        (String::from("lease_id"), JsonValue::string("lease-1")),
        (String::from("lease_epoch"), JsonValue::Number(1)),
        (String::from("generation"), JsonValue::Number(4)),
        (
            String::from("kind"),
            JsonValue::string("synchronization_response"),
        ),
        (
            String::from("players"),
            JsonValue::Array(vec![
                JsonValue::object([
                    (String::from("peer_id"), JsonValue::string("local-1")),
                    (String::from("role"), JsonValue::string("local")),
                ]),
                JsonValue::object([
                    (String::from("peer_id"), JsonValue::string("ally-1")),
                    (String::from("role"), JsonValue::string("ally")),
                ]),
            ]),
        ),
        (String::from("local_action"), JsonValue::Null),
        (String::from("shared_vote"), JsonValue::Null),
        (String::from("shared_effect"), JsonValue::Null),
        (String::from("ally_target"), JsonValue::Null),
        (
            String::from("synchronization"),
            JsonValue::object([
                (String::from("status"), JsonValue::string("synchronized")),
                (String::from("generation"), JsonValue::Number(4)),
                (String::from("peer_count"), JsonValue::Number(2)),
                (String::from("missing_peers"), JsonValue::Array(Vec::new())),
            ]),
        ),
    ])
}

fn call(arguments: &str) -> String {
    format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{{\"name\":\"{COOP_SYNCHRONIZATION_TOOL}\",\"arguments\":{{{arguments}}}}}}}"
    )
}

#[test]
fn co_op_sync_is_read_only_and_preserves_identity() {
    let gateway = FakeGateway {
        requests: Vec::new(),
        responses: VecDeque::from([Ok(GatewayResponse {
            status: 200,
            body: response(),
        })]),
    };
    let mut server = McpServer::with_catalog(gateway, ToolCatalog::coop_gameplay());
    let frame = call(
        "\"instance_id\":\"instance-1\",\"mcp_session_id\":\"session-1\",\"lease_id\":\"lease-1\",\"lease_epoch\":1,\"generation\":4",
    );
    let result = server.handle_frame(&frame);
    assert!(result.contains("synchronized"));
    assert_eq!(server.gateway().requests[0].method, GatewayMethod::Get);
    assert_eq!(
        server.gateway().requests[0].path,
        "/v1/instances/instance-1/coop/synchronization"
    );
}

#[test]
fn co_op_sync_rejects_extra_input_before_gateway() {
    let gateway = FakeGateway {
        requests: Vec::new(),
        responses: VecDeque::new(),
    };
    let mut server = McpServer::with_catalog(gateway, ToolCatalog::coop_gameplay());
    let frame = call(
        "\"instance_id\":\"instance-1\",\"mcp_session_id\":\"session-1\",\"lease_id\":\"lease-1\",\"lease_epoch\":1,\"generation\":4,\"action\":\"blocked\"",
    );
    assert!(server.handle_frame(&frame).contains("-32602"));
    assert!(server.gateway().requests.is_empty());
}

#[test]
fn co_op_peer_count_must_match_the_declared_peer_set() {
    let mut body = response();
    if let JsonValue::Object(root) = &mut body
        && let Some(JsonValue::Object(sync)) = root.get_mut("synchronization")
    {
        sync.insert(String::from("peer_count"), JsonValue::Number(3));
    }
    let gateway = FakeGateway {
        requests: Vec::new(),
        responses: VecDeque::from([Ok(GatewayResponse { status: 200, body })]),
    };
    let mut server = McpServer::with_catalog(gateway, ToolCatalog::coop_gameplay());
    let result = server.handle_frame(&call(
        "\"instance_id\":\"instance-1\",\"mcp_session_id\":\"session-1\",\"lease_id\":\"lease-1\",\"lease_epoch\":1,\"generation\":4",
    ));
    assert!(result.contains("\"isError\":true"), "{result}");
}

#[test]
fn synchronization_fixture_matches_the_pinned_protocol_prototype() -> Result<(), String> {
    let text = include_str!("../../../schemas/coop-gameplay-v1.schema.json");
    assert_eq!(
        hash::sha256_hex(text.as_bytes()),
        "85e0028c1ae20e49542791da165eeabaaea0cc2023626b5094b6660ebcc0cc81"
    );
    let schema: serde_json::Value =
        serde_json::from_str(text).map_err(|error| error.to_string())?;
    let validator = jsonschema::draft202012::options()
        .build(&schema)
        .map_err(|error| error.to_string())?;
    let fixture: serde_json::Value =
        serde_json::from_str(&response().to_json()).map_err(|error| error.to_string())?;
    assert!(validator.is_valid(&fixture));
    Ok(())
}
