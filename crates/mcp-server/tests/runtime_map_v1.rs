// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

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

fn mutate_snapshot(
    body: &mut JsonValue,
    mutation: impl FnOnce(&mut BTreeMap<String, JsonValue>),
) -> bool {
    let JsonValue::Object(root) = body else {
        return false;
    };
    let Some(JsonValue::Object(snapshot)) = root.get_mut("snapshot") else {
        return false;
    };
    mutation(snapshot);
    true
}

fn map_output(body: JsonValue) -> Value {
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
    serde_json::from_str(&output).unwrap_or(Value::Null)
}

fn assert_success(wire: &Value) {
    assert_eq!(wire["result"]["isError"], false, "{wire}");
}

fn assert_tool_error(wire: &Value) {
    assert_eq!(wire["result"]["isError"], true, "{wire}");
}

fn set_snapshot_text(body: &mut JsonValue, field: &str, value: String) -> bool {
    mutate_snapshot(body, |snapshot| {
        snapshot.insert(field.to_owned(), JsonValue::string(value));
    })
}

fn set_reason(body: &mut JsonValue, value: String) -> bool {
    mutate_snapshot(body, |snapshot| {
        snapshot.insert(
            String::from("completeness"),
            JsonValue::string("incomplete"),
        );
        snapshot.insert(String::from("reason"), JsonValue::string(value));
    })
}

fn binding_field(
    snapshot: &BTreeMap<String, JsonValue>,
    index: usize,
    field: &str,
) -> Option<JsonValue> {
    let Some(JsonValue::Array(bindings)) = snapshot.get("bindings") else {
        return None;
    };
    let Some(JsonValue::Object(binding)) = bindings.get(index) else {
        return None;
    };
    binding.get(field).cloned()
}

fn set_binding_option(
    snapshot: &mut BTreeMap<String, JsonValue>,
    index: usize,
    value: JsonValue,
) -> bool {
    let Some(JsonValue::Array(bindings)) = snapshot.get_mut("bindings") else {
        return false;
    };
    let Some(JsonValue::Object(binding)) = bindings.get_mut(index) else {
        return false;
    };
    let Some(JsonValue::Object(action)) = binding.get_mut("action") else {
        return false;
    };
    action.insert(String::from("node_id"), value);
    true
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

#[test]
fn serialized_boundary_accepts_multibyte_build_and_mod_text_at_128_utf8_bytes() {
    let valid = "é".repeat(64);
    assert_eq!(valid.len(), 128);
    for field in ["game_build", "mod_version"] {
        let mut body = response_body();
        assert!(set_snapshot_text(&mut body, field, valid.clone()));
        assert_success(&map_output(body));

        let over = format!("{valid}a");
        assert_eq!(over.len(), 129);
        let mut body = response_body();
        assert!(set_snapshot_text(&mut body, field, over));
        assert_tool_error(&map_output(body));
    }
}

#[test]
fn serialized_boundary_accepts_multibyte_reason_at_256_utf8_bytes_and_rejects_overflow() {
    let valid = format!("{}a", "界".repeat(85));
    assert_eq!(valid.len(), 256);
    let mut body = response_body();
    assert!(set_reason(&mut body, valid.clone()));
    assert_success(&map_output(body));

    let over = format!("{valid}a");
    assert_eq!(over.len(), 257);
    let mut body = response_body();
    assert!(set_reason(&mut body, over));
    assert_tool_error(&map_output(body));
}

#[test]
fn serialized_boundary_rejects_c0_del_and_c1_controls_in_text_fields() {
    for field in ["game_build", "mod_version"] {
        for control in ['\0', '\u{7f}', '\u{85}'] {
            let mut body = response_body();
            assert!(set_snapshot_text(
                &mut body,
                field,
                format!("valid{control}text")
            ));
            assert_tool_error(&map_output(body));
        }
    }
    for control in ['\0', '\u{7f}', '\u{85}'] {
        let mut body = response_body();
        assert!(set_reason(&mut body, format!("valid{control}text")));
        assert_tool_error(&map_output(body));
    }
}

#[test]
fn serialized_boundary_accepts_cross_namespace_option_identity_equalities() {
    let mut graph_equal = response_body();
    assert!(mutate_snapshot(&mut graph_equal, |snapshot| {
        let graph_id = binding_field(snapshot, 0, "graph_node_id");
        assert!(graph_id.is_some());
        if let Some(graph_id) = graph_id {
            assert!(set_binding_option(snapshot, 0, graph_id));
        }
    }));
    let graph_wire = map_output(graph_equal);
    assert_success(&graph_wire);

    let mut host_equal = response_body();
    assert!(mutate_snapshot(&mut host_equal, |snapshot| {
        let host_id = binding_field(snapshot, 0, "host_action_id");
        assert!(host_id.is_some());
        if let Some(host_id) = host_id {
            assert!(set_binding_option(snapshot, 0, host_id));
        }
    }));
    let host_wire = map_output(host_equal);
    assert_success(&host_wire);
}
