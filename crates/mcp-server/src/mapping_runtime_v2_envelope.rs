// SPDX-License-Identifier: MIT

use crate::json::JsonValue;
use crate::projection::RuntimeV2Context;
use crate::protocol_artifact_runtime_v2::{
    RUNTIME_V2_ACTION_ID, RUNTIME_V2_ARTIFACT, RUNTIME_V2_GENERATOR, RUNTIME_V2_PROTOCOL_VERSION,
    RUNTIME_V2_SCHEMA_DIGEST, RUNTIME_V2_SCHEMA_SOURCE,
};

pub(super) fn request_envelope(
    context: &RuntimeV2Context,
    kind: &str,
    include_action: bool,
    include_operation: bool,
) -> JsonValue {
    JsonValue::object([
        (
            String::from("protocol_version"),
            JsonValue::string(RUNTIME_V2_PROTOCOL_VERSION),
        ),
        (
            String::from("schema_digest"),
            JsonValue::string(RUNTIME_V2_SCHEMA_DIGEST),
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
            if include_operation {
                JsonValue::string(context.operation_id.as_str())
            } else {
                JsonValue::Null
            },
        ),
        (String::from("observation"), JsonValue::Null),
        (
            String::from("action"),
            if include_action {
                JsonValue::object([(
                    String::from("action_id"),
                    JsonValue::string(RUNTIME_V2_ACTION_ID),
                )])
            } else {
                JsonValue::Null
            },
        ),
        (String::from("status"), JsonValue::Null),
        (String::from("error_code"), JsonValue::Null),
        (String::from("effect_witness"), JsonValue::Null),
    ])
}

pub(super) fn result_envelope(
    context: &RuntimeV2Context,
    kind: &str,
    status: &str,
    error_code: &str,
    observation: Option<JsonValue>,
    effect_witness: Option<JsonValue>,
) -> JsonValue {
    JsonValue::object([
        (
            String::from("protocol_version"),
            JsonValue::string(RUNTIME_V2_PROTOCOL_VERSION),
        ),
        (
            String::from("schema_digest"),
            JsonValue::string(RUNTIME_V2_SCHEMA_DIGEST),
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
            JsonValue::string(context.operation_id.as_str()),
        ),
        (
            String::from("observation"),
            observation.unwrap_or(JsonValue::Null),
        ),
        (
            String::from("action"),
            JsonValue::object([(
                String::from("action_id"),
                JsonValue::string(RUNTIME_V2_ACTION_ID),
            )]),
        ),
        (String::from("status"), JsonValue::string(status)),
        (
            String::from("error_code"),
            if error_code.is_empty() {
                JsonValue::Null
            } else {
                JsonValue::string(error_code)
            },
        ),
        (
            String::from("effect_witness"),
            effect_witness.unwrap_or(JsonValue::Null),
        ),
    ])
}

fn provenance() -> JsonValue {
    JsonValue::object([
        (
            String::from("artifact"),
            JsonValue::string(RUNTIME_V2_ARTIFACT),
        ),
        (
            String::from("source"),
            JsonValue::string(RUNTIME_V2_SCHEMA_SOURCE),
        ),
        (
            String::from("generator"),
            JsonValue::string(RUNTIME_V2_GENERATOR),
        ),
    ])
}
