// SPDX-License-Identifier: MIT

use crate::json::JsonValue;

const MAX_INTEGER: i64 = 9_007_199_254_740_991;
const UUID_PATTERN: &str = "^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$";
const UUID4_PATTERN: &str = "^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$";
const DIGEST_PATTERN: &str = "^[0-9a-f]{64}$";
const TOKEN_PATTERN: &str = "^[A-Za-z0-9_-]{43}$";
const TIMESTAMP_PATTERN: &str =
    "^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}(?:\\.[0-9]{1,9})?Z$";
pub(super) fn release_set() -> JsonValue {
    let fields = [
        "release_digest",
        "config_digest",
        "profile_digest",
        "runtime_v3_schema_digest",
    ];
    closed(&fields, fields.map(|key| (key, digest())))
}

pub(super) fn lease_policy() -> JsonValue {
    renewal_before_ttl(closed(
        &["ttl_seconds", "renewal_interval_seconds"],
        [
            ("ttl_seconds", integer(5, 300)),
            ("renewal_interval_seconds", integer(1, 100)),
        ],
    ))
}

pub(super) fn boot_context() -> JsonValue {
    closed(
        &[
            "deployment_id",
            "instance_id",
            "instance_incarnation",
            "boot_id",
            "authority_generation",
            "release",
            "created_at",
            "state",
        ],
        [
            ("deployment_id", uuid()),
            ("instance_id", uuid()),
            ("instance_incarnation", uuid4()),
            ("boot_id", uuid4()),
            ("authority_generation", positive_integer()),
            ("release", release_set()),
            ("created_at", timestamp()),
            (
                "state",
                enum_value(&["FENCE_REQUIRED", "READY", "BLOCKED", "REVOKED"]),
            ),
        ],
    )
}

pub(super) fn host_fence_context() -> JsonValue {
    closed(
        &[
            "host_fence_id",
            "deployment_id",
            "instance_id",
            "instance_incarnation",
            "boot_id",
            "authority_generation",
            "fence_generation",
            "created_at",
        ],
        [
            ("host_fence_id", uuid4()),
            ("deployment_id", uuid()),
            ("instance_id", uuid()),
            ("instance_incarnation", uuid4()),
            ("boot_id", uuid4()),
            ("authority_generation", positive_integer()),
            ("fence_generation", positive_integer()),
            ("created_at", timestamp()),
        ],
    )
}

pub(super) fn lease_context() -> JsonValue {
    renewal_before_ttl(closed(
        &[
            "deployment_id",
            "instance_id",
            "instance_incarnation",
            "boot_id",
            "authority_generation",
            "lease_id",
            "lease_epoch",
            "fence_token",
            "issued_at",
            "expires_at",
            "ttl_seconds",
            "renewal_interval_seconds",
        ],
        [
            ("deployment_id", uuid()),
            ("instance_id", uuid()),
            ("instance_incarnation", uuid4()),
            ("boot_id", uuid4()),
            ("authority_generation", positive_integer()),
            ("lease_id", uuid4()),
            ("lease_epoch", positive_integer()),
            ("fence_token", bounded_token()),
            ("issued_at", timestamp()),
            ("expires_at", timestamp()),
            ("ttl_seconds", integer(5, 300)),
            ("renewal_interval_seconds", integer(1, 100)),
        ],
    ))
}

fn renewal_before_ttl(mut schema: JsonValue) -> JsonValue {
    let bounds = (5_i64..=100).map(renewal_bound).collect();
    if let Some(object) = schema.as_object_mut() {
        object.insert("allOf".to_owned(), JsonValue::Array(bounds));
    }
    schema
}

fn renewal_bound(ttl: i64) -> JsonValue {
    JsonValue::object([
        (
            "if".to_owned(),
            JsonValue::object([(
                "properties".to_owned(),
                JsonValue::object([("ttl_seconds".to_owned(), const_integer(ttl))]),
            )]),
        ),
        (
            "then".to_owned(),
            JsonValue::object([(
                "properties".to_owned(),
                JsonValue::object([("renewal_interval_seconds".to_owned(), integer(1, ttl - 1))]),
            )]),
        ),
    ])
}

fn original_context() -> JsonValue {
    closed(
        &[
            "deployment_id",
            "instance_id",
            "instance_incarnation",
            "boot_id",
            "authority_generation",
            "lease_id",
            "lease_epoch",
        ],
        [
            ("deployment_id", uuid()),
            ("instance_id", uuid()),
            ("instance_incarnation", uuid4()),
            ("boot_id", uuid4()),
            ("authority_generation", positive_integer()),
            ("lease_id", uuid4()),
            ("lease_epoch", positive_integer()),
        ],
    )
}

