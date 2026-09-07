// SPDX-License-Identifier: MIT

use serde_json::{Value, json};
use sts2_mcp_server::{
    BOOTSTRAP_TOOL, GatewayAdapter, GatewayError, GatewayMethod, GatewayRequest, GatewayResponse,
    HOST_FENCE_TOOL, JsonValue, LEASE_ACQUIRE_TOOL, LEASE_RENEW_TOOL, LEASE_REVOKE_TOOL, McpServer,
    OPERATION_DISPATCH_TOOL, OPERATION_INTENT_TOOL, OPERATION_LOOKUP_TOOL,
    OPERATION_RECONCILE_TOOL, ToolCatalog, parse_json, validate_recovery_request,
    validate_recovery_response,
};

macro_rules! valid_fixture {
    ($name:literal) => {
        include_str!(concat!(
            "../../../protocol-artifact/watchdog-recovery-v1/fixtures/valid/",
            $name
        ))
    };
}

const BOOTSTRAP_REQUEST: &str = valid_fixture!("bootstrap-request.json");
const BOOTSTRAP_RESPONSE: &str = valid_fixture!("bootstrap-response.json");
const HOST_FENCE_REQUEST: &str = valid_fixture!("host-fence-request.json");
const LEASE_ACQUIRE_REQUEST: &str = valid_fixture!("lease-acquire-request.json");
const LEASE_RENEW_REQUEST: &str = valid_fixture!("lease-renew-request.json");
const LEASE_REVOKE_REQUEST: &str = valid_fixture!("lease-revoke-request.json");
const LEASE_ACQUIRE_RESPONSE: &str = valid_fixture!("lease-acquire-response.json");
const OPERATION_INTENT_REQUEST: &str = valid_fixture!("operation-intent-request.json");
const OPERATION_DISPATCH_REQUEST: &str = valid_fixture!("operation-dispatch-request.json");
const OPERATION_LOOKUP_REQUEST: &str = valid_fixture!("operation-lookup-request.json");
const OPERATION_RECONCILE_REQUEST: &str = valid_fixture!("operation-reconcile-request.json");
const RECOVERY_FIXTURES: [(&str, &str); 9] = [
    (BOOTSTRAP_TOOL, BOOTSTRAP_REQUEST),
    (HOST_FENCE_TOOL, HOST_FENCE_REQUEST),
    (LEASE_ACQUIRE_TOOL, LEASE_ACQUIRE_REQUEST),
    (LEASE_RENEW_TOOL, LEASE_RENEW_REQUEST),
    (LEASE_REVOKE_TOOL, LEASE_REVOKE_REQUEST),
    (OPERATION_INTENT_TOOL, OPERATION_INTENT_REQUEST),
    (OPERATION_DISPATCH_TOOL, OPERATION_DISPATCH_REQUEST),
    (OPERATION_LOOKUP_TOOL, OPERATION_LOOKUP_REQUEST),
    (OPERATION_RECONCILE_TOOL, OPERATION_RECONCILE_REQUEST),
];
const LEASE_SCHEMA_FIXTURES: [(&str, &str); 4] = [
    (LEASE_RENEW_TOOL, LEASE_RENEW_REQUEST),
    (LEASE_REVOKE_TOOL, LEASE_REVOKE_REQUEST),
    (OPERATION_INTENT_TOOL, OPERATION_INTENT_REQUEST),
    (OPERATION_DISPATCH_TOOL, OPERATION_DISPATCH_REQUEST),
];

#[derive(Clone)]
enum GatewayOutcome {
    Error(GatewayError),
    Response(JsonValue),
}

struct RecordingGateway {
    requests: Vec<GatewayRequest>,
    outcome: GatewayOutcome,
}

impl RecordingGateway {
    fn error(error: GatewayError) -> Self {
        Self {
            requests: Vec::new(),
            outcome: GatewayOutcome::Error(error),
        }
    }

    fn response(body: JsonValue) -> Self {
        Self {
            requests: Vec::new(),
            outcome: GatewayOutcome::Response(body),
        }
    }
}

