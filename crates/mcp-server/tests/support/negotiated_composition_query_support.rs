// SPDX-License-Identifier: MIT
#![allow(clippy::expect_used, clippy::panic)]

use std::collections::VecDeque;

use serde_json::Value;
use sts2_mcp_server::{
    CapabilityLayer, CapabilityOwner, CapabilityScope, GatewayAdapter, GatewayError, GatewayMethod,
    GatewayRequest, GatewayResponse, JsonValue, ToolCatalog, parse_json,
};

pub struct FakeGateway {
    pub requests: Vec<GatewayRequest>,
    responses: VecDeque<Result<GatewayResponse, GatewayError>>,
}

impl FakeGateway {
    pub fn new(responses: impl IntoIterator<Item = Result<GatewayResponse, GatewayError>>) -> Self {
        Self {
            requests: Vec::new(),
            responses: responses.into_iter().collect(),
        }
    }
}

impl GatewayAdapter for FakeGateway {
    fn forward(&mut self, request: GatewayRequest) -> Result<GatewayResponse, GatewayError> {
        self.requests.push(request);
        self.responses
            .pop_front()
            .unwrap_or(Err(GatewayError::Unavailable))
    }
}

/// A gateway that answers the fixed game-information routes from the committed
/// goldens and, for the definition search/get kinds (which have no committed
/// success golden), derives a bounded empty final page that echoes the exact
/// request query. This is a synthetic owned producer.
pub enum Reply {
    Capabilities,
    EmptyPage,
    ListPageOne,
    ListPageTwo,
    Detail,
}

pub struct ScriptedGateway {
    script: VecDeque<Reply>,
    capabilities: JsonValue,
    page_one: JsonValue,
    page_two: JsonValue,
    detail: JsonValue,
    pub requests: Vec<GatewayRequest>,
}

impl ScriptedGateway {
    pub fn new(script: impl IntoIterator<Item = Reply>) -> Self {
        Self {
            script: script.into_iter().collect(),
            capabilities: golden("capabilities-response"),
            page_one: golden("static-page-1-response"),
            page_two: golden("static-page-2-response"),
            detail: golden("live-detail-response"),
            requests: Vec::new(),
        }
    }
}

impl GatewayAdapter for ScriptedGateway {
    fn forward(&mut self, request: GatewayRequest) -> Result<GatewayResponse, GatewayError> {
        let reply = self.script.pop_front().ok_or(GatewayError::Unavailable)?;
        let body = match reply {
            Reply::Capabilities => {
                let mut body = self.capabilities.clone();
                set_correlation(&mut body, "capabilities");
                body
            }
            Reply::EmptyPage => {
                let request_body = request.body.as_ref().ok_or(GatewayError::Unavailable)?;
                empty_query_response(request_body)
            }
            Reply::ListPageOne => {
                let mut body = self.page_one.clone();
                set_correlation_from_request(&mut body, &request)?;
                body
            }
            Reply::ListPageTwo => {
                let mut body = self.page_two.clone();
                set_correlation_from_request(&mut body, &request)?;
                body
            }
            Reply::Detail => {
                let mut body = self.detail.clone();
                set_correlation_from_request(&mut body, &request)?;
                body
            }
        };
        self.requests.push(request);
        Ok(GatewayResponse { status: 200, body })
    }
}

pub fn golden(name: &str) -> JsonValue {
    let text = match name {
        "capabilities-response" => include_str!(
            "../../../../protocol-artifact/game-information-query-v1/golden/capabilities-response.json"
        ),
        "error-stale-cursor" => include_str!(
            "../../../../protocol-artifact/game-information-query-v1/golden/error-stale-cursor.json"
        ),
        "live-detail-request" => include_str!(
            "../../../../protocol-artifact/game-information-query-v1/golden/live-detail-request.json"
        ),
        "live-detail-response" => include_str!(
            "../../../../protocol-artifact/game-information-query-v1/golden/live-detail-response.json"
        ),
        "static-page-1-request" => include_str!(
            "../../../../protocol-artifact/game-information-query-v1/golden/static-page-1-request.json"
        ),
        "static-page-1-response" => include_str!(
            "../../../../protocol-artifact/game-information-query-v1/golden/static-page-1-response.json"
        ),
        "static-page-2-request" => include_str!(
            "../../../../protocol-artifact/game-information-query-v1/golden/static-page-2-request.json"
        ),
        "static-page-2-response" => include_str!(
            "../../../../protocol-artifact/game-information-query-v1/golden/static-page-2-response.json"
        ),
        _ => panic!("unsupported golden {name}"),
    };
    parse_json(text).expect("accepted golden JSON")
}

