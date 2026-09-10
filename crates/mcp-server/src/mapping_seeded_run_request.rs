// SPDX-License-Identifier: MIT

use crate::json::JsonValue;
use crate::protocol_artifact_seeded_run::{
    SEEDED_RUN_ARTIFACT, SEEDED_RUN_GENERATOR, SEEDED_RUN_PROTOCOL_VERSION,
    SEEDED_RUN_SCHEMA_DIGEST, SEEDED_RUN_SCHEMA_SOURCE,
};

use super::SeededContext;

pub(super) fn start_request(context: &SeededContext, seed: &str, run_mode: &str) -> JsonValue {
    let mut body = base_message(context, "start_request", &context.operation_id);
    if let JsonValue::Object(object) = &mut body {
        object.insert(String::from("requested_seed"), JsonValue::string(seed));
        object.insert(String::from("run_mode"), JsonValue::string(run_mode));
    }
    body
}

pub(super) fn base_message(context: &SeededContext, kind: &str, operation_id: &str) -> JsonValue {
    JsonValue::object([
        (
            String::from("protocol_version"),
            JsonValue::string(SEEDED_RUN_PROTOCOL_VERSION),
        ),
        (
            String::from("schema_digest"),
            JsonValue::string(SEEDED_RUN_SCHEMA_DIGEST),
        ),
        (
            String::from("provenance"),
            JsonValue::object([
                (
                    String::from("artifact"),
                    JsonValue::string(SEEDED_RUN_ARTIFACT),
                ),
                (
                    String::from("source"),
                    JsonValue::string(SEEDED_RUN_SCHEMA_SOURCE),
                ),
                (
                    String::from("generator"),
                    JsonValue::string(SEEDED_RUN_GENERATOR),
                ),
            ]),
        ),
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
            JsonValue::string(operation_id),
        ),
        (String::from("requested_seed"), JsonValue::Null),
        (String::from("run_mode"), JsonValue::Null),
        (
            String::from("context_digest"),
            context
                .context_digest
                .as_deref()
                .map_or(JsonValue::Null, JsonValue::string),
        ),
        (
            String::from("selected_context"),
            context.selected_context.clone().unwrap_or(JsonValue::Null),
        ),
        (String::from("status"), JsonValue::Null),
        (String::from("canonical_seed"), JsonValue::Null),
        (String::from("observation"), JsonValue::Null),
        (String::from("effect_witness"), JsonValue::Null),
        (String::from("error_code"), JsonValue::Null),
    ])
}
