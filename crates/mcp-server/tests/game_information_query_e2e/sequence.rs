// SPDX-License-Identifier: MIT
//! Positive acceptance sequence for issue #51: initialize, discovery, manifest,
//! static two-page search with a cursor, get and one pinned live detail, all
//! through the real MCP catalog/mapping composition.

use serde_json::{Value, json};
use sts2_mcp_server::{
    GAME_INFORMATION_AVAILABILITY_TOOL, GAME_INFORMATION_CAPABILITIES_TOOL,
    GAME_INFORMATION_DETAIL_TOOL, GAME_INFORMATION_GET_TOOL, GAME_INFORMATION_LIST_TOOL,
    GAME_INFORMATION_SEARCH_TOOL, verify_game_information_artifact,
};

use super::content::CARD_FIELDS;
use super::producer::{
    CAPABILITIES_PATH, CONTENT_MANIFEST_ID, INSTANCE_ID, LEASE_EPOCH, LEASE_ID, LIVE_ENTITY_ID,
    MCP_SESSION_ID, QUERY_PATH, SNAPSHOT_GENERATION, SyntheticProducer,
};
use super::{
    assert_live_detail_item, assert_live_query_echo, assert_static_query_echo, base_query, call,
    definition_fields, get_arguments, live_arguments, request, search_arguments, server,
    tool_envelope,
};

const REVISION: &str = "game-information-query-v1-mcp";
const PROFILE: &str = "game-information-query-v1";

#[test]
fn pinned_artifact_and_read_only_catalog_are_advertised() {
    assert_eq!(verify_game_information_artifact(), Ok(()));
    let mut server = server(SyntheticProducer::contract());
    let wire = request(&mut server, "list", "tools/list", json!({}));
    assert_eq!(wire["result"]["revision"], REVISION);
    let tools = wire["result"]["tools"].as_array().expect("tools array");
    let names: Vec<&str> = tools
        .iter()
        .filter_map(|tool| tool["name"].as_str())
        .collect();
    for name in [
        GAME_INFORMATION_CAPABILITIES_TOOL,
        GAME_INFORMATION_LIST_TOOL,
        GAME_INFORMATION_SEARCH_TOOL,
        GAME_INFORMATION_GET_TOOL,
        GAME_INFORMATION_DETAIL_TOOL,
        GAME_INFORMATION_AVAILABILITY_TOOL,
    ] {
        assert!(
            names.contains(&name),
            "{name} is not advertised in {names:?}"
        );
    }
    for tool in tools {
        assert_eq!(tool["annotations"]["readOnlyHint"], true, "{tool}");
        assert_eq!(tool["annotations"]["destructiveHint"], false, "{tool}");
        assert_eq!(tool["annotations"]["idempotentHint"], true, "{tool}");
        assert_eq!(tool["annotations"]["openWorldHint"], false, "{tool}");
        assert_eq!(tool["inputSchema"]["additionalProperties"], false, "{tool}");
    }
    assert!(server.gateway().violations().is_empty());
}

#[test]
fn initialize_tools_list_and_manifest_discovery_succeed() {
    let mut server = server(SyntheticProducer::contract());
    let wire = request(
        &mut server,
        "init",
        "initialize",
        json!({
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {"name": "issue-51-acceptance", "version": "1.0.0"},
        }),
    );
    assert_eq!(wire["result"]["protocolVersion"], "2025-06-18");
    assert_eq!(wire["result"]["serverInfo"]["name"], "sts2-mcp-server");
    assert_eq!(wire["result"]["capabilities"]["tools"], json!({}));

    let wire = request(&mut server, "list", "tools/list", json!({}));
    assert_eq!(wire["result"]["revision"], REVISION);
    assert_eq!(wire["result"]["tools"].as_array().map(Vec::len), Some(8));

    let wire = call(
        &mut server,
        "manifest",
        GAME_INFORMATION_CAPABILITIES_TOOL,
        json!({
            "instance_id": INSTANCE_ID,
            "mcp_session_id": MCP_SESSION_ID,
            "lease_id": LEASE_ID,
            "lease_epoch": LEASE_EPOCH,
        }),
    );
    let manifest = tool_envelope(&wire);
    assert_eq!(manifest["kind"], "capabilities_response");
    assert_eq!(manifest["capabilities"]["profile"], PROFILE);
    assert_eq!(
        manifest["capabilities"]["entity_kinds"],
        json!(["card", "relic"])
    );
    assert_eq!(manifest["capabilities"]["fields"], json!(CARD_FIELDS));
    assert_eq!(
        manifest["capabilities"]["snapshot_policy"]["supports_live"],
        true
    );
    assert_eq!(manifest["capabilities"]["limits"]["page_items"], 128);

    let records = server.gateway().records();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].path, CAPABILITIES_PATH);
    assert!(
        server.gateway().violations().is_empty(),
        "{:?}",
        server.gateway().violations()
    );
}