pub fn set_correlation(body: &mut JsonValue, correlation: &str) {
    if let JsonValue::Object(object) = body {
        object.insert(
            String::from("correlation_id"),
            JsonValue::string(correlation),
        );
    }
}

pub fn set_correlation_from_request(
    body: &mut JsonValue,
    request: &GatewayRequest,
) -> Result<(), GatewayError> {
    let request_body = request.body.as_ref().ok_or(GatewayError::Unavailable)?;
    let parsed: Value =
        serde_json::from_str(&request_body.to_json()).map_err(|_| GatewayError::Unavailable)?;
    let correlation = parsed
        .get("correlation_id")
        .and_then(Value::as_str)
        .ok_or(GatewayError::Unavailable)?;
    set_correlation(body, correlation);
    Ok(())
}

pub fn empty_query_response(request_body: &JsonValue) -> JsonValue {
    let envelope: Value =
        serde_json::from_str(&request_body.to_json()).expect("query envelope JSON");
    let query = envelope
        .get("query")
        .cloned()
        .expect("query envelope carries a query");
    let limits = query.get("limits").cloned().expect("query carries limits");
    let parent = query
        .get("parent_observation")
        .cloned()
        .expect("query carries parent observation");
    let mut page = serde_json::json!({
        "items": [],
        "next_cursor": null,
        "cursor_binding": null,
        "final_page": true,
        "total_count_known": false,
        "total_count": null,
        "coverage": "complete",
        "ordering": {
            "key": "namespaced_id",
            "direction": "ascending",
            "algorithm": "identity_bytes",
            "deterministic": true
        },
        "limits": limits,
    });
    let page_bytes = serde_json::to_string(&page).expect("page JSON").len() as i64;
    page["accounting"] = serde_json::json!({
        "item_count": 0,
        "item_bytes": 0,
        "payload_bytes": 2,
        "page_bytes": page_bytes,
        "text_bytes": 0
    });
    let response = serde_json::json!({
        "protocol_version": envelope.get("protocol_version").cloned().expect("protocol version"),
        "schema_digest": envelope.get("schema_digest").cloned().expect("schema digest"),
        "provenance": envelope.get("provenance").cloned().expect("provenance"),
        "correlation_id": envelope.get("correlation_id").cloned().expect("correlation"),
        "kind": "query_response",
        "query": query,
        "result": {
            "page": page,
            "result_generation": null,
            "parent_observation": parent,
            "read_only": true
        },
        "capabilities": null,
        "error": null
    });
    parse_json(&response.to_string()).expect("empty query response JSON")
}

pub fn frame(id: &str, name: &str, arguments: JsonValue) -> String {
    JsonValue::object([
        (String::from("jsonrpc"), JsonValue::string("2.0")),
        (String::from("id"), JsonValue::string(id)),
        (String::from("method"), JsonValue::string("tools/call")),
        (
            String::from("params"),
            JsonValue::object([
                (String::from("name"), JsonValue::string(name)),
                (String::from("arguments"), arguments),
            ]),
        ),
    ])
    .to_json()
}

pub fn context() -> JsonValue {
    JsonValue::object([
        (String::from("instance_id"), JsonValue::string("instance-1")),
        (
            String::from("mcp_session_id"),
            JsonValue::string("mcp-session-1"),
        ),
        (String::from("lease_id"), JsonValue::string("lease-1")),
        (String::from("lease_epoch"), JsonValue::Number(7)),
    ])
}

pub fn static_arguments(cursor: Option<&str>) -> JsonValue {
    let mut arguments = context();
    let JsonValue::Object(object) = &mut arguments else {
        return JsonValue::Null;
    };
    for (key, value) in [
        ("content_manifest_id", JsonValue::string("content-1")),
        ("locale", JsonValue::string("en-US")),
        ("visibility_scope", JsonValue::string("public")),
        ("entity_kind", JsonValue::string("card")),
        ("projection", JsonValue::string("summary")),
        ("detail_level", JsonValue::string("summary")),
        (
            "fields",
            JsonValue::Array(vec![JsonValue::string("display_name")]),
        ),
        ("page_items", JsonValue::Number(2)),
        ("item_bytes", JsonValue::Number(4_096)),
        ("page_bytes", JsonValue::Number(65_536)),
        ("text_bytes", JsonValue::Number(1_024)),
    ] {
        object.insert(String::from(key), value);
    }
    object.insert(
        String::from("cursor"),
        cursor.map_or(JsonValue::Null, JsonValue::string),
    );
    arguments
}

