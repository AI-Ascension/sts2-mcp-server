// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::gateway::GatewayAdapter;
use crate::json::JsonValue;
use crate::projection::RuntimeV2Context;
use crate::protocol_artifact_runtime_v2::RUNTIME_V2_MAX_GENERATION;
use crate::server::McpServer;

use super::super::{headers, non_empty_string, safe_header_value, safe_segment};

pub(super) fn authority_headers(context: &RuntimeV2Context) -> BTreeMap<String, String> {
    let mut headers = headers(&context.mcp_session_id, &context.correlation_id);
    for (name, value) in [
        ("x-sts2-instance-id", context.instance_id.clone()),
        ("x-sts2-session-id", context.session_id.clone()),
        ("x-sts2-lease-id", context.lease_id.clone()),
        ("x-sts2-lease-epoch", context.lease_epoch.to_string()),
    ] {
        headers.insert(String::from(name), value);
    }
    if let Some(workflow_boot_epoch) = &context.workflow_boot_epoch {
        headers.insert(
            String::from("x-sts2-workflow-boot-epoch"),
            workflow_boot_epoch.clone(),
        );
    }
    headers
}

pub(super) fn request_context<G: GatewayAdapter>(
    server: &McpServer<G>,
    arguments: &BTreeMap<String, JsonValue>,
    correlation_id: &str,
    require_operation_id: bool,
) -> Result<RuntimeV2Context, &'static str> {
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
            .ok_or("operation_id is required for Runtime-v2 operations")?
    } else {
        ""
    };
    if !safe_segment(instance_id)
        || !safe_header_value(mcp_session_id)
        || !safe_header_value(session_id)
        || !safe_header_value(lease_id)
        || (require_operation_id
            && (!safe_header_value(operation_id)
                || operation_id.contains('/')
                || matches!(operation_id, "." | "..")))
    {
        return Err("Runtime-v2 identity is unsafe or oversized");
    }
    let lease_epoch = bounded_argument(arguments, "lease_epoch")?;
    let generation = bounded_argument(arguments, "generation")?;
    let workflow_boot_epoch = match arguments.get("workflow_boot_epoch") {
        None => None,
        Some(value) => {
            let value = value
                .as_string()
                .ok_or("workflow_boot_epoch must be a safe string")?;
            if !safe_header_value(value) {
                return Err("workflow_boot_epoch must be a safe string");
            }
            Some(String::from(value))
        }
    };
    Ok(RuntimeV2Context {
        correlation_id: String::from(correlation_id),
        instance_id: String::from(instance_id),
        session_id: String::from(session_id),
        mcp_session_id: String::from(mcp_session_id),
        lease_id: String::from(lease_id),
        lease_epoch,
        generation,
        operation_id: String::from(operation_id),
        workflow_boot_epoch,
    })
}

fn bounded_argument(
    arguments: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<i64, &'static str> {
    match arguments.get(key) {
        Some(JsonValue::Number(value)) if *value >= 0 && *value <= RUNTIME_V2_MAX_GENERATION => {
            Ok(*value)
        }
        _ => Err("Runtime-v2 generation or lease_epoch is outside the protocol bound"),
    }
}
