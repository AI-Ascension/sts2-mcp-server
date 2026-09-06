// SPDX-License-Identifier: MIT

use serde_json::{Value, json};
use sts2_mcp_server::{
    GatewayAdapter, GatewayError, GatewayMethod, GatewayRequest, GatewayResponse, JsonValue,
    McpServer, ToolCatalog, parse_json,
};

const GOLDEN: &str =
    include_str!("../../../protocol-artifact/coop-synchronization-v1/golden/synchronized.json");
const CASES: &str =
    include_str!("../../../protocol-artifact/coop-synchronization-v1/conformance.json");
const SCHEMA: &str = include_str!("../../../protocol-artifact/coop-synchronization-v1/schema.json");

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

fn server(wire: &str) -> Result<McpServer<Gateway>, String> {
    let gateway = Gateway {
        requests: Vec::new(),
        body: parse_json(wire).map_err(|error| error.to_string())?,
    };
    Ok(McpServer::with_catalog_and_sessions(
        gateway,
        ToolCatalog::coop_synchronization(),
        "session-1",
        "mcp-session-1",
    ))
}

fn arguments() -> Value {
    json!({"instance_id":"instance-1","mcp_session_id":"mcp-session-1","lease_id":"lease-1","lease_epoch":1})
}

fn call(arguments: Value) -> String {
    json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{
        "name":"sts2.coop_synchronization","arguments":arguments,
    }})
    .to_string()
}

#[test]
fn executable_catalog_is_one_read_only_tool_with_complete_response()
-> Result<(), Box<dyn std::error::Error>> {
    let mut server = server(GOLDEN)?;
    let result: Value = serde_json::from_str(&server.handle_frame(&call(arguments())))?;
    assert_eq!(result["result"]["isError"], false);
    let body: Value = serde_json::from_str(
        result["result"]["content"][0]["text"]
            .as_str()
            .ok_or("text absent")?,
    )?;
    assert_eq!(body, serde_json::from_str::<Value>(GOLDEN)?);
    let request = &server.gateway().requests[0];
    assert_eq!(request.method, GatewayMethod::Get);
    assert_eq!(
        request.path,
        "/v1/instances/instance-1/coop/synchronization"
    );
    assert!(request.body.is_none());
    for (key, value) in [
        ("x-sts2-instance-id", "instance-1"),
        ("x-sts2-session-id", "session-1"),
        ("x-mcp-session-id", "mcp-session-1"),
        ("x-sts2-lease-id", "lease-1"),
        ("x-sts2-lease-epoch", "1"),
    ] {
        assert_eq!(request.headers.get(key).map(String::as_str), Some(value));
    }
    let catalog = ToolCatalog::coop_synchronization();
    let listed: Value = serde_json::from_str(
        &server.handle_frame(r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#),
    )?;
    assert_eq!(
        listed["result"]["tools"]
            .as_array()
            .ok_or("tools absent")?
            .len(),
        1
    );
    assert_eq!(
        listed["result"]["tools"][0]["name"],
        "sts2.coop_synchronization"
    );
    assert_eq!(catalog.max_frame_bytes(), 16 * 1024);
    Ok(())
}

#[test]
fn full_shared_conformance_vectors_are_consumed() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        SCHEMA,
        include_str!("../../../schemas/coop-synchronization-v1.schema.json")
    );
    assert_eq!(
        CASES,
        include_str!("../../../conformance/cases/coop-synchronization-v1.json")
    );
    let schema: Value = serde_json::from_str(SCHEMA)?;
    let validator = jsonschema::draft202012::options().build(&schema)?;
    let cases: Value = serde_json::from_str(CASES)?;
    for case in cases["cases"].as_array().ok_or("cases absent")? {
        let mut server = server(&case["value"].to_string())?;
        let result: Value = serde_json::from_str(&server.handle_frame(&call(arguments())))?;
        assert_eq!(
            result["result"]["isError"],
            case["valid"] != true,
            "{}",
            case["name"]
        );
        assert_eq!(
            validator.is_valid(&case["value"]),
            case["schema_valid"] == true,
            "{}",
            case["name"]
        );
    }
    Ok(())
}

#[test]
fn supplied_identity_and_unknown_input_fail_before_gateway()
-> Result<(), Box<dyn std::error::Error>> {
    for (field, value) in [
        ("mcp_session_id", json!("foreign")),
        ("generation", json!(4)),
        ("lease_epoch", json!(-1)),
        ("lease_epoch", json!(9007199254740992_u64)),
        ("instance_id", json!("../foreign")),
        ("lease_id", json!("bad\nheader")),
        ("action", json!("end_turn")),
    ] {
        let mut args = arguments();
        args[field] = value;
        let mut server = server(GOLDEN)?;
        let result = server.handle_frame(&call(args));
        assert!(result.contains("-32602"), "{field}: {result}");
        assert!(server.gateway().requests.is_empty());
    }
    for field in ["instance_id", "mcp_session_id", "lease_id", "lease_epoch"] {
        let mut args = arguments();
        args.as_object_mut()
            .ok_or("arguments absent")?
            .remove(field);
        let mut server = server(GOLDEN)?;
        assert!(server.handle_frame(&call(args)).contains("-32602"));
        assert!(server.gateway().requests.is_empty());
    }
    Ok(())
}

#[test]
fn response_scope_mismatch_and_unknown_fields_fail_closed() -> Result<(), Box<dyn std::error::Error>>
{
    let golden: Value = serde_json::from_str(GOLDEN)?;
    for field in ["instance_id", "session_id", "lease_id", "correlation_id"] {
        let mut value = golden.clone();
        value[field] = json!("foreign");
        let result = server(&value.to_string())?.handle_frame(&call(arguments()));
        assert!(result.contains("\"isError\":true"), "{field}: {result}");
    }
    for path in ["", "/provenance", "/players/0", "/synchronization"] {
        let mut value = golden.clone();
        value
            .pointer_mut(path)
            .and_then(Value::as_object_mut)
            .ok_or("object absent")?
            .insert("extension".to_owned(), json!(null));
        assert!(
            server(&value.to_string())?
                .handle_frame(&call(arguments()))
                .contains("\"isError\":true")
        );
    }
    Ok(())
}

#[test]
fn duplicate_and_fractional_wire_members_are_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let wire = serde_json::from_str::<Value>(GOLDEN)?.to_string();
    for malformed in [
        wire.replacen("\"generation\":4", "\"generation\":4,\"generation\":4", 1),
        wire.replacen("\"generation\":4", "\"generation\":4.0", 1),
        wire.replacen(
            "\"role\":\"local\"",
            "\"role\":\"local\",\"role\":\"local\"",
            1,
        ),
    ] {
        assert_ne!(malformed, wire);
        assert!(parse_json(&malformed).is_err());
    }
    Ok(())
}
