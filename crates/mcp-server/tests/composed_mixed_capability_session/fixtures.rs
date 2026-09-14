// SPDX-License-Identifier: MIT

//! Synthetic producers, routes, and golden fixtures for the composed mixed-capability
//! session acceptance. Every value is owned by this repository: no game host, gateway
//! process, or provider is executed, and the recordings are plain in-memory adapters.

use std::collections::VecDeque;

use sts2_mcp_server::{
    CapabilityLayer, CapabilityOwner, CapabilityScope, GatewayAdapter, GatewayError,
    GatewayRequest, GatewayResponse, JsonValue, NegotiationError, ToolCatalog, parse_json,
};

pub(crate) const INSTANCE: &str = "instance-1";
pub(crate) const GATEWAY_SESSION: &str = "session-1";
pub(crate) const MCP_SESSION: &str = "mcp-session-1";
pub(crate) const LEASE: &str = "lease-1";
pub(crate) const LEASE_EPOCH: i64 = 7;
pub(crate) const GENERATION: i64 = 4;
pub(crate) const SNAPSHOT: &str = "snapshot-42";
pub(crate) const FRESH_SNAPSHOT: &str = "snapshot-43";

const V3_STATE_RESPONSE: &str =
    include_str!("../../../../protocol-artifact/runtime-v3-gameplay/golden/state-response.json");
const MAP_SNAPSHOT_RESPONSE: &str =
    include_str!("../../../../protocol-artifact/runtime-map-v1/golden/snapshot-response.json");
const SEARCH_RESPONSE: &str = include_str!(
    "../../../../protocol-artifact/game-information-query-v1/golden/static-page-2-response.json"
);
const DETAIL_RESPONSE: &str = include_str!(
    "../../../../protocol-artifact/game-information-query-v1/golden/live-detail-response.json"
);

/// The live detail query the adapter builds for `SNAPSHOT`; snapshot identity is
/// substituted so one template covers both the invalidated and refreshed cases.
const DETAIL_ARGUMENTS: &str = r#"{
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
}"#;

pub(crate) struct RecordingGateway {
    pub(crate) requests: Vec<GatewayRequest>,
    responses: VecDeque<Result<GatewayResponse, GatewayError>>,
}

impl RecordingGateway {
    pub(crate) fn new(
        responses: impl IntoIterator<Item = Result<GatewayResponse, GatewayError>>,
    ) -> Self {
        Self {
            requests: Vec::new(),
            responses: responses.into_iter().collect(),
        }
    }

    pub(crate) fn forwarded(&self) -> usize {
        self.requests.len()
    }
}

impl GatewayAdapter for RecordingGateway {
    fn forward(&mut self, request: GatewayRequest) -> Result<GatewayResponse, GatewayError> {
        self.requests.push(request);
        self.responses
            .pop_front()
            .unwrap_or(Err(GatewayError::Unavailable))
    }
}

pub(crate) fn ok(body: JsonValue) -> Result<GatewayResponse, GatewayError> {
    Ok(GatewayResponse { status: 200, body })
}

/// The two profiles that jointly cover ordinary gameplay, maps, and content lookups.
pub(crate) fn gameplay_lookup_profiles() -> [ToolCatalog; 2] {
    [
        ToolCatalog::runtime_map_v1(),
        ToolCatalog::game_information_query_v1(),
    ]
}

pub(crate) fn gateway_layer(profiles: &[ToolCatalog]) -> Result<CapabilityLayer, NegotiationError> {
    CapabilityLayer::from_catalogs(CapabilityOwner::Gateway, profiles)
}

pub(crate) fn producer_layer(
    profiles: &[ToolCatalog],
) -> Result<CapabilityLayer, NegotiationError> {
    CapabilityLayer::from_catalogs(CapabilityOwner::Producer, profiles)
}

pub(crate) fn composed_catalog(
    profiles: &[ToolCatalog],
    caller_scope: CapabilityScope,
) -> Result<ToolCatalog, NegotiationError> {
    ToolCatalog::compose_profiles(
        profiles,
        gateway_layer(profiles)?,
        producer_layer(profiles)?,
        caller_scope,
    )
}

