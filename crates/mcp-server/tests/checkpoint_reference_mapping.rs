// SPDX-License-Identifier: MIT

use serde_json::{Value, json};
use sts2_mcp_server::{
    GatewayAdapter, GatewayError, GatewayMethod, GatewayRequest, GatewayResponse, McpServer,
    ToolCatalog, parse_json,
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
fn reference() -> Result<Value, String> {
    serde_json::from_str(include_str!(
        "../../../protocol-artifact/exact-checkpoint-reference-v1/golden/reference.json"
    ))
    .map_err(|e| e.to_string())
}
fn envelope() -> Result<Value, String> {
    Ok(
        json!({"schema":"ascension.checkpoint_reference_response.v1", "instance_id":"instance-1",
        "caller_id":"caller-1", "session_id":"session-1", "lease_id":"lease-1", "lease_epoch":7,
        "correlation_id":"request-1", "reference":reference()?}),
    )
}
fn arguments() -> Value {
    json!({"instance_id":"instance-1", "caller_id":"caller-1", "mcp_session_id":"mcp-1", "lease_id":"lease-1", "lease_epoch":7})
}
fn call(arguments: Value) -> String {
    json!({"jsonrpc":"2.0", "id":"request-1", "method":"tools/call",
        "params":{"name":"sts2.checkpoint_reference", "arguments":arguments}})
    .to_string()
}
fn server(body: Value) -> Result<McpServer<RecordingGateway>, String> {
    Ok(McpServer::with_catalog_and_sessions(
        RecordingGateway {
            requests: vec![],
            response: Ok(GatewayResponse {
                status: 200,
                body: parse_json(&body.to_string()).map_err(|e| format!("{e:?}"))?,
            }),
        },
        ToolCatalog::checkpoint_reference_v1(),
        "session-1",
        "mcp-1",
    ))
}
#[test]
fn real_mapping_projects_reference_and_sends_explicit_authority() -> Result<(), String> {
    let mut server = server(envelope()?)?;
    let result: Value = serde_json::from_str(&server.handle_frame(&call(arguments())))
        .map_err(|e| e.to_string())?;
    assert_eq!(result["result"]["isError"], false, "{result}");
    let projected: Value = serde_json::from_str(
        result["result"]["content"][0]["text"]
            .as_str()
            .ok_or("no text")?,
    )
    .map_err(|e| e.to_string())?;
    assert_eq!(projected, reference()?);
    let request = server.gateway().requests.first().ok_or("no request")?;
    assert_eq!(
        request.path,
        "/v1/instances/instance-1/checkpoint-reference"
    );
    assert_eq!(request.method, GatewayMethod::Get);
    assert!(request.body.is_none());
    for (header, value) in [
        ("x-sts2-caller-id", "caller-1"),
        ("x-sts2-session-id", "session-1"),
        ("x-mcp-session-id", "mcp-1"),
        ("x-sts2-lease-id", "lease-1"),
        ("x-sts2-lease-epoch", "7"),
    ] {
        assert_eq!(request.headers.get(header).map(String::as_str), Some(value));
    }
    Ok(())
}
#[test]
fn profile_is_opt_in_and_arguments_fail_before_transport() -> Result<(), String> {
    let mut server = server(envelope()?)?;
    let list = server.handle_frame(r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#);
    assert_eq!(list.matches("\"name\"").count(), 1);
    assert!(list.contains("sts2.checkpoint_reference"));
    for (field, bad) in [
        ("caller_id", json!("\r\nsecret")),
        ("mcp_session_id", json!("foreign")),
        ("lease_epoch", json!(-1)),
        ("instance_id", json!("../other")),
        ("path", json!("/secret")),
    ] {
        let mut args = arguments();
        args[field] = bad;
        let result: Value =
            serde_json::from_str(&server.handle_frame(&call(args))).map_err(|e| e.to_string())?;
        assert_eq!(result["error"]["code"], -32602, "{field}: {result}");
    }
    assert!(server.gateway().requests.is_empty());
    let mut legacy = McpServer::with_catalog(
        RecordingGateway {
            requests: vec![],
            response: Err(GatewayError::Unavailable),
        },
        ToolCatalog::runtime_v1(),
    );
    assert!(
        !legacy
            .handle_frame(r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#)
            .contains("sts2.checkpoint_reference")
    );
    Ok(())
}
#[test]
fn foreign_authority_privileged_payload_and_unsupported_version_are_redacted() -> Result<(), String>
{
    let mut cases = vec![];
    for field in [
        "schema",
        "instance_id",
        "caller_id",
        "session_id",
        "lease_id",
        "lease_epoch",
        "correlation_id",
        "extra",
    ] {
        let mut body = envelope()?;
        body[field] = json!("secret");
        cases.push(body);
    }
    for (field, value) in [
        ("exact_state_digest", json!("secret")),
        ("reference_version", json!("future")),
        ("handle", json!("secret")),
        ("restore_verified", json!(false)),
    ] {
        let mut body = envelope()?;
        body["reference"][field] = value;
        cases.push(body);
    }
    for body in cases {
        let mut server = server(body)?;
        let output = server.handle_frame(&call(arguments()));
        let result: Value = serde_json::from_str(&output).map_err(|e| e.to_string())?;
        assert_eq!(result["result"]["isError"], true, "{result}");
        assert!(!output.contains("secret"));
    }
    let mut server = McpServer::with_catalog_and_sessions(
        RecordingGateway {
            requests: vec![],
            response: Err(GatewayError::Unavailable),
        },
        ToolCatalog::checkpoint_reference_v1(),
        "session-1",
        "mcp-1",
    );
    let result = server.handle_frame(&call(arguments()));
    assert!(result.contains("\"isError\":true"));
    assert!(!result.contains("ckpt-h1:"));
    Ok(())
}
