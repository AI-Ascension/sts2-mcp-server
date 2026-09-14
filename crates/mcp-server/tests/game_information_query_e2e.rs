// SPDX-License-Identifier: MIT
#![allow(clippy::expect_used, clippy::panic)]

//! Scoped issue #51 acceptance: the real MCP catalog and mapping composition is
//! driven through `initialize` -> `tools/list` -> manifest -> search -> get ->
//! live detail -> next page against one owned synthetic producer through the
//! fixed gateway route mapping.
//!
//! The producer is synthetic test code. Nothing here contacts a game process, a
//! provider, a profile, a save or a network, and no claim about native behavior,
//! deployment or effect settlement is made.

#[path = "game_information_query_e2e/composition.rs"]
mod composition;
#[path = "game_information_query_e2e/content.rs"]
mod content;
#[path = "game_information_query_e2e/envelope.rs"]
mod envelope;
#[path = "game_information_query_e2e/errors.rs"]
mod errors;
#[path = "game_information_query_e2e/page.rs"]
mod page;
#[path = "game_information_query_e2e/producer.rs"]
mod producer;
#[path = "game_information_query_e2e/sequence.rs"]
mod sequence;

use serde_json::{Value, json};
use sts2_mcp_server::{McpServer, ToolCatalog};

use content::live_instance_ref;
use producer::{
    CONTENT_MANIFEST_ID, GATEWAY_SESSION_ID, INSTANCE_ID, LEASE_EPOCH, LEASE_ID, LIVE_ENTITY_ID,
    MCP_SESSION_ID, RUN_ID, SNAPSHOT_GENERATION, SNAPSHOT_ID, SyntheticProducer,
};

/// Builds the runtime composition the shipped `game-information-query-v1`
/// executable profile uses, bound to one gateway and one MCP session.
fn server(producer: SyntheticProducer) -> McpServer<SyntheticProducer> {
    McpServer::with_catalog_and_sessions(
        producer,
        ToolCatalog::game_information_query_v1(),
        GATEWAY_SESSION_ID,
        MCP_SESSION_ID,
    )
}

fn request(
    server: &mut McpServer<SyntheticProducer>,
    id: &str,
    method: &str,
    params: Value,
) -> Value {
    let frame = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}).to_string();
    let response = server
        .handle_message(&frame)
        .expect("request frames produce exactly one response");
    serde_json::from_str(&response).expect("MCP response is JSON")
}

fn call(
    server: &mut McpServer<SyntheticProducer>,
    id: &str,
    name: &str,
    arguments: Value,
) -> Value {
    let wire = request(
        server,
        id,
        "tools/call",
        json!({"name": name, "arguments": arguments}),
    );
    assert!(
        wire.get("result").is_some(),
        "tools/call {name} returned a JSON-RPC error: {wire}"
    );
    wire
}

/// Dispatches a `tools/call` without assuming the outcome, for frames that are
/// expected to be refused before or instead of a projection.
fn attempt(
    server: &mut McpServer<SyntheticProducer>,
    id: &str,
    name: &str,
    arguments: Value,
) -> Value {
    request(
        server,
        id,
        "tools/call",
        json!({"name": name, "arguments": arguments}),
    )
}

fn tool_envelope(wire: &Value) -> Value {
    assert_eq!(wire["result"]["isError"], false, "{wire}");
    let text = wire["result"]["content"][0]["text"]
        .as_str()
        .expect("tool result carries bounded text");
    serde_json::from_str(text).expect("tool result text is a game-information envelope")
}

fn error_code(wire: &Value) -> &str {
    wire["result"]["structuredContent"]["error"]["code"]
        .as_str()
        .expect("typed error code")
}

fn error_category(wire: &Value) -> &str {
    wire["result"]["structuredContent"]["error"]["category"]
        .as_str()
        .expect("typed error category")
}

fn assert_tool_error(wire: &Value, code: &str, category: &str) {
    assert_eq!(wire["result"]["isError"], true, "{wire}");
    assert_eq!(error_code(wire), code, "{wire}");
    assert_eq!(error_category(wire), category, "{wire}");
}

fn base_query() -> Value {
    json!({
        "instance_id": INSTANCE_ID,
        "mcp_session_id": MCP_SESSION_ID,
        "lease_id": LEASE_ID,
        "lease_epoch": LEASE_EPOCH,
        "content_manifest_id": CONTENT_MANIFEST_ID,
        "locale": "en-US",
        "visibility_scope": "public",
        "entity_kind": "card",
        "projection": "summary",
        "detail_level": "summary",
        "fields": ["display_name"],
        "page_items": 2,
        "item_bytes": 4096,
        "page_bytes": 65536,
        "text_bytes": 1024,
        "cursor": Value::Null,
    })
}

fn search_arguments(cursor: Option<&str>, projection: &str) -> Value {
    let mut arguments = base_query();
    arguments["projection"] = json!(projection);
    arguments["cursor"] = cursor.map_or(Value::Null, |cursor| json!(cursor));
    arguments
}

fn get_arguments(namespaced_id: &str) -> Value {
    let mut arguments = base_query();
    arguments["definition_ref"] = json!({
        "content_manifest_id": CONTENT_MANIFEST_ID,
        "entity_kind": "card",
        "namespaced_id": namespaced_id,
        "variant": Value::Null,
    });
    arguments
}

fn instance_ref(entity_kind: &str, entity_id: &str) -> Value {
    json!({
        "instance_id": INSTANCE_ID,
        "run_id": RUN_ID,
        "epoch": LEASE_EPOCH,
        "entity_kind": entity_kind,
        "entity_id": entity_id,
    })
}

