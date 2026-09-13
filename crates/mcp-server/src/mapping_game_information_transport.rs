// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::gateway::{Correlation, GatewayAdapter, GatewayMethod, GatewayRequest};
use crate::json::JsonValue;
use crate::protocol::RequestId;
use crate::protocol_artifact_game_information::{
    GAME_INFORMATION_ARTIFACT, GAME_INFORMATION_GENERATOR, GAME_INFORMATION_PROTOCOL_VERSION,
    GAME_INFORMATION_SCHEMA_DIGEST, GAME_INFORMATION_SCHEMA_SOURCE,
};
use crate::server::McpServer;

use super::{GameInformationContext, identity};

pub(super) fn gateway_request(
    context: &GameInformationContext,
    method: GatewayMethod,
    path: String,
    body: Option<JsonValue>,
) -> GatewayRequest {
    let mut headers = super::super::headers(&context.mcp_session_id, &context.correlation_id);
    headers.extend([
        (
            String::from("x-sts2-instance-id"),
            context.instance_id.clone(),
        ),
        (
            String::from("x-sts2-session-id"),
            context.gateway_session_id.clone(),
        ),
        (String::from("x-sts2-lease-id"), context.lease_id.clone()),
        (
            String::from("x-sts2-lease-epoch"),
            context.lease_epoch.to_string(),
        ),
    ]);
    GatewayRequest {
        method,
        path,
        headers,
        body,
        correlation: Correlation {
            mcp_session_id: context.mcp_session_id.clone(),
            mcp_request_id: context.request_id.clone(),
        },
    }
}

pub(super) fn envelope(correlation: &str, kind: &str, query: JsonValue) -> JsonValue {
    JsonValue::object([
        (
            String::from("protocol_version"),
            JsonValue::string(GAME_INFORMATION_PROTOCOL_VERSION),
        ),
        (
            String::from("schema_digest"),
            JsonValue::string(GAME_INFORMATION_SCHEMA_DIGEST),
        ),
        (
            String::from("provenance"),
            JsonValue::object([
                (
                    String::from("artifact"),
                    JsonValue::string(GAME_INFORMATION_ARTIFACT),
                ),
                (
                    String::from("source"),
                    JsonValue::string(GAME_INFORMATION_SCHEMA_SOURCE),
                ),
                (
                    String::from("generator"),
                    JsonValue::string(GAME_INFORMATION_GENERATOR),
                ),
            ]),
        ),
        (
            String::from("correlation_id"),
            JsonValue::string(correlation),
        ),
        (String::from("kind"), JsonValue::string(kind)),
        (String::from("query"), query),
        (String::from("result"), JsonValue::Null),
        (String::from("capabilities"), JsonValue::Null),
        (String::from("error"), JsonValue::Null),
    ])
}

pub(super) fn context<G: GatewayAdapter>(
    server: &McpServer<G>,
    arguments: &BTreeMap<String, JsonValue>,
    correlation_id: &str,
    request_id: RequestId,
) -> Result<GameInformationContext, &'static str> {
    let instance_id = identity(arguments, "instance_id")?;
    if !super::super::safe_segment(instance_id) {
        return Err("instance_id is unsafe or oversized");
    }
    let mcp_session_id = identity(arguments, "mcp_session_id")?;
    if !super::super::safe_header_value(mcp_session_id) {
        return Err("mcp_session_id is unsafe or oversized");
    }
    if server
        .mcp_session_id()
        .is_some_and(|expected| expected != mcp_session_id)
    {
        return Err("MCP session identity does not match the configured session");
    }
    let gateway_session_id = server
        .gateway_session_id()
        .unwrap_or(mcp_session_id)
        .to_owned();
    if !super::super::safe_header_value(&gateway_session_id) {
        return Err("gateway session identity is unsafe or oversized");
    }
    let lease_id = identity(arguments, "lease_id")?;
    if !super::super::safe_header_value(lease_id) {
        return Err("lease_id is unsafe or oversized");
    }
    let lease_epoch = match arguments.get("lease_epoch") {
        Some(JsonValue::Number(value)) if (0..=9_007_199_254_740_991).contains(value) => *value,
        _ => return Err("lease_epoch is outside the protocol bound"),
    };
    Ok(GameInformationContext {
        instance_id: String::from(instance_id),
        mcp_session_id: String::from(mcp_session_id),
        gateway_session_id,
        lease_id: String::from(lease_id),
        lease_epoch,
        correlation_id: String::from(correlation_id),
        request_id,
    })
}