#[test]
fn search_get_live_detail_and_next_page_reach_the_synthetic_producer() {
    let mut server = server(SyntheticProducer::contract());
    let wire = call(
        &mut server,
        "search-1",
        GAME_INFORMATION_SEARCH_TOOL,
        search_arguments(None, "summary"),
    );
    let envelope = tool_envelope(&wire);
    assert_eq!(envelope["correlation_id"], "search-1");
    assert_eq!(envelope["kind"], "query_response");
    assert_static_query_echo(&envelope["query"], Value::Null);
    let result = &envelope["result"];
    assert_eq!(result["read_only"], true);
    assert_eq!(result["result_generation"], Value::Null);
    let page = &result["page"];
    assert_eq!(page["items"].as_array().map(Vec::len), Some(2));
    assert_eq!(
        page["items"][0]["definition_ref"]["content_manifest_id"],
        CONTENT_MANIFEST_ID
    );
    assert_eq!(
        page["items"][0]["definition_ref"]["namespaced_id"],
        "synthetic:bash"
    );
    assert_eq!(
        page["items"][1]["definition_ref"]["namespaced_id"],
        "synthetic:defend"
    );
    assert_eq!(page["items"][0]["fields"], definition_fields(json!("Bash")));
    assert_eq!(
        page["items"][1]["fields"],
        definition_fields(json!("Defend"))
    );
    assert_eq!(page["total_count"], 3);
    assert_eq!(page["total_count_known"], true);
    assert_eq!(page["coverage"], "complete");
    assert_eq!(page["final_page"], false);
    assert_eq!(page["cursor_binding"]["cursor"], Value::Null);
    assert_eq!(page["ordering"]["deterministic"], true);
    let cursor = page["next_cursor"]
        .as_str()
        .expect("first page carries a continuation cursor")
        .to_owned();

    let wire = call(
        &mut server,
        "search-2",
        GAME_INFORMATION_SEARCH_TOOL,
        search_arguments(Some(&cursor), "summary"),
    );
    let envelope = tool_envelope(&wire);
    assert_static_query_echo(&envelope["query"], json!(cursor));
    let page = &envelope["result"]["page"];
    assert_eq!(page["items"].as_array().map(Vec::len), Some(1));
    assert_eq!(
        page["items"][0]["definition_ref"]["namespaced_id"],
        "synthetic:strike"
    );
    assert_eq!(
        page["items"][0]["fields"],
        definition_fields(json!("Strike"))
    );
    assert_eq!(page["final_page"], true);
    assert_eq!(page["next_cursor"], Value::Null);
    assert_eq!(page["cursor_binding"], Value::Null);
    assert_eq!(page["total_count"], 3);

    let wire = call(
        &mut server,
        "get",
        GAME_INFORMATION_GET_TOOL,
        get_arguments("synthetic:defend"),
    );
    let envelope = tool_envelope(&wire);
    assert_eq!(envelope["query"]["query_kind"], "get");
    assert_eq!(
        envelope["query"]["target"]["definition_ref"]["namespaced_id"],
        "synthetic:defend"
    );
    let page = &envelope["result"]["page"];
    assert_eq!(page["items"].as_array().map(Vec::len), Some(1));
    assert_eq!(
        page["items"][0]["definition_ref"]["namespaced_id"],
        "synthetic:defend"
    );
    assert_eq!(page["final_page"], true);
    assert_eq!(page["total_count"], 1);

    let wire = call(
        &mut server,
        "detail",
        GAME_INFORMATION_DETAIL_TOOL,
        live_arguments("card", LIVE_ENTITY_ID, "synthetic:strike"),
    );
    let envelope = tool_envelope(&wire);
    assert_live_query_echo(&envelope["query"]);
    let result = &envelope["result"];
    assert_eq!(result["read_only"], true);
    assert_eq!(result["result_generation"], SNAPSHOT_GENERATION);
    assert_eq!(
        result["parent_observation"],
        envelope["query"]["parent_observation"]
    );
    assert_live_detail_item(&result["page"]["items"][0]);
    assert_eq!(result["page"]["final_page"], true);

    let mut availability = base_query();
    availability["entity_kind"] = json!("relic");
    availability["fields"] = json!(["description", "display_name", "rarity", "source_id", "tags"]);
    availability["mode"] = json!("static");
    let wire = call(
        &mut server,
        "availability",
        GAME_INFORMATION_AVAILABILITY_TOOL,
        availability,
    );
    let envelope = tool_envelope(&wire);
    let fields = envelope["result"]["page"]["items"][0]["fields"]
        .as_array()
        .expect("availability fields");
    assert_eq!(fields.len(), 5);
    for field in fields {
        assert_eq!(field["availability"], "not_observable", "{field}");
        assert_eq!(field["value"], Value::Null, "{field}");
        assert!(
            field["reason"]
                .as_str()
                .is_some_and(|reason| !reason.is_empty()),
            "{field}"
        );
    }

    let records = server.gateway().records();
    let kinds: Vec<&str> = records
        .iter()
        .map(|record| record.query_kind.as_str())
        .collect();
    assert_eq!(kinds, ["search", "search", "get", "detail", "availability"]);
    assert!(
        records[1].cursor.is_some(),
        "second page carried the cursor"
    );
    assert!(records[0].cursor.is_none());
    for record in records {
        assert_eq!(record.path, QUERY_PATH);
    }
    assert!(
        server.gateway().violations().is_empty(),
        "{:?}",
        server.gateway().violations()
    );
}
/// The issue #51 acceptance sequence interleaved in one ordered run:
/// initialize -> tools/list -> manifest -> search -> get -> live detail ->
/// next page. The continuation cursor from the first page is retained across
/// the interleaved get and live detail and redeemed only afterwards, and the
/// final page and downstream request order are asserted explicitly.
#[test]
fn interleaved_lookup_sequence_succeeds_in_order() {
    let mut server = server(SyntheticProducer::contract());
    let wire = request(
        &mut server,
        "step-1",
        "initialize",
        json!({
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {"name": "issue-51-interleaved-sequence", "version": "1.0.0"},
        }),
    );
    assert_eq!(wire["result"]["protocolVersion"], "2025-06-18");

    let wire = request(&mut server, "step-2", "tools/list", json!({}));
    assert_eq!(wire["result"]["revision"], REVISION);
    assert_eq!(wire["result"]["tools"].as_array().map(Vec::len), Some(8));

    let wire = call(
        &mut server,
        "step-3",
        GAME_INFORMATION_CAPABILITIES_TOOL,
        json!({
            "instance_id": INSTANCE_ID,
            "mcp_session_id": MCP_SESSION_ID,
            "lease_id": LEASE_ID,
            "lease_epoch": LEASE_EPOCH,
        }),
    );
    let manifest = tool_envelope(&wire);
    assert_eq!(manifest["capabilities"]["profile"], PROFILE);

    let wire = call(
        &mut server,
        "step-4",
        GAME_INFORMATION_SEARCH_TOOL,
        search_arguments(None, "summary"),
    );
    let envelope = tool_envelope(&wire);
    assert_static_query_echo(&envelope["query"], Value::Null);
    let page = &envelope["result"]["page"];
    assert_eq!(page["items"].as_array().map(Vec::len), Some(2));
    assert_eq!(page["final_page"], false);
    assert_eq!(page["cursor_binding"]["query_kind"], "search");
    assert_eq!(
        page["cursor_binding"]["binding"],
        json!({
            "content_manifest_id": CONTENT_MANIFEST_ID,
            "instance_ref": Value::Null,
            "locale": "en-US",
            "mode": "static",
            "snapshot_ref": Value::Null,
            "visibility_scope": "public",
        })
    );
    let cursor = page["next_cursor"]
        .as_str()
        .expect("first page carries a continuation cursor")
        .to_owned();

    let wire = call(
        &mut server,
        "step-5",
        GAME_INFORMATION_GET_TOOL,
        get_arguments("synthetic:defend"),
    );
    let page = &tool_envelope(&wire)["result"]["page"];
    assert_eq!(page["final_page"], true);
    assert_eq!(page["items"].as_array().map(Vec::len), Some(1));
    assert_eq!(
        page["items"][0]["definition_ref"]["namespaced_id"],
        "synthetic:defend"
    );

    let wire = call(
        &mut server,
        "step-6",
        GAME_INFORMATION_DETAIL_TOOL,
        live_arguments("card", LIVE_ENTITY_ID, "synthetic:strike"),
    );
    let envelope = tool_envelope(&wire);
    assert_live_query_echo(&envelope["query"]);
    assert_eq!(envelope["result"]["result_generation"], SNAPSHOT_GENERATION);
    assert_live_detail_item(&envelope["result"]["page"]["items"][0]);

    let wire = call(
        &mut server,
        "step-7",
        GAME_INFORMATION_SEARCH_TOOL,
        search_arguments(Some(&cursor), "summary"),
    );
    let envelope = tool_envelope(&wire);
    assert_static_query_echo(&envelope["query"], json!(cursor));
    let page = &envelope["result"]["page"];
    assert_eq!(page["final_page"], true);
    assert_eq!(page["next_cursor"], Value::Null);
    assert_eq!(page["cursor_binding"], Value::Null);
    assert_eq!(page["total_count"], 3);
    assert_eq!(page["items"].as_array().map(Vec::len), Some(1));
    assert_eq!(
        page["items"][0]["definition_ref"]["namespaced_id"],
        "synthetic:strike"
    );

    let records = server.gateway().records();
    let kinds: Vec<&str> = records
        .iter()
        .map(|record| record.query_kind.as_str())
        .collect();
    assert_eq!(kinds, ["capabilities", "search", "get", "detail", "search"]);
    assert_eq!(records[0].path, CAPABILITIES_PATH);
    assert!(records[1].cursor.is_none());
    assert_eq!(records[4].cursor.as_deref(), Some(cursor.as_str()));
    for record in &records[1..] {
        assert_eq!(record.path, QUERY_PATH);
    }
    assert!(
        server.gateway().violations().is_empty(),
        "{:?}",
        server.gateway().violations()
    );
}
