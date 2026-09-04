// SPDX-License-Identifier: MIT

use crate::json::JsonValue;

use super::context::RuntimeV3GameplayContext;

const PROTOCOL_VERSION: &str = "runtime-v3-gameplay";
const SCHEMA_DIGEST: &str = "fbfb18279b0c7ebb350ef0ce0d56547fa11e83985b13380cb2b0f1dba4cb56e9";
const ARTIFACT: &str = "sts2-protocol/runtime-v3-gameplay";
const SOURCE: &str = "schemas/runtime-v3-gameplay.schema.json";
const GENERATOR: &str = "hand-authored";

pub(super) fn request_envelope(
    context: &RuntimeV3GameplayContext,
    kind: &str,
    state_id: Option<&str>,
    operation_id: Option<&str>,
    action: Option<JsonValue>,
    wait_for_millis: Option<i64>,
    recovery: Option<JsonValue>,
) -> JsonValue {
    JsonValue::object([
        (
            String::from("protocol_version"),
            JsonValue::string(PROTOCOL_VERSION),
        ),
        (
            String::from("schema_digest"),
            JsonValue::string(SCHEMA_DIGEST),
        ),
        (String::from("provenance"), provenance()),
        (
            String::from("correlation_id"),
            JsonValue::string(context.correlation_id()),
        ),
        (
            String::from("instance_id"),
            JsonValue::string(context.instance_id()),
        ),
        (
            String::from("session_id"),
            JsonValue::string(context.session_id()),
        ),
        (
            String::from("lease_id"),
            JsonValue::string(context.projection.lease_id.as_str()),
        ),
        (
            String::from("lease_epoch"),
            JsonValue::Number(context.projection.lease_epoch),
        ),
        (
            String::from("generation"),
            JsonValue::Number(context.generation()),
        ),
        (String::from("kind"), JsonValue::string(kind)),
        (
            String::from("state_id"),
            state_id.map_or(JsonValue::Null, JsonValue::string),
        ),
        (
            String::from("operation_id"),
            operation_id.map_or(JsonValue::Null, JsonValue::string),
        ),
        (String::from("observation"), JsonValue::Null),
        (String::from("legal_actions"), JsonValue::Null),
        (
            String::from("action"),
            action.map_or(JsonValue::Null, |value| value),
        ),
        (String::from("status"), JsonValue::Null),
        (String::from("transition"), JsonValue::Null),
        (String::from("error_code"), JsonValue::Null),
        (
            String::from("wait_for_millis"),
            wait_for_millis.map_or(JsonValue::Null, JsonValue::Number),
        ),
        (String::from("wait_outcome"), JsonValue::Null),
        (
            String::from("recovery"),
            recovery.map_or(JsonValue::Null, |value| value),
        ),
    ])
}

pub(super) fn unknown_response(
    context: &RuntimeV3GameplayContext,
    kind: &str,
    error_code: &str,
) -> JsonValue {
    let wait_outcome = if kind == "wait_response" {
        JsonValue::string("timeout")
    } else {
        JsonValue::Null
    };
    let operation_id = context
        .operation_id
        .as_deref()
        .or((kind == "recover_response").then_some(context.correlation_id()));
    JsonValue::object([
        (
            String::from("protocol_version"),
            JsonValue::string(PROTOCOL_VERSION),
        ),
        (
            String::from("schema_digest"),
            JsonValue::string(SCHEMA_DIGEST),
        ),
        (String::from("provenance"), provenance()),
        (
            String::from("correlation_id"),
            JsonValue::string(context.correlation_id()),
        ),
        (
            String::from("instance_id"),
            JsonValue::string(context.instance_id()),
        ),
        (
            String::from("session_id"),
            JsonValue::string(context.session_id()),
        ),
        (
            String::from("lease_id"),
            JsonValue::string(context.projection.lease_id.as_str()),
        ),
        (
            String::from("lease_epoch"),
            JsonValue::Number(context.projection.lease_epoch),
        ),
        (
            String::from("generation"),
            JsonValue::Number(context.generation()),
        ),
        (String::from("kind"), JsonValue::string(kind)),
        (String::from("state_id"), JsonValue::Null),
        (
            String::from("operation_id"),
            operation_id.map_or(JsonValue::Null, JsonValue::string),
        ),
        (String::from("observation"), JsonValue::Null),
        (String::from("legal_actions"), JsonValue::Null),
        (String::from("action"), JsonValue::Null),
        (String::from("status"), JsonValue::string("unknown")),
        (String::from("transition"), JsonValue::Null),
        (String::from("error_code"), JsonValue::string(error_code)),
        (String::from("wait_for_millis"), JsonValue::Null),
        (String::from("wait_outcome"), wait_outcome),
        (String::from("recovery"), JsonValue::Null),
    ])
}

fn provenance() -> JsonValue {
    JsonValue::object([
        (String::from("artifact"), JsonValue::string(ARTIFACT)),
        (String::from("source"), JsonValue::string(SOURCE)),
        (String::from("generator"), JsonValue::string(GENERATOR)),
    ])
}