impl GatewayAdapter for RecordingGateway {
    fn forward(&mut self, request: GatewayRequest) -> Result<GatewayResponse, GatewayError> {
        self.requests.push(request);
        match self.outcome.clone() {
            GatewayOutcome::Error(error) => Err(error),
            GatewayOutcome::Response(mut body) => {
                let correlation = self
                    .requests
                    .last()
                    .map(|request| request.correlation.mcp_request_id.stable_text())
                    .ok_or(GatewayError::MalformedResponse)?;
                body.as_object_mut()
                    .ok_or(GatewayError::MalformedResponse)?
                    .insert("correlation_id".to_owned(), JsonValue::string(correlation));
                Ok(GatewayResponse { status: 200, body })
            }
        }
    }
}

fn server(gateway: RecordingGateway) -> McpServer<RecordingGateway> {
    McpServer::with_catalog_and_sessions(
        gateway,
        ToolCatalog::watchdog_recovery_v1(),
        "gateway-session",
        "mcp-session",
    )
    .with_recovery_identity(
        "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
        "harness",
        Some(String::from("recovery-proof")),
    )
}

fn fixture_payload(fixture: &str) -> Result<JsonValue, String> {
    parse_json(fixture)?
        .as_object()
        .and_then(|object| object.get("payload"))
        .cloned()
        .ok_or_else(|| String::from("fixture has no payload"))
}

fn call(tool: &str, payload: &JsonValue, session: &str) -> String {
    format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":\"call-1\",\"method\":\"tools/call\",\"params\":{{\"name\":\"{tool}\",\"arguments\":{{\"mcp_session_id\":\"{session}\",\"payload\":{}}}}}}}",
        payload.to_json()
    )
}

fn bootstrap_payload_with_policy(ttl: i64, renewal: i64) -> Result<JsonValue, String> {
    let mut payload = fixture_payload(BOOTSTRAP_REQUEST)?;
    let policy = payload
        .as_object_mut()
        .and_then(|object| object.get_mut("lease_policy"))
        .and_then(JsonValue::as_object_mut)
        .ok_or_else(|| String::from("bootstrap fixture is missing lease policy"))?;
    policy.insert("ttl_seconds".to_owned(), JsonValue::Number(ttl));
    policy.insert(
        "renewal_interval_seconds".to_owned(),
        JsonValue::Number(renewal),
    );
    Ok(payload)
}

fn listed_tool_schema(listed: &Value, tool: &str) -> Result<Value, String> {
    listed["result"]["tools"]
        .as_array()
        .and_then(|tools| {
            tools
                .iter()
                .find(|candidate| candidate.get("name").and_then(Value::as_str) == Some(tool))
        })
        .and_then(|descriptor| descriptor.get("inputSchema"))
        .cloned()
        .ok_or_else(|| format!("{tool} has no input schema"))
}

fn listed_catalog() -> Result<Value, Box<dyn std::error::Error>> {
    let mut server = server(RecordingGateway::error(GatewayError::Timeout));
    Ok(serde_json::from_str(&server.handle_frame(
        "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\",\"params\":{}}",
    ))?)
}

fn schema_arguments(fixture: &str) -> Result<Value, Box<dyn std::error::Error>> {
    Ok(
        json!({"mcp_session_id":"mcp-session","payload":serde_json::from_str::<Value>(fixture)?["payload"].clone()}),
    )
}

fn set_schema_value(
    value: &mut Value,
    path: &str,
    field: &str,
    replacement: Value,
) -> Result<(), String> {
    value
        .pointer_mut(path)
        .and_then(Value::as_object_mut)
        .ok_or_else(|| format!("schema fixture path is not an object: {path}"))?
        .insert(field.to_owned(), replacement);
    Ok(())
}

fn assert_schema_policy(
    validator: &jsonschema::Validator,
    baseline: &Value,
    path: &str,
    label: &str,
) -> Result<(), String> {
    for (ttl, renewal) in [(5, 4), (100, 99), (101, 100), (300, 100)] {
        let mut candidate = baseline.clone();
        for (field, value) in [("ttl_seconds", ttl), ("renewal_interval_seconds", renewal)] {
            set_schema_value(&mut candidate, path, field, json!(value))?;
        }
        assert!(
            validator.is_valid(&candidate),
            "{label} rejected valid {ttl}/{renewal} policy"
        );
        set_schema_value(&mut candidate, path, "renewal_interval_seconds", json!(ttl))?;
        assert!(
            !validator.is_valid(&candidate),
            "{label} accepted invalid {ttl}/{ttl} policy"
        );
    }
    Ok(())
}