fn snapshot_ref(instance: &Value) -> Value {
    json!({
        "snapshot_id": SNAPSHOT_ID,
        "instance_ref": instance,
        "state_generation": SNAPSHOT_GENERATION,
    })
}

fn live_arguments(entity_kind: &str, entity_id: &str, definition_id: &str) -> Value {
    let mut arguments = base_query();
    arguments["visibility_scope"] = json!("player");
    arguments["entity_kind"] = json!(entity_kind);
    arguments["projection"] = json!("full");
    arguments["detail_level"] = json!("full");
    arguments["fields"] = json!(["amount", "cost", "description", "display_name"]);
    arguments["page_items"] = json!(1);
    arguments["text_bytes"] = json!(4096);
    arguments["definition_ref"] = json!({
        "content_manifest_id": CONTENT_MANIFEST_ID,
        "entity_kind": entity_kind,
        "namespaced_id": definition_id,
        "variant": Value::Null,
    });
    let instance = instance_ref(entity_kind, entity_id);
    arguments["snapshot_ref"] = snapshot_ref(&instance);
    arguments["parent_observation"] = json!({
        "instance_ref": instance,
        "snapshot_ref": snapshot_ref(&instance),
        "state_generation": SNAPSHOT_GENERATION,
    });
    arguments["instance_ref"] = instance;
    arguments
}

/// Asserts the mapped request preserved every static parameter, so no content,
/// locale, scope, bound, filter or cursor field is silently dropped.
fn assert_static_query_echo(query: &Value, cursor: Value) {
    assert_eq!(query["query_kind"], "search");
    assert_eq!(query["entity_kind"], "card");
    assert_eq!(
        query["target"],
        json!({"definition_ref": Value::Null, "instance_ref": Value::Null})
    );
    assert_eq!(
        query["filters"],
        json!({
            "definition_refs": [],
            "display_name": Value::Null,
            "instance_ids": [],
            "namespaced_ids": [],
        })
    );
    assert_eq!(query["projection"], "summary");
    assert_eq!(query["detail_level"], "summary");
    assert_eq!(query["fields"], json!(["display_name"]));
    assert_eq!(
        query["binding"],
        json!({
            "content_manifest_id": CONTENT_MANIFEST_ID,
            "instance_ref": Value::Null,
            "locale": "en-US",
            "mode": "static",
            "snapshot_ref": Value::Null,
            "visibility_scope": "public",
        })
    );
    assert_eq!(query["parent_observation"], Value::Null);
    assert_eq!(
        query["limits"],
        json!({"item_bytes": 4096, "page_bytes": 65536, "page_items": 2, "text_bytes": 1024})
    );
    assert_eq!(query["cursor"], cursor);
}

/// Asserts the mapped live request preserved the instance and snapshot fence.
fn assert_live_query_echo(query: &Value) {
    let instance = instance_ref("card", LIVE_ENTITY_ID);
    assert_eq!(query["query_kind"], "detail");
    assert_eq!(query["entity_kind"], "card");
    assert_eq!(query["cursor"], Value::Null);
    assert_eq!(query["binding"]["mode"], "live");
    assert_eq!(query["binding"]["visibility_scope"], "player");
    assert_eq!(query["binding"]["content_manifest_id"], CONTENT_MANIFEST_ID);
    assert_eq!(query["binding"]["locale"], "en-US");
    assert_eq!(query["binding"]["instance_ref"], instance);
    assert_eq!(query["binding"]["snapshot_ref"], snapshot_ref(&instance));
    assert_eq!(query["parent_observation"]["instance_ref"], instance);
    assert_eq!(
        query["parent_observation"]["snapshot_ref"],
        snapshot_ref(&instance)
    );
    assert_eq!(
        query["parent_observation"]["state_generation"],
        SNAPSHOT_GENERATION
    );
    assert_eq!(query["target"]["instance_ref"], instance);
    assert_eq!(
        query["target"]["definition_ref"]["namespaced_id"],
        "synthetic:strike"
    );
}

/// One requested static field exactly as the producer serves it.
fn definition_fields(value: Value) -> Value {
    json!([{
        "availability": "available",
        "kind": "text",
        "name": "display_name",
        "reason": Value::Null,
        "source": {"kind": "content_manifest", "ref": CONTENT_MANIFEST_ID},
        "unit": Value::Null,
        "value": value,
    }])
}

/// Asserts the live detail item is bound to the pinned snapshot and keeps every
/// requested field, including the one the scope cannot observe.
fn assert_live_detail_item(item: &Value) {
    assert_eq!(item["instance_ref"], live_instance_ref());
    assert_eq!(item["definition_ref"]["namespaced_id"], "synthetic:strike");
    let fields = item["fields"].as_array().expect("live detail fields");
    assert_eq!(fields.len(), 4);
    assert_eq!(fields[0]["name"], "amount");
    assert_eq!(fields[0]["value"], 3);
    assert_eq!(fields[0]["availability"], "available");
    assert_eq!(fields[0]["source"]["kind"], "game_mod");
    assert_eq!(fields[1]["name"], "cost");
    assert_eq!(fields[1]["value"], 1);
    assert_eq!(fields[2]["name"], "description");
    assert_eq!(fields[2]["availability"], "not_observable");
    assert_eq!(fields[2]["value"], Value::Null);
    assert!(
        fields[2]["reason"]
            .as_str()
            .is_some_and(|reason| !reason.is_empty()),
        "unavailable fields keep their reason: {fields:?}"
    );
    assert_eq!(fields[3]["name"], "display_name");
    assert_eq!(fields[3]["value"], "Strike");
}
