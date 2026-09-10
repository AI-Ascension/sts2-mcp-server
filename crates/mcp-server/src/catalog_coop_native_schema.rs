// SPDX-License-Identifier: MIT

use crate::json::JsonValue;

use super::{generation, identity, nullable};

pub(super) fn recovery_schema() -> JsonValue {
    JsonValue::object([
        (String::from("type"), JsonValue::string("object")),
        (String::from("additionalProperties"), JsonValue::Bool(false)),
        (
            String::from("required"),
            JsonValue::Array(
                ["kind", "rejoin_epoch"]
                    .into_iter()
                    .map(JsonValue::string)
                    .collect(),
            ),
        ),
        (
            String::from("properties"),
            JsonValue::object([
                (
                    String::from("kind"),
                    JsonValue::object([(
                        String::from("enum"),
                        JsonValue::Array(
                            ["reconcile", "rejoin"]
                                .into_iter()
                                .map(JsonValue::string)
                                .collect(),
                        ),
                    )]),
                ),
                (String::from("rejoin_epoch"), generation()),
            ]),
        ),
    ])
}

pub(super) fn effect_envelope_schema() -> JsonValue {
    let required = [
        "protocol_version",
        "schema_digest",
        "provenance",
        "correlation_id",
        "instance_id",
        "session_id",
        "lease_id",
        "lease_epoch",
        "kind",
        "operation_id",
        "actor_peer",
        "expected_host_generation",
        "action",
        "vote",
        "status",
        "observation",
        "effect",
        "recovery",
        "catalog",
        "receipt",
    ];
    let properties = vec![
        (
            String::from("protocol_version"),
            identity("^coop-native-v1$"),
        ),
        (String::from("schema_digest"), identity("^[0-9a-f]{64}$")),
        (
            String::from("provenance"),
            JsonValue::object([(String::from("type"), JsonValue::string("object"))]),
        ),
        (
            String::from("correlation_id"),
            identity("^[A-Za-z0-9_.:/-]{1,128}$"),
        ),
        (
            String::from("instance_id"),
            identity("^[A-Za-z0-9_.:/-]{1,128}$"),
        ),
        (
            String::from("session_id"),
            identity("^[A-Za-z0-9_.:/-]{1,128}$"),
        ),
        (
            String::from("lease_id"),
            identity("^[A-Za-z0-9_.:/-]{1,128}$"),
        ),
        (String::from("lease_epoch"), generation()),
        (
            String::from("kind"),
            JsonValue::object([(String::from("const"), JsonValue::string("effect_response"))]),
        ),
        (
            String::from("operation_id"),
            identity("^[A-Za-z0-9_.:/-]{1,128}$"),
        ),
        (
            String::from("actor_peer"),
            JsonValue::object([(String::from("type"), JsonValue::string("null"))]),
        ),
        (
            String::from("expected_host_generation"),
            JsonValue::object([(String::from("type"), JsonValue::string("null"))]),
        ),
        (
            String::from("action"),
            JsonValue::object([(String::from("type"), JsonValue::string("null"))]),
        ),
        (
            String::from("vote"),
            JsonValue::object([(String::from("type"), JsonValue::string("null"))]),
        ),
        (
            String::from("status"),
            identity("^(accepted|settled|rejected|unknown)$"),
        ),
        (
            String::from("observation"),
            JsonValue::object([(String::from("type"), JsonValue::string("object"))]),
        ),
        (
            String::from("effect"),
            nullable(JsonValue::object([(
                String::from("type"),
                JsonValue::string("object"),
            )])),
        ),
        (
            String::from("recovery"),
            JsonValue::object([(String::from("type"), JsonValue::string("null"))]),
        ),
        (
            String::from("catalog"),
            JsonValue::object([(String::from("type"), JsonValue::string("null"))]),
        ),
        (String::from("receipt"), receipt_schema()),
    ];
    JsonValue::object([
        (String::from("type"), JsonValue::string("object")),
        (String::from("additionalProperties"), JsonValue::Bool(false)),
        (
            String::from("required"),
            JsonValue::Array(required.into_iter().map(JsonValue::string).collect()),
        ),
        (String::from("properties"), JsonValue::object(properties)),
    ])
}

fn receipt_schema() -> JsonValue {
    JsonValue::object([
        (String::from("type"), JsonValue::string("object")),
        (String::from("additionalProperties"), JsonValue::Bool(false)),
        (
            String::from("required"),
            JsonValue::Array(
                [
                    "operation_id",
                    "status",
                    "before_host_generation",
                    "after_host_generation",
                    "authority_id",
                    "authority_epoch",
                    "checkpoint_id",
                    "state_digest",
                    "native_checksum",
                    "error_code",
                ]
                .into_iter()
                .map(JsonValue::string)
                .collect(),
            ),
        ),
        (
            String::from("properties"),
            JsonValue::object([
                (
                    String::from("operation_id"),
                    identity("^[A-Za-z0-9_.:/-]{1,512}$"),
                ),
                (
                    String::from("status"),
                    identity("^(accepted|settled|rejected|unknown)$"),
                ),
                (String::from("before_host_generation"), generation()),
                (
                    String::from("after_host_generation"),
                    nullable(generation()),
                ),
                (
                    String::from("authority_id"),
                    identity("^[A-Za-z0-9_.:/-]{1,512}$"),
                ),
                (
                    String::from("authority_epoch"),
                    identity("^[A-Za-z0-9_.:/-]{1,512}$"),
                ),
                (
                    String::from("checkpoint_id"),
                    identity("^[A-Za-z0-9_.:/-]{1,512}$"),
                ),
                (String::from("state_digest"), identity("^[0-9a-f]{64}$")),
                (
                    String::from("native_checksum"),
                    nullable(identity("^[0-9a-f]{64}$")),
                ),
                (
                    String::from("error_code"),
                    nullable(identity("^[A-Za-z0-9_.:/-]{1,512}$")),
                ),
            ]),
        ),
    ])
}