fn assert_schema_case(
    listed: &Value,
    tool: &str,
    fixture: &str,
    policy: &str,
    identity: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let schema = listed_tool_schema(listed, tool)?;
    let validator = jsonschema::draft202012::options().build(&schema)?;
    let baseline = schema_arguments(fixture)?;
    assert_schema_policy(&validator, &baseline, policy, tool)?;
    let mut wrong_identity = baseline.clone();
    set_schema_value(
        &mut wrong_identity,
        identity,
        "instance_id",
        json!("not-a-uuid"),
    )?;
    assert!(
        !validator.is_valid(&wrong_identity),
        "{tool} accepted bad identity"
    );
    let mut unknown = baseline;
    set_schema_value(&mut unknown, policy, "unexpected", Value::Null)?;
    assert!(
        !validator.is_valid(&unknown),
        "{tool} accepted nested unknown field"
    );
    Ok(())
}

fn assert_closed_schema(
    listed: &Value,
    tool: &str,
    fixture: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let schema = listed_tool_schema(listed, tool)?;
    let validator = jsonschema::draft202012::options().build(&schema)?;
    let arguments = schema_arguments(fixture)?;
    assert!(
        validator.is_valid(&arguments),
        "{tool} rejected its valid fixture"
    );
    let mut unknown = arguments.clone();
    unknown["payload"]["unexpected"] = Value::Null;
    assert!(
        !validator.is_valid(&unknown),
        "{tool} advertised an open payload object"
    );
    let mut wrong_type = arguments;
    wrong_type["payload"] = json!("not-an-object");
    assert!(
        !validator.is_valid(&wrong_type),
        "{tool} omitted the payload object type"
    );
    Ok(())
}

#[test]
fn recovery_catalog_advertises_closed_bounded_request_payloads()
-> Result<(), Box<dyn std::error::Error>> {
    let listed = listed_catalog()?;
    assert_eq!(listed["result"]["tools"].as_array().map(Vec::len), Some(9));
    for (tool, fixture) in RECOVERY_FIXTURES {
        assert_closed_schema(&listed, tool, fixture)?;
    }
    assert_schema_case(
        &listed,
        BOOTSTRAP_TOOL,
        BOOTSTRAP_REQUEST,
        "/payload/lease_policy",
        "/payload",
    )?;
    for (tool, fixture) in LEASE_SCHEMA_FIXTURES {
        assert_schema_case(&listed, tool, fixture, "/payload/lease", "/payload/lease")?;
    }
    Ok(())
}

#[test]
fn recovery_mapping_uses_fixed_route_capability_and_no_retry_after_timeout() -> Result<(), String> {
    let payload = fixture_payload(OPERATION_DISPATCH_REQUEST)?;
    let mut server = server(RecordingGateway::error(GatewayError::Timeout));
    let response = server.handle_frame(&call(
        "watchdog.operation_dispatch",
        &payload,
        "mcp-session",
    ));
    assert!(
        response.contains("\\\"status\\\":\\\"UNKNOWN\\\""),
        "{response}"
    );
    assert!(
        response.contains("\\\"mutation_resubmitted\\\":false"),
        "{response}"
    );
    assert_eq!(server.gateway().requests.len(), 1);
    let request = &server.gateway().requests[0];
    assert_eq!(request.method, GatewayMethod::Post);
    assert_eq!(request.path, "/v1/recovery/operation/dispatch");
    assert_eq!(request.correlation.mcp_session_id, "mcp-session");
    assert_eq!(
        request
            .headers
            .get("x-sts2-recovery-capability")
            .map(String::as_str),
        Some("operation_submit")
    );
    let frame = request
        .body
        .as_ref()
        .and_then(JsonValue::as_object)
        .ok_or_else(|| String::from("recovery request body is not an object"))?;
    assert_eq!(frame.len(), 9);
    for name in ["instance_id", "session_id", "lease_id", "lease_epoch"] {
        assert!(frame.get(name).is_none());
    }
    Ok(())
}