fn expected_boundary() -> JsonValue {
    closed(
        &["state_id", "generation", "catalog_digest"],
        [
            ("state_id", uuid()),
            ("generation", integer(0, MAX_INTEGER)),
            ("catalog_digest", digest()),
        ],
    )
}

fn v3_action() -> JsonValue {
    closed(
        &["schema_digest", "canonical_json_b64", "payload_digest"],
        [
            ("schema_digest", digest()),
            (
                "canonical_json_b64",
                string("^[A-Za-z0-9+/=_-]+$", 1, 65_536),
            ),
            ("payload_digest", digest()),
        ],
    )
}

pub(super) fn operation_intent_context() -> JsonValue {
    closed(
        &[
            "operation_id",
            "payload_digest",
            "original_context",
            "expected_boundary",
            "action",
        ],
        [
            ("operation_id", uuid4()),
            ("payload_digest", digest()),
            ("original_context", original_context()),
            ("expected_boundary", expected_boundary()),
            ("action", v3_action()),
        ],
    )
}

pub(super) fn operation_ref() -> JsonValue {
    closed(
        &["operation_id", "payload_digest", "original_context"],
        [
            ("operation_id", uuid4()),
            ("payload_digest", digest()),
            ("original_context", original_context()),
        ],
    )
}

pub(super) fn closed(
    required: &[&str],
    properties: impl IntoIterator<Item = (&'static str, JsonValue)>,
) -> JsonValue {
    JsonValue::object([
        ("type".to_owned(), JsonValue::string("object")),
        ("additionalProperties".to_owned(), JsonValue::Bool(false)),
        (
            "required".to_owned(),
            JsonValue::Array(required.iter().map(|key| JsonValue::string(*key)).collect()),
        ),
        (
            "properties".to_owned(),
            JsonValue::object(
                properties
                    .into_iter()
                    .map(|(key, value)| (key.to_owned(), value)),
            ),
        ),
    ])
}

pub(super) fn uuid() -> JsonValue {
    string(UUID_PATTERN, 36, 36)
}

pub(super) fn uuid4() -> JsonValue {
    string(UUID4_PATTERN, 36, 36)
}

fn digest() -> JsonValue {
    string(DIGEST_PATTERN, 64, 64)
}

fn bounded_token() -> JsonValue {
    string(TOKEN_PATTERN, 43, 43)
}

fn timestamp() -> JsonValue {
    JsonValue::object([
        ("type".to_owned(), JsonValue::string("string")),
        ("format".to_owned(), JsonValue::string("date-time")),
        ("pattern".to_owned(), JsonValue::string(TIMESTAMP_PATTERN)),
        ("minLength".to_owned(), JsonValue::Number(20)),
        ("maxLength".to_owned(), JsonValue::Number(30)),
    ])
}

pub(super) fn bounded_string(pattern: &str) -> JsonValue {
    string(pattern, 1, 128)
}

pub(super) fn positive_integer() -> JsonValue {
    integer(1, MAX_INTEGER)
}

fn integer(minimum: i64, maximum: i64) -> JsonValue {
    JsonValue::object([
        ("type".to_owned(), JsonValue::string("integer")),
        ("minimum".to_owned(), JsonValue::Number(minimum)),
        ("maximum".to_owned(), JsonValue::Number(maximum)),
    ])
}

fn string(pattern: &str, minimum: i64, maximum: i64) -> JsonValue {
    JsonValue::object([
        ("type".to_owned(), JsonValue::string("string")),
        ("pattern".to_owned(), JsonValue::string(pattern)),
        ("minLength".to_owned(), JsonValue::Number(minimum)),
        ("maxLength".to_owned(), JsonValue::Number(maximum)),
    ])
}

pub(super) fn enum_value(values: &[&str]) -> JsonValue {
    JsonValue::object([
        ("type".to_owned(), JsonValue::string("string")),
        (
            "enum".to_owned(),
            JsonValue::Array(
                values
                    .iter()
                    .map(|value| JsonValue::string(*value))
                    .collect(),
            ),
        ),
    ])
}

pub(super) fn const_value(value: &str) -> JsonValue {
    JsonValue::object([("const".to_owned(), JsonValue::string(value))])
}

fn const_integer(value: i64) -> JsonValue {
    JsonValue::object([("const".to_owned(), JsonValue::Number(value))])
}
