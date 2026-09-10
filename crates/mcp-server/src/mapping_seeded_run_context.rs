// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::gateway::GatewayAdapter;
use crate::json::JsonValue;
use crate::protocol_artifact_seeded_run::SEEDED_RUN_MAX_GENERATION;
use crate::server::McpServer;

use super::SeededContext;
pub(super) fn parse<G: GatewayAdapter>(
    server: &McpServer<G>,
    arguments: &BTreeMap<String, JsonValue>,
    correlation_id: &str,
    require_selected: bool,
) -> Result<SeededContext, &'static str> {
    let instance_id = string(arguments, "instance_id")?;
    let mcp_session_id = string(arguments, "mcp_session_id")?;
    let lease_id = string(arguments, "lease_id")?;
    let operation_id = string(arguments, "operation_id")?;
    let session_id = server.gateway_session_id().unwrap_or(mcp_session_id);
    if !super::super::safe_segment(instance_id)
        || !super::super::safe_header_value(mcp_session_id)
        || !super::super::safe_header_value(session_id)
        || !super::super::safe_header_value(lease_id)
        || !safe_operation_id(operation_id)
    {
        return Err("seeded-run identity is unsafe, not a path-safe segment, or oversized");
    }
    if server
        .mcp_session_id()
        .is_some_and(|expected| expected != mcp_session_id)
    {
        return Err("mcp_session_id is not bound to this MCP session");
    }
    let lease_epoch = bounded_argument(arguments, "lease_epoch")?;
    let generation = bounded_argument(arguments, "generation")?;
    if require_selected && arguments.get("selected_context").is_none() {
        return Err("selected_context is required");
    }
    Ok(SeededContext {
        correlation_id: String::from(correlation_id),
        instance_id: String::from(instance_id),
        mcp_session_id: String::from(mcp_session_id),
        session_id: String::from(session_id),
        lease_id: String::from(lease_id),
        lease_epoch,
        generation,
        operation_id: String::from(operation_id),
        selected_context: None,
        context_digest: None,
    })
}

fn string<'a>(
    arguments: &'a BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<&'a str, &'static str> {
    arguments
        .get(key)
        .and_then(JsonValue::as_string)
        .filter(|value| !value.is_empty())
        .ok_or("seeded-run identity is missing")
}

fn bounded_argument(
    arguments: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<i64, &'static str> {
    match arguments.get(key) {
        Some(JsonValue::Number(value)) if (0..=SEEDED_RUN_MAX_GENERATION).contains(value) => {
            Ok(*value)
        }
        _ => Err("seeded-run generation or lease_epoch is outside the protocol bound"),
    }
}

fn safe_operation_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && !value.contains("..")
        && !value.contains('/')
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}

pub(super) fn authority_headers(context: &SeededContext) -> BTreeMap<String, String> {
    let mut headers = super::super::headers(&context.mcp_session_id, &context.correlation_id);
    headers.insert(
        String::from("x-sts2-instance-id"),
        context.instance_id.clone(),
    );
    headers.insert(
        String::from("x-sts2-session-id"),
        context.session_id.clone(),
    );
    headers.insert(String::from("x-sts2-lease-id"), context.lease_id.clone());
    headers.insert(
        String::from("x-sts2-lease-epoch"),
        context.lease_epoch.to_string(),
    );
    headers
}
