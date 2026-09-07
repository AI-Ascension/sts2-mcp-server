// SPDX-License-Identifier: MIT

use serde_json::Value;
use sts2_mcp_server::{
    GatewayAdapter, GatewayError, GatewayMethod, GatewayRequest, GatewayResponse, JsonValue,
    MAP_SNAPSHOT_TOOL, McpServer, ToolCatalog, parse_json,
};

struct RecordingGateway {
    requests: Vec<GatewayRequest>,
    response: Result<GatewayResponse, GatewayError>,
}

impl GatewayAdapter for RecordingGateway {
    fn forward(&mut self, request: GatewayRequest) -> Result<GatewayResponse, GatewayError> {
        self.requests.push(request);
        self.response.clone()
    }
}

fn response_body() -> JsonValue {
    parse_json(include_str!(
        "../../../protocol-artifact/runtime-map-v1/golden/snapshot-response.json"
    ))
    .unwrap_or(JsonValue::Null)
}

fn call(arguments: &str) -> String {
    format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":\"corr-42\",\"method\":\"tools/call\",\"params\":{{\"name\":\"{MAP_SNAPSHOT_TOOL}\",\"arguments\":{{{arguments}}}}}}}"
    )
}

fn arguments() -> &'static str {
    "\"instance_id\":\"instance-1\",\"mcp_session_id\":\"mcp-1\",\
     \"lease_id\":\"lease-1\",\"lease_epoch\":7,\"generation\":42"
}

#[test]
fn map_profile_advertises_the_six_gameplay_tools_plus_one_read_tool() {
    let catalog = ToolCatalog::runtime_map_v1();
    assert_eq!(catalog.revision, "runtime-map-v1-mcp");
    let mut server = McpServer::with_catalog(
        RecordingGateway {
            requests: Vec::new(),
            response: Err(GatewayError::Unavailable),
        },
        catalog,
    );
    let response = server
        .handle_frame("{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\",\"params\":{}}");
    assert!(response.contains(MAP_SNAPSHOT_TOOL));
    assert!(response.contains("sts2.dispatch_action"));
    assert_eq!(response.matches("\"name\"").count(), 7);
}

#[test]
fn map_snapshot_maps_one_fenced_read_route_and_projects_the_complete_graph() -> Result<(), String> {
    let mut server = McpServer::with_catalog_and_sessions(
        RecordingGateway {
            requests: Vec::new(),
            response: Ok(GatewayResponse {
                status: 200,
                body: response_body(),
            }),
        },
        ToolCatalog::runtime_map_v1(),
        "session-1",
        "mcp-1",
    );
    let output = server.handle_frame(&call(arguments()));
    let wire: Value = serde_json::from_str(&output).map_err(|error| error.to_string())?;
    assert_eq!(wire["result"]["isError"], false, "{output}");
    let projected: Value = serde_json::from_str(
        wire["result"]["content"][0]["text"]
            .as_str()
            .ok_or("map projection is missing")?,
    )
    .map_err(|error| error.to_string())?;
    assert_eq!(projected["kind"], "snapshot_response");
    assert_eq!(projected["generation"], 42);
    assert_eq!(
        projected["snapshot"]["nodes"].as_array().map(Vec::len),
        Some(4)
    );
    assert_eq!(
        projected["snapshot"]["edges"].as_array().map(Vec::len),
        Some(4)
    );
    assert_eq!(
        projected["snapshot"]["bindings"][0]["graph_node_id"],
        "map:1:1:0"
    );

    assert_eq!(server.gateway().requests.len(), 1);
    let request = &server.gateway().requests[0];
    assert_eq!(request.method, GatewayMethod::Get);
    assert_eq!(request.path, "/v1/instances/instance-1/map-snapshot");
    assert!(request.body.is_none());
    assert_eq!(
        request.headers.get("x-mcp-session-id").map(String::as_str),
        Some("mcp-1")
    );
    for (name, expected) in [
        ("x-sts2-instance-id", "instance-1"),
        ("x-sts2-session-id", "session-1"),
        ("x-sts2-lease-id", "lease-1"),
        ("x-sts2-lease-epoch", "7"),
    ] {
        assert_eq!(
            request.headers.get(name).map(String::as_str),
            Some(expected),
            "map authority header {name}"
        );
    }
    Ok(())
}

#[test]
fn map_snapshot_rejects_stale_generation_unknown_fields_and_foreign_identity() {
    let mutations: [fn(&mut JsonValue); 3] = [
        |body: &mut JsonValue| {
            if let JsonValue::Object(root) = body {
                root.insert(String::from("generation"), JsonValue::Number(41));
            }
        },
        |body: &mut JsonValue| {
            if let JsonValue::Object(root) = body {
                root.insert(
                    String::from("private_host_state"),
                    JsonValue::string("secret"),
                );
            }
        },
        |body: &mut JsonValue| {
            if let JsonValue::Object(root) = body {
                root.insert(
                    String::from("instance_id"),
                    JsonValue::string("other-instance"),
                );
            }
        },
    ];
    for mutation in mutations {
        let mut body = response_body();
        mutation(&mut body);
        let mut server = McpServer::with_catalog_and_sessions(
            RecordingGateway {
                requests: Vec::new(),
                response: Ok(GatewayResponse { status: 200, body }),
            },
            ToolCatalog::runtime_map_v1(),
            "session-1",
            "mcp-1",
        );
        let output = server.handle_frame(&call(arguments()));
        assert!(output.contains("\"isError\":true"), "{output}");
    }
}

#[test]
fn map_snapshot_rejects_unknown_input_before_gateway() {
    let mut server = McpServer::with_catalog(
        RecordingGateway {
            requests: Vec::new(),
            response: Err(GatewayError::Unavailable),
        },
        ToolCatalog::runtime_map_v1(),
    );
    let output = server.handle_frame(&call(
        "\"instance_id\":\"instance-1\",\"mcp_session_id\":\"mcp-1\",\
         \"lease_id\":\"lease-1\",\"lease_epoch\":7,\"generation\":42,\
         \"unexpected\":true",
    ));
    assert!(output.contains("\"code\":-32602"));
    assert!(server.gateway().requests.is_empty());
}
