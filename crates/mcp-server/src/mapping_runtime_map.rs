// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::catalog::MAP_SNAPSHOT_TOOL;
use crate::gateway::{Correlation, GatewayAdapter, GatewayMethod, GatewayRequest};
use crate::json::JsonValue;
use crate::projection::RuntimeMapProjectionContext;
use crate::protocol::{INVALID_PARAMS, RequestId, RpcError, RpcRequest, RpcResponse};
use crate::server::McpServer;

use super::{gateway_error_result, has_only_arguments, headers, invalid_params, response};

const MAP_ARGUMENTS: [&str; 5] = [
    "instance_id",
    "mcp_session_id",
    "lease_id",
    "lease_epoch",
    "generation",
];
const MAX_GENERATION: i64 = 9_007_199_254_740_991;

pub(super) fn tools_call<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    request: RpcRequest,
) -> RpcResponse {
    let Some(params) = request.params.as_object() else {
        return RpcResponse::failure(
            Some(request.id),
            RpcError::new(INVALID_PARAMS, "tools/call params must be an object"),
        );
    };
    if !has_only_arguments(params, &["name", "arguments"]) {
        return invalid_params(request.id, "tools/call params contain an unsupported field");
    }
    let Some(tool_name) = params.get("name").and_then(JsonValue::as_string) else {
        return invalid_params(request.id, "tools/call requires a tool name");
    };
    if tool_name != MAP_SNAPSHOT_TOOL {
        return super::runtime_v3_gameplay::tools_call(server, request);
    }
    let Some(arguments) = params.get("arguments").and_then(JsonValue::as_object) else {
        return invalid_params(request.id, "tools/call arguments must be an object");
    };
    let id = request.id;
    let correlation_id = id.stable_text();
    if !super::safe_header_value(&correlation_id) {
        return invalid_params(
            id,
            "request id contains an unsafe or oversized header value",
        );
    }
    map_snapshot_call(server, id, arguments, &correlation_id)
}

fn map_snapshot_call<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    id: RequestId,
    arguments: &BTreeMap<String, JsonValue>,
    correlation_id: &str,
) -> RpcResponse {
    if !has_only_arguments(arguments, &MAP_ARGUMENTS) {
        return invalid_params(
            id,
            "sts2.map_snapshot arguments contain an unsupported field",
        );
    }
    let context = match MapContext::parse(server, arguments, correlation_id) {
        Ok(context) => context,
        Err(message) => return invalid_params(id, message),
    };
    let mut request_headers = headers(&context.mcp_session_id, correlation_id);
    request_headers.extend([
        (
            String::from("x-sts2-instance-id"),
            context.projection.instance_id.clone(),
        ),
        (
            String::from("x-sts2-session-id"),
            context.projection.session_id.clone(),
        ),
        (
            String::from("x-sts2-lease-id"),
            context.projection.lease_id.clone(),
        ),
        (
            String::from("x-sts2-lease-epoch"),
            context.projection.lease_epoch.to_string(),
        ),
    ]);
    let request = GatewayRequest {
        method: GatewayMethod::Get,
        path: format!(
            "/v1/instances/{}/map-snapshot",
            context.projection.instance_id
        ),
        headers: request_headers,
        body: None,
        correlation: Correlation {
            mcp_session_id: context.mcp_session_id.clone(),
            mcp_request_id: id.clone(),
        },
    };
    match server.gateway.forward(request) {
        Ok(response) => response::gateway_success_map(id, response, &context.projection),
        Err(error) => gateway_error_result(id, error),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct MapContext {
    projection: RuntimeMapProjectionContext,
    mcp_session_id: String,
}

impl MapContext {
    fn parse<G: GatewayAdapter>(
        server: &McpServer<G>,
        arguments: &BTreeMap<String, JsonValue>,
        correlation_id: &str,
    ) -> Result<Self, &'static str> {
        let instance_id = string_argument(arguments, "instance_id")?;
        let mcp_session_id = string_argument(arguments, "mcp_session_id")?;
        if server
            .mcp_session_id()
            .is_some_and(|expected| expected != mcp_session_id)
        {
            return Err("MCP session identity does not match the configured session");
        }
        let session_id = server.gateway_session_id().unwrap_or(mcp_session_id);
        let lease_id = string_argument(arguments, "lease_id")?;
        if !crate::mapping::safe_segment(instance_id)
            || !crate::mapping::safe_header_value(mcp_session_id)
            || !crate::mapping::safe_header_value(session_id)
            || !crate::mapping::safe_header_value(lease_id)
            || !crate::mapping::safe_header_value(correlation_id)
        {
            return Err("Runtime-map identity is unsafe or oversized");
        }
        let lease_epoch = bounded_argument(arguments, "lease_epoch")?;
        let generation = bounded_argument(arguments, "generation")?;
        Ok(Self {
            projection: RuntimeMapProjectionContext {
                correlation_id: String::from(correlation_id),
                instance_id: String::from(instance_id),
                session_id: String::from(session_id),
                lease_id: String::from(lease_id),
                lease_epoch,
                generation,
            },
            mcp_session_id: String::from(mcp_session_id),
        })
    }
}

fn string_argument<'a>(
    arguments: &'a BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<&'a str, &'static str> {
    arguments
        .get(key)
        .and_then(JsonValue::as_string)
        .filter(|value| !value.is_empty())
        .ok_or("Runtime-map identity argument must be a non-empty string")
}

fn bounded_argument(
    arguments: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<i64, &'static str> {
    match arguments.get(key) {
        Some(JsonValue::Number(value)) if *value >= 0 && *value <= MAX_GENERATION => Ok(*value),
        _ => Err("Runtime-map generation or lease_epoch is outside the protocol bound"),
    }
}