pub fn definition_ref() -> JsonValue {
    JsonValue::object([
        (
            String::from("content_manifest_id"),
            JsonValue::string("content-1"),
        ),
        (String::from("entity_kind"), JsonValue::string("card")),
        (
            String::from("namespaced_id"),
            JsonValue::string("ironclad:strike"),
        ),
        (String::from("variant"), JsonValue::Null),
    ])
}

pub fn live_arguments() -> JsonValue {
    parse_json(
        r#"{
          "instance_id":"instance-1","mcp_session_id":"mcp-session-1",
          "lease_id":"lease-1","lease_epoch":7,
          "content_manifest_id":"content-1","locale":"en-US",
          "visibility_scope":"player","entity_kind":"card",
          "projection":"full","detail_level":"full","fields":["amount","cost","description","display_name"],"page_items":1,
          "item_bytes":4096,"page_bytes":65536,"text_bytes":4096,
          "definition_ref":{"content_manifest_id":"content-1","entity_kind":"card","namespaced_id":"ironclad:strike","variant":null},
          "instance_ref":{"instance_id":"instance-1","run_id":"run-1","epoch":7,"entity_kind":"card","entity_id":"card-17"},
          "snapshot_ref":{"snapshot_id":"snapshot-42","instance_ref":{"instance_id":"instance-1","run_id":"run-1","epoch":7,"entity_kind":"card","entity_id":"card-17"},"state_generation":42},
          "parent_observation":{"instance_ref":{"instance_id":"instance-1","run_id":"run-1","epoch":7,"entity_kind":"card","entity_id":"card-17"},"snapshot_ref":{"snapshot_id":"snapshot-42","instance_ref":{"instance_id":"instance-1","run_id":"run-1","epoch":7,"entity_kind":"card","entity_id":"card-17"},"state_generation":42},"state_generation":42}
        }"#,
    )
    .expect("live arguments JSON")
}

pub fn wire_value(output: &str) -> Value {
    serde_json::from_str(output).expect("MCP response JSON")
}

pub fn successful_response(
    body: &mut JsonValue,
    correlation: &str,
) -> Result<GatewayResponse, GatewayError> {
    set_correlation(body, correlation);
    Ok(GatewayResponse {
        status: 200,
        body: body.clone(),
    })
}

pub fn composed_catalog(scope: CapabilityScope) -> Result<ToolCatalog, String> {
    let profiles = [
        ToolCatalog::runtime_map_v1(),
        ToolCatalog::game_information(),
    ];
    let gateway = CapabilityLayer::from_catalogs(CapabilityOwner::Gateway, &profiles)
        .map_err(|error| error.to_string())?;
    let producer = CapabilityLayer::from_catalogs(CapabilityOwner::Producer, &profiles)
        .map_err(|error| error.to_string())?;
    ToolCatalog::compose_profiles(&profiles, gateway, producer, scope)
        .map_err(|error| error.to_string())
}

pub fn assert_query_route(request: &GatewayRequest, index: usize) {
    assert_eq!(request.method, GatewayMethod::Post, "query {index} method");
    assert_eq!(
        request.path, "/v1/instances/instance-1/game-information/query",
        "query {index} path"
    );
    assert_eq!(
        request
            .headers
            .get("x-sts2-instance-id")
            .map(String::as_str),
        Some("instance-1"),
        "query {index} instance header"
    );
    assert_eq!(
        request.headers.get("x-sts2-lease-id").map(String::as_str),
        Some("lease-1"),
        "query {index} lease header"
    );
    assert_eq!(
        request.headers.get("x-mcp-session-id").map(String::as_str),
        Some("mcp-session-1"),
        "query {index} mcp session header"
    );
}

pub fn assert_typed_failure(wire: &Value, label: &str) {
    let structured_error = wire.get("error").is_some()
        || wire["result"]["isError"] == Value::Bool(true)
        || wire["result"]["structuredContent"]["error"].is_object();
    assert!(structured_error, "{label} was not a typed error: {wire}");
}
