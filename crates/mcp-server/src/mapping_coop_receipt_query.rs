// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::catalog::COOP_RECEIPT_QUERY_TOOL;
use crate::gateway::{Correlation, GatewayAdapter, GatewayMethod, GatewayRequest};
use crate::json::JsonValue;
use crate::protocol::{METHOD_NOT_FOUND, RpcError, RpcRequest, RpcResponse};
use crate::server::McpServer;
use crate::{
    COOP_RECEIPT_QUERY_ARTIFACT, COOP_RECEIPT_QUERY_GENERATOR, COOP_RECEIPT_QUERY_MAX_GENERATION,
    COOP_RECEIPT_QUERY_PROTOCOL_VERSION, COOP_RECEIPT_QUERY_SCHEMA_DIGEST,
    COOP_RECEIPT_QUERY_SCHEMA_SOURCE,
};

use super::{gateway_error_result, has_only_arguments, headers, invalid_params, tool_result};

#[path = "projection_coop_receipt_query.rs"]
mod projection;
#[path = "mapping_coop_receipt_query_request.rs"]
mod request;

const ARGUMENTS: [&str; 15] = [
    "instance_id",
    "mcp_session_id",
    "lease_id",
    "lease_epoch",
    "operation_id",
    "action_kind",
    "action_fingerprint",
    "run_id",
    "location",
    "actor_id",
    "authority_id",
    "authority_epoch",
    "expected_host_generation",
    "before_host_generation",
    "participant_ids",
];
const MAX_IDENTIFIER_BYTES: usize = 128;
const MAX_BODY_BYTES: usize = 16 * 1024;

