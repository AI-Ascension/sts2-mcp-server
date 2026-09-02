// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::gateway::{Correlation, GatewayAdapter, GatewayMethod, GatewayRequest};
use crate::json::JsonValue;
use crate::protocol::{RequestId, RpcResponse};
use crate::protocol_artifact::{
    RUNTIME_ACTION_ID, RUNTIME_ARTIFACT, RUNTIME_GENERATOR, RUNTIME_MAX_GENERATION,
    RUNTIME_PROTOCOL_VERSION, RUNTIME_SCHEMA_DIGEST, RUNTIME_SCHEMA_SOURCE,
};
use crate::server::McpServer;

use super::{
    forward, has_only_arguments, headers, invalid_params, non_empty_string, nonnegative_integer,
    request_context,
};

pub(super) fn runtime_action_call<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    id: RequestId,
    arguments: &BTreeMap<String, JsonValue>,
    correlation_id: &str,
) -> RpcResponse {
    if !has_only_arguments(
        arguments,
        &["instance_id", "mcp_session_id", "generation", "action_id"],
    ) {
        return invalid_params(id, "tools/call arguments contain an unsupported field");
    }
    let (instance_id, session_id) = match request_context(arguments) {
        Ok(context) => context,
        Err(message) => {
            return invalid_params(id, message);
        }
    };
    let Some(generation) = nonnegative_integer(arguments, "generation")
        .filter(|value| *value <= RUNTIME_MAX_GENERATION)
    else {
        return invalid_params(id, "generation exceeds the runtime protocol bound");
    };
    let Some(action_id) = non_empty_string(arguments, "action_id") else {
        return invalid_params(id, "action_id must be a non-empty string");
    };
    if action_id != RUNTIME_ACTION_ID {
        return invalid_params(id, "action_id must be show_runtime_probe");
    }
    let gateway_request = GatewayRequest {
        method: GatewayMethod::Post,
        path: format!("/v1/instances/{instance_id}/action"),
        headers: headers(session_id, correlation_id),
        body: Some(runtime_action_request(
            correlation_id,
            instance_id,
            session_id,
            generation,
            action_id,
        )),
        correlation: Correlation {
            mcp_session_id: String::from(session_id),
            mcp_request_id: id.clone(),
        },
    };
    forward(server, id, gateway_request)
}

fn runtime_action_request(
    correlation_id: &str,
    instance_id: &str,
    session_id: &str,
    generation: i64,
    action_id: &str,
) -> JsonValue {
    JsonValue::object([
        (
            "action".to_owned(),
            JsonValue::object([("action_id".to_owned(), JsonValue::string(action_id))]),
        ),
        (
            "correlation_id".to_owned(),
            JsonValue::string(correlation_id),
        ),
        ("effect_witness".to_owned(), JsonValue::Null),
        ("error_code".to_owned(), JsonValue::Null),
        ("generation".to_owned(), JsonValue::Number(generation)),
        ("instance_id".to_owned(), JsonValue::string(instance_id)),
        ("kind".to_owned(), JsonValue::string("action_request")),
        ("lease_epoch".to_owned(), JsonValue::Number(0)),
        ("lease_id".to_owned(), JsonValue::string("lease-pending")),
        ("observation".to_owned(), JsonValue::Null),
        (
            "protocol_version".to_owned(),
            JsonValue::string(RUNTIME_PROTOCOL_VERSION),
        ),
        (
            "provenance".to_owned(),
            JsonValue::object([
                ("artifact".to_owned(), JsonValue::string(RUNTIME_ARTIFACT)),
                ("generator".to_owned(), JsonValue::string(RUNTIME_GENERATOR)),
                (
                    "source".to_owned(),
                    JsonValue::string(RUNTIME_SCHEMA_SOURCE),
                ),
            ]),
        ),
        (
            "schema_digest".to_owned(),
            JsonValue::string(RUNTIME_SCHEMA_DIGEST),
        ),
        ("session_id".to_owned(), JsonValue::string(session_id)),
        ("status".to_owned(), JsonValue::Null),
    ])
}
