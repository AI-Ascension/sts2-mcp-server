// SPDX-License-Identifier: MIT

use crate::json::JsonValue;
use crate::projection::RuntimeV3GameplayContext;
use crate::protocol_artifact_runtime_v3_gameplay::{
    RUNTIME_V3_GAMEPLAY_ACTION_ID, RUNTIME_V3_GAMEPLAY_ARTIFACT, RUNTIME_V3_GAMEPLAY_GENERATOR,
    RUNTIME_V3_GAMEPLAY_PROTOCOL_VERSION, RUNTIME_V3_GAMEPLAY_SCHEMA_DIGEST,
    RUNTIME_V3_GAMEPLAY_SCHEMA_SOURCE,
};

pub(super) fn action_request(
    context: &RuntimeV3GameplayContext,
    card_index: i64,
    target_id: Option<String>,
) -> JsonValue {
    base(
        context,
        "action_request",
        Some(context.operation_id.clone()),
        Some(JsonValue::object([
            (
                String::from("action_id"),
                JsonValue::string(RUNTIME_V3_GAMEPLAY_ACTION_ID),
            ),
            (String::from("card_index"), JsonValue::Number(card_index)),
            (
                String::from("target_id"),
                target_id.map_or(JsonValue::Null, JsonValue::String),
            ),
        ])),
        None,
        None,
        None,
    )
}

pub(super) fn result_envelope(
    context: &RuntimeV3GameplayContext,
    kind: &str,
    status: &str,
    error_code: &str,
    observation: Option<JsonValue>,
) -> JsonValue {
    base(
        context,
        kind,
        Some(context.operation_id.clone()),
        Some(JsonValue::object([
            (
                String::from("action_id"),
                JsonValue::string(RUNTIME_V3_GAMEPLAY_ACTION_ID),
            ),
            (
                String::from("card_index"),
                JsonValue::Number(context.card_index),
            ),
            (
                String::from("target_id"),
                context
                    .target_id
                    .clone()
                    .map_or(JsonValue::Null, JsonValue::String),
            ),
        ])),
        Some(observation.unwrap_or(JsonValue::Null)),
        Some(JsonValue::string(status)),
        Some(if error_code.is_empty() {
            JsonValue::Null
        } else {
            JsonValue::string(error_code)
        }),
    )
}

fn base(
    context: &RuntimeV3GameplayContext,
    kind: &str,
    operation_id: Option<String>,
    action: Option<JsonValue>,
    observation: Option<JsonValue>,
    status: Option<JsonValue>,
    error_code: Option<JsonValue>,
) -> JsonValue {
    JsonValue::object([
        (
            String::from("protocol_version"),
            JsonValue::string(RUNTIME_V3_GAMEPLAY_PROTOCOL_VERSION),
        ),
        (
            String::from("schema_digest"),
            JsonValue::string(RUNTIME_V3_GAMEPLAY_SCHEMA_DIGEST),
        ),
        (String::from("provenance"), provenance()),
        (
            String::from("correlation_id"),
            JsonValue::string(context.correlation_id.as_str()),
        ),
        (
            String::from("instance_id"),
            JsonValue::string(context.instance_id.as_str()),
        ),
        (
            String::from("session_id"),
            JsonValue::string(context.session_id.as_str()),
        ),
        (
            String::from("lease_id"),
            JsonValue::string(context.lease_id.as_str()),
        ),
        (
            String::from("lease_epoch"),
            JsonValue::Number(context.lease_epoch),
        ),
        (
            String::from("generation"),
            JsonValue::Number(context.generation),
        ),
        (String::from("kind"), JsonValue::string(kind)),
        (
            String::from("operation_id"),
            operation_id.map_or(JsonValue::Null, JsonValue::String),
        ),
        (
            String::from("observation"),
            observation.unwrap_or(JsonValue::Null),
        ),
        (String::from("action"), action.unwrap_or(JsonValue::Null)),
        (String::from("status"), status.unwrap_or(JsonValue::Null)),
        (
            String::from("error_code"),
            error_code.unwrap_or(JsonValue::Null),
        ),
        (String::from("effect_witness"), JsonValue::Null),
    ])
}

fn provenance() -> JsonValue {
    JsonValue::object([
        (
            String::from("artifact"),
            JsonValue::string(RUNTIME_V3_GAMEPLAY_ARTIFACT),
        ),
        (
            String::from("source"),
            JsonValue::string(RUNTIME_V3_GAMEPLAY_SCHEMA_SOURCE),
        ),
        (
            String::from("generator"),
            JsonValue::string(RUNTIME_V3_GAMEPLAY_GENERATOR),
        ),
    ])
}