pub(crate) fn frame(id: &str, name: &str, arguments: JsonValue) -> String {
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

pub(crate) fn tools_list_frame() -> String {
    String::from(r#"{"jsonrpc":"2.0","id":"list","method":"tools/list","params":{}}"#)
}

pub(crate) fn wire(output: &str) -> serde_json::Value {
    serde_json::from_str(output).unwrap_or(serde_json::Value::Null)
}

pub(crate) fn listed_names(output: &serde_json::Value) -> Vec<String> {
    output["result"]["tools"]
        .as_array()
        .map(|tools| {
            tools
                .iter()
                .filter_map(|tool| tool["name"].as_str())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn context_arguments() -> Vec<(String, JsonValue)> {
    vec![
        (String::from("instance_id"), JsonValue::string(INSTANCE)),
        (
            String::from("mcp_session_id"),
            JsonValue::string(MCP_SESSION),
        ),
        (String::from("lease_id"), JsonValue::string(LEASE)),
        (String::from("lease_epoch"), JsonValue::Number(LEASE_EPOCH)),
        (String::from("generation"), JsonValue::Number(GENERATION)),
    ]
}

pub(crate) fn observe_arguments() -> JsonValue {
    JsonValue::object(context_arguments())
}

pub(crate) fn map_arguments() -> JsonValue {
    JsonValue::object(context_arguments())
}

pub(crate) fn legal_actions_arguments() -> JsonValue {
    let mut fields = context_arguments();
    fields.push((String::from("state_id"), JsonValue::string("combat-1")));
    JsonValue::object(fields)
}

/// Game-information operations carry no generation: they bind to a content
/// manifest and a snapshot instead.
fn game_information_context() -> Vec<(String, JsonValue)> {
    vec![
        (String::from("instance_id"), JsonValue::string(INSTANCE)),
        (
            String::from("mcp_session_id"),
            JsonValue::string(MCP_SESSION),
        ),
        (String::from("lease_id"), JsonValue::string(LEASE)),
        (String::from("lease_epoch"), JsonValue::Number(LEASE_EPOCH)),
    ]
}

pub(crate) fn static_arguments() -> JsonValue {
    let mut fields = game_information_context();
    fields.extend([
        (
            String::from("content_manifest_id"),
            JsonValue::string("content-1"),
        ),
        (String::from("locale"), JsonValue::string("en-US")),
        (
            String::from("visibility_scope"),
            JsonValue::string("public"),
        ),
        (String::from("entity_kind"), JsonValue::string("card")),
        (String::from("projection"), JsonValue::string("summary")),
        (String::from("detail_level"), JsonValue::string("summary")),
        (
            String::from("fields"),
            JsonValue::Array(vec![JsonValue::string("display_name")]),
        ),
        (String::from("page_items"), JsonValue::Number(2)),
        (String::from("item_bytes"), JsonValue::Number(4096)),
        (String::from("page_bytes"), JsonValue::Number(65_536)),
        (String::from("text_bytes"), JsonValue::Number(1_024)),
        (String::from("cursor"), JsonValue::Null),
    ]);
    JsonValue::object(fields)
}

pub(crate) fn detail_arguments(snapshot: &str) -> JsonValue {
    parse_json(&DETAIL_ARGUMENTS.replace(SNAPSHOT, snapshot)).unwrap_or(JsonValue::Null)
}

fn v3_template() -> JsonValue {
    parse_json(V3_STATE_RESPONSE).unwrap_or(JsonValue::Null)
}

/// A complete `state_response` for the composed observe operation.
pub(crate) fn state_response(correlation: &str) -> JsonValue {
    let mut body = v3_template();
    if let JsonValue::Object(root) = &mut body {
        root.insert(
            String::from("correlation_id"),
            JsonValue::string(correlation),
        );
        root.insert(String::from("lease_epoch"), JsonValue::Number(LEASE_EPOCH));
        root.insert(String::from("generation"), JsonValue::Number(GENERATION));
        if let Some(JsonValue::Object(observation)) = root.get_mut("observation") {
            observation.insert(String::from("generation"), JsonValue::Number(GENERATION));
        }
    }
    body
}

/// A `legal_actions_response` for the composed legal-actions operation.
pub(crate) fn legal_actions_response(correlation: &str) -> JsonValue {
    let mut body = v3_template();
    if let JsonValue::Object(root) = &mut body {
        root.insert(
            String::from("correlation_id"),
            JsonValue::string(correlation),
        );
        root.insert(
            String::from("kind"),
            JsonValue::string("legal_actions_response"),
        );
        root.insert(String::from("lease_epoch"), JsonValue::Number(LEASE_EPOCH));
        root.insert(String::from("generation"), JsonValue::Number(GENERATION));
        root.insert(String::from("observation"), JsonValue::Null);
        root.insert(
            String::from("legal_actions"),
            JsonValue::Array(vec![JsonValue::object([
                (String::from("action_id"), JsonValue::string("end-turn")),
                (
                    String::from("action"),
                    JsonValue::object([(String::from("kind"), JsonValue::string("end_turn"))]),
                ),
            ])]),
        );
    }
    body
}

/// One complete map snapshot read, normalized onto the session generation.
pub(crate) fn map_snapshot_response(correlation: &str) -> JsonValue {
    let mut body = parse_json(MAP_SNAPSHOT_RESPONSE).unwrap_or(JsonValue::Null);
    if let JsonValue::Object(root) = &mut body {
        root.insert(
            String::from("correlation_id"),
            JsonValue::string(correlation),
        );
        root.insert(String::from("generation"), JsonValue::Number(GENERATION));
        if let Some(JsonValue::Object(snapshot)) = root.get_mut("snapshot") {
            snapshot.insert(String::from("generation"), JsonValue::Number(GENERATION));
        }
    }
    body
}

/// A successful static content search that echoes the plain search query.
pub(crate) fn search_response(correlation: &str) -> JsonValue {
    let mut body = parse_json(SEARCH_RESPONSE).unwrap_or(JsonValue::Null);
    if let JsonValue::Object(root) = &mut body {
        root.insert(
            String::from("correlation_id"),
            JsonValue::string(correlation),
        );
        if let Some(JsonValue::Object(query)) = root.get_mut("query") {
            query.insert(String::from("cursor"), JsonValue::Null);
            query.insert(String::from("query_kind"), JsonValue::string("search"));
        }
    }
    body
}

/// A live content detail bound to `snapshot`, carrying that identity in the echo.
pub(crate) fn detail_response(correlation: &str, snapshot: &str) -> JsonValue {
    let mut body =
        parse_json(&DETAIL_RESPONSE.replace(SNAPSHOT, snapshot)).unwrap_or(JsonValue::Null);
    if let JsonValue::Object(root) = &mut body {
        root.insert(
            String::from("correlation_id"),
            JsonValue::string(correlation),
        );
    }
    body
}

/// Every forwarded call in one composed session must carry the same selected
/// instance, gateway session, MCP session, and lease identity. Gameplay routes
/// carry it in the Runtime-v3 envelope body; map and game-information routes
/// carry it in the gateway authority headers.
pub(crate) fn assert_consistent_identity(requests: &[GatewayRequest]) {
    assert!(!requests.is_empty(), "no forwarded request to check");
    for request in requests {
        let body_string = |key: &str| match &request.body {
            Some(JsonValue::Object(object)) => match object.get(key) {
                Some(JsonValue::String(value)) => Some(value.clone()),
                _ => None,
            },
            _ => None,
        };
        let instance = request
            .headers
            .get("x-sts2-instance-id")
            .cloned()
            .or_else(|| body_string("instance_id"));
        assert_eq!(
            instance.as_deref(),
            Some(INSTANCE),
            "instance identity on {}",
            request.path
        );
        let session = request
            .headers
            .get("x-sts2-session-id")
            .cloned()
            .or_else(|| body_string("session_id"));
        assert_eq!(
            session.as_deref(),
            Some(GATEWAY_SESSION),
            "gateway session identity on {}",
            request.path
        );
        let lease = request
            .headers
            .get("x-sts2-lease-id")
            .cloned()
            .or_else(|| body_string("lease_id"));
        assert_eq!(
            lease.as_deref(),
            Some(LEASE),
            "lease identity on {}",
            request.path
        );
        let epoch = request
            .headers
            .get("x-sts2-lease-epoch")
            .and_then(|value| value.parse::<i64>().ok())
            .or_else(|| match &request.body {
                Some(JsonValue::Object(object)) => match object.get("lease_epoch") {
                    Some(JsonValue::Number(value)) => Some(*value),
                    _ => None,
                },
                _ => None,
            });
        assert_eq!(epoch, Some(LEASE_EPOCH), "lease epoch on {}", request.path);
        assert_eq!(
            request.headers.get("x-mcp-session-id").map(String::as_str),
            Some(MCP_SESSION),
            "mcp session identity on {}",
            request.path
        );
        assert!(
            request.path.contains(INSTANCE),
            "foreign instance in route {}",
            request.path
        );
    }
}
