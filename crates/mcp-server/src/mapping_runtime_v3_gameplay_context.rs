// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::gateway::GatewayAdapter;
use crate::json::JsonValue;
use crate::projection::RuntimeV3GameplayContext;
use crate::protocol_artifact_runtime_v3_gameplay::RUNTIME_V3_GAMEPLAY_MAX_GENERATION;
use crate::server::McpServer;

use super::super::{non_empty_string, safe_header_value, safe_segment};

pub(super) fn authority_headers(context: &RuntimeV3GameplayContext) -> BTreeMap<String, String> {
    let mut headers = super::super::headers(&context.mcp_session_id, &context.correlation_id);
    for (name, value) in [
        ("x-sts2-instance-id", context.instance_id.clone()),
        ("x-sts2-session-id", context.session_id.clone()),
        ("x-sts2-lease-id", context.lease_id.clone()),
        ("x-sts2-lease-epoch", context.lease_epoch.to_string()),
    ] {
        headers.insert(String::from(name), value);
    }
    headers
}

pub(super) fn request_context<G: GatewayAdapter>(
    server: &McpServer<G>,
    arguments: &BTreeMap<String, JsonValue>,
    correlation_id: &str,
    require_operation_id: bool,
) -> Result<RuntimeV3GameplayContext, &'static str> {
    let instance_id = non_empty_string(arguments, "instance_id")
        .ok_or("instance_id must be a non-empty string")?;
    let mcp_session_id = non_empty_string(arguments, "mcp_session_id")
        .ok_or("mcp_session_id must be a non-empty string")?;
    if let Some(expected) = server.mcp_session_id()
        && expected != mcp_session_id
    {
        return Err("MCP session identity does not match the configured session");
    }
    let session_id = server.gateway_session_id().unwrap_or(mcp_session_id);
    let lease_id =
        non_empty_string(arguments, "lease_id").ok_or("lease_id must be a non-empty string")?;
    let operation_id = if require_operation_id {
        non_empty_string(arguments, "operation_id")
            .ok_or("operation_id is required for Runtime-v3 gameplay operations")?
    } else {
        ""
    };
    if !safe_segment(instance_id)
        || !safe_header_value(mcp_session_id)
        || !safe_header_value(session_id)
        || !safe_header_value(lease_id)
        || (require_operation_id
            && (!safe_header_value(operation_id) || operation_id.contains('/')))
    {
        return Err("Runtime-v3 gameplay identity is unsafe or oversized");
    }
    let lease_epoch = bounded_argument(arguments, "lease_epoch")?;
    let generation = bounded_argument(arguments, "generation")?;
    let card_index = nonnegative_integer(arguments, "card_index").unwrap_or(0);
    let target_id = arguments
        .get("target_id")
        .map(|value| optional_target(Some(value)))
        .transpose()?
        .flatten();
    Ok(RuntimeV3GameplayContext {
        correlation_id: String::from(correlation_id),
        instance_id: String::from(instance_id),
        session_id: String::from(session_id),
        mcp_session_id: String::from(mcp_session_id),
        lease_id: String::from(lease_id),
        lease_epoch,
        generation,
        operation_id: String::from(operation_id),
        card_index,
        target_id,
    })
}

fn bounded_argument(
    arguments: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<i64, &'static str> {
    match arguments.get(key) {
        Some(JsonValue::Number(value))
            if *value >= 0 && *value <= RUNTIME_V3_GAMEPLAY_MAX_GENERATION =>
        {
            Ok(*value)
        }
        _ => Err("Runtime-v3 gameplay generation or lease_epoch is outside the protocol bound"),
    }
}

pub(super) fn nonnegative_integer(
    arguments: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Option<i64> {
    match arguments.get(key) {
        Some(JsonValue::Number(value)) if *value >= 0 => Some(*value),
        _ => None,
    }
}

pub(super) fn optional_target(value: Option<&JsonValue>) -> Result<Option<String>, &'static str> {
    match value {
        Some(JsonValue::Null) => Ok(None),
        Some(JsonValue::String(value)) if !value.is_empty() && safe_header_value(value) => {
            Ok(Some(value.clone()))
        }
        _ => Err("target_id must be null or a safe non-empty string"),
    }
}