#[test]
fn mixed_runtime_digest_and_foreign_mcp_session_are_rejected_before_gateway() -> Result<(), String>
{
    let mixed_payload = fixture_payload(OPERATION_INTENT_REQUEST)?;
    let mut mixed = server(RecordingGateway::error(GatewayError::Timeout));
    let mixed_response = mixed.handle_frame(&call(
        "watchdog.operation_intent",
        &mixed_payload,
        "mcp-session",
    ));
    assert!(mixed_response.contains("unsupported Runtime-v3 schema"));
    assert!(mixed.gateway().requests.is_empty());

    let bootstrap = fixture_payload(BOOTSTRAP_REQUEST)?;
    let mut foreign = server(RecordingGateway::error(GatewayError::Timeout));
    let foreign_response =
        foreign.handle_frame(&call("watchdog.bootstrap", &bootstrap, "foreign-session"));
    assert!(foreign_response.contains("MCP session identity does not match"));
    assert!(foreign.gateway().requests.is_empty());
    Ok(())
}

#[test]
fn request_decoder_rejects_lease_renewal_at_or_above_ttl_before_gateway() -> Result<(), String> {
    for (ttl, renewal) in [(30, 30), (30, 31)] {
        let payload = bootstrap_payload_with_policy(ttl, renewal)?;
        let mut server = server(RecordingGateway::error(GatewayError::Timeout));
        let response = server.handle_frame(&call(BOOTSTRAP_TOOL, &payload, "mcp-session"));
        assert!(
            response.contains("\"code\":-32602"),
            "{ttl}/{renewal}: {response}"
        );
        assert!(
            response.contains("recovery lease policy is outside bounds"),
            "{ttl}/{renewal}: {response}"
        );
        assert_eq!(server.gateway().requests.len(), 0);
    }
    Ok(())
}

#[test]
fn not_found_is_typed_without_claiming_non_execution_and_response_secrets_are_redacted()
-> Result<(), String> {
    let bootstrap = fixture_payload(BOOTSTRAP_REQUEST)?;
    let mut not_found = server(RecordingGateway::error(GatewayError::NotFound));
    let not_found_response =
        not_found.handle_frame(&call("watchdog.bootstrap", &bootstrap, "mcp-session"));
    assert!(not_found_response.contains("NOT_FOUND"));
    assert!(!not_found_response.contains("\"status\":\"UNKNOWN\""));

    let lease_request = fixture_payload(LEASE_ACQUIRE_REQUEST)?;
    let lease_response = parse_json(LEASE_ACQUIRE_RESPONSE)?;
    let mut redacted = server(RecordingGateway::response(lease_response));
    let response = redacted.handle_frame(&call(
        "watchdog.lease_acquire",
        &lease_request,
        "mcp-session",
    ));
    assert!(response.contains("\"isError\":false"), "{response}");
    assert!(!response.contains("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"));
    assert!(!response.contains("contract-test-proof"));
    Ok(())
}

#[test]
fn recovery_frame_identity_and_response_correlation_are_checked() -> Result<(), String> {
    let mut request = parse_json(BOOTSTRAP_REQUEST)?;
    let correlation = "66666666-6666-4666-8666-666666666666";
    assert!(
        validate_recovery_request(
            &request,
            "bootstrap",
            correlation,
            Some("22222222-2222-4222-8222-222222222222")
        )
        .is_ok()
    );
    assert!(
        validate_recovery_request(&request, "bootstrap", correlation, Some("instance-1")).is_err()
    );
    request
        .as_object_mut()
        .ok_or("bootstrap frame is not an object")?
        .insert(
            "correlation_id".to_owned(),
            JsonValue::string("66666666-6666-4666-8666-666666666667"),
        );
    assert!(validate_recovery_request(&request, "bootstrap", correlation, None).is_err());

    let response = parse_json(BOOTSTRAP_RESPONSE)?;
    let response_correlation = "66666666-6666-4666-8666-666666666666";
    assert!(validate_recovery_response(&response, "bootstrap", response_correlation).is_ok());
    assert!(
        validate_recovery_response(
            &response,
            "bootstrap",
            "66666666-6666-4666-8666-666666666667"
        )
        .is_err()
    );
    Ok(())
}