pub(super) fn tools_call<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    request: RpcRequest,
) -> RpcResponse {
    let Some(params) = request.params.as_object() else {
        return invalid_params(request.id, "tools/call params must be an object");
    };
    if !has_only_arguments(params, &["name", "arguments"]) {
        return invalid_params(
            request.id,
            "receipt-query tools/call has unsupported fields",
        );
    }
    if params.get("name").and_then(JsonValue::as_string) != Some(COOP_RECEIPT_QUERY_TOOL) {
        return RpcResponse::failure(
            Some(request.id),
            RpcError::new(METHOD_NOT_FOUND, "co-op receipt-query tool is not active"),
        );
    }
    let Some(arguments) = params.get("arguments").and_then(JsonValue::as_object) else {
        return invalid_params(request.id, "receipt-query tool arguments must be an object");
    };
    let id = request.id;
    let correlation = id.stable_text();
    if !safe_header(&correlation) {
        return invalid_params(
            id,
            "request id contains an unsafe or oversized header value",
        );
    }
    let context = match Context::read(server, arguments, &correlation) {
        Ok(context) => context,
        Err(message) => return invalid_params(id, message),
    };
    let body = context.to_request();
    if body.to_json().len() > MAX_BODY_BYTES {
        return invalid_params(id, "receipt-query request exceeds the protocol body limit");
    }
    let request = GatewayRequest {
        method: GatewayMethod::Post,
        path: format!("/v1/instances/{}/coop/receipt-query", context.instance),
        headers: context.headers(),
        body: Some(body),
        correlation: Correlation {
            mcp_session_id: context.mcp_session.clone(),
            mcp_request_id: id.clone(),
        },
    };
    match server.gateway.forward(request) {
        Ok(response) => match projection::project_coop_receipt_query_response(
            &response.body,
            &context,
            response.status,
        ) {
            Ok((body, is_error)) => tool_result(id, body.to_json(), is_error),
            Err(message) => tool_result(id, message, true),
        },
        Err(error) => gateway_error_result(id, error),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Context {
    pub(super) correlation: String,
    pub(super) instance: String,
    pub(super) session: String,
    pub(super) mcp_session: String,
    pub(super) lease: String,
    pub(super) epoch: i64,
    pub(super) operation_id: String,
    pub(super) action_kind: String,
    pub(super) action_fingerprint: String,
    pub(super) run_id: String,
    pub(super) location: JsonValue,
    pub(super) actor_id: String,
    pub(super) authority_id: String,
    pub(super) authority_epoch: String,
    pub(super) expected_host_generation: i64,
    pub(super) before_host_generation: i64,
    pub(super) participant_ids: Vec<String>,
}

impl Context {
    fn read<G: GatewayAdapter>(
        server: &McpServer<G>,
        arguments: &BTreeMap<String, JsonValue>,
        correlation: &str,
    ) -> Result<Self, &'static str> {
        if !has_only_arguments(arguments, &ARGUMENTS) {
            return Err("receipt-query arguments contain an unsupported field");
        }
        let instance = identity(arguments, "instance_id")?;
        let supplied_mcp_session = identity(arguments, "mcp_session_id")?;
        let session = server.gateway_session_id().unwrap_or(supplied_mcp_session);
        let lease = identity(arguments, "lease_id")?;
        let epoch = counter(arguments, "lease_epoch")?;
        if server
            .mcp_session_id()
            .is_some_and(|expected| expected != supplied_mcp_session)
        {
            return Err("MCP session identity does not match the configured session");
        }
        if !crate::mapping::safe_segment(instance)
            || !safe_header(session)
            || !safe_header(correlation)
        {
            return Err("receipt-query transport identity is unsafe or oversized");
        }
        let operation_id = operation_id(arguments, "operation_id")?;
        let action_kind = arguments
            .get("action_kind")
            .and_then(JsonValue::as_string)
            .filter(|value| matches!(*value, "end_turn" | "play_card"))
            .ok_or("action_kind must be end_turn or play_card")?;
        let action_fingerprint = digest(arguments, "action_fingerprint")?;
        let run_id = identity(arguments, "run_id")?;
        let actor_id = identity(arguments, "actor_id")?;
        let authority_id = identity(arguments, "authority_id")?;
        let authority_epoch = identity(arguments, "authority_epoch")?;
        let expected = counter(arguments, "expected_host_generation")?;
        let before = counter(arguments, "before_host_generation")?;
        if expected != before {
            return Err("expected_host_generation must equal before_host_generation");
        }
        let location = location(arguments.get("location"))?;
        let participant_ids = participants(arguments.get("participant_ids"), actor_id)?;
        Ok(Self {
            correlation: correlation.to_owned(),
            instance: instance.to_owned(),
            session: session.to_owned(),
            mcp_session: supplied_mcp_session.to_owned(),
            lease: lease.to_owned(),
            epoch,
            operation_id: operation_id.to_owned(),
            action_kind: action_kind.to_owned(),
            action_fingerprint: action_fingerprint.to_owned(),
            run_id: run_id.to_owned(),
            location,
            actor_id: actor_id.to_owned(),
            authority_id: authority_id.to_owned(),
            authority_epoch: authority_epoch.to_owned(),
            expected_host_generation: expected,
            before_host_generation: before,
            participant_ids,
        })
    }

    fn headers(&self) -> BTreeMap<String, String> {
        let mut result = headers(&self.mcp_session, &self.correlation);
        result.extend([
            (String::from("x-sts2-instance-id"), self.instance.clone()),
            (String::from("x-sts2-session-id"), self.session.clone()),
            (String::from("x-sts2-lease-id"), self.lease.clone()),
            (String::from("x-sts2-lease-epoch"), self.epoch.to_string()),
        ]);
        result
    }
}

fn identity<'a>(
    arguments: &'a BTreeMap<String, JsonValue>,
    name: &str,
) -> Result<&'a str, &'static str> {
    arguments
        .get(name)
        .and_then(JsonValue::as_string)
        .filter(|value| safe_header(value))
        .ok_or("receipt-query identity is missing, unsafe, or oversized")
}

fn operation_id<'a>(
    arguments: &'a BTreeMap<String, JsonValue>,
    name: &str,
) -> Result<&'a str, &'static str> {
    identity(arguments, name).and_then(|value| {
        if value.contains('/') {
            Err("operation_id must not contain '/'")
        } else {
            Ok(value)
        }
    })
}

fn digest<'a>(
    arguments: &'a BTreeMap<String, JsonValue>,
    name: &str,
) -> Result<&'a str, &'static str> {
    let value = arguments
        .get(name)
        .and_then(JsonValue::as_string)
        .ok_or("action_fingerprint must be a lowercase hexadecimal digest")?;
    if value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Ok(value)
    } else {
        Err("action_fingerprint must be a lowercase hexadecimal digest")
    }
}

fn counter(arguments: &BTreeMap<String, JsonValue>, name: &str) -> Result<i64, &'static str> {
    match arguments.get(name) {
        Some(JsonValue::Number(value))
            if (0..=COOP_RECEIPT_QUERY_MAX_GENERATION).contains(value) =>
        {
            Ok(*value)
        }
        _ => Err("receipt-query generation or lease_epoch is outside the protocol bound"),
    }
}

#[path = "mapping_coop_receipt_query_location.rs"]
mod location;

use location::{location, participants};

fn safe_header(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_IDENTIFIER_BYTES
        && !value.contains("..")
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':' | b'/')
        })
}

#[cfg(test)]
#[path = "mapping_coop_receipt_query_tests.rs"]
mod tests;
