// SPDX-License-Identifier: MIT

use std::sync::OnceLock;

use crate::json::JsonValue;

pub(super) const NEUTRAL_SCHEMA: &[u8] =
    include_bytes!("../../../../protocol-artifact/exact-restore-v1/schema.json");
pub(super) const GATEWAY_SCHEMA: &[u8] =
    include_bytes!("../../../../protocol-artifact/exact-restore-gateway-v1/schema.json");

static NEUTRAL_VALIDATOR: OnceLock<Result<jsonschema::Validator, ()>> = OnceLock::new();
static GATEWAY_VALIDATOR: OnceLock<Result<jsonschema::Validator, ()>> = OnceLock::new();

pub(super) fn validate_neutral(value: &JsonValue) -> Result<(), &'static str> {
    validate(value, NEUTRAL_SCHEMA, &NEUTRAL_VALIDATOR)
}

pub(super) fn validate_gateway(value: &JsonValue) -> Result<(), &'static str> {
    validate(value, GATEWAY_SCHEMA, &GATEWAY_VALIDATOR)
}

pub(super) fn gateway_request_schema() -> JsonValue {
    let Ok(text) = std::str::from_utf8(GATEWAY_SCHEMA) else {
        return JsonValue::Null;
    };
    let Ok(schema) = crate::json::parse(text) else {
        return JsonValue::Null;
    };
    let Some(definitions) = schema.as_object().and_then(|object| object.get("$defs")) else {
        return JsonValue::Null;
    };
    JsonValue::object([
        (
            String::from("$schema"),
            JsonValue::string("https://json-schema.org/draft/2020-12/schema"),
        ),
        (String::from("$defs"), definitions.clone()),
        (String::from("$ref"), JsonValue::string("#/$defs/request")),
    ])
}

fn validate(
    value: &JsonValue,
    schema_bytes: &[u8],
    validator_slot: &'static OnceLock<Result<jsonschema::Validator, ()>>,
) -> Result<(), &'static str> {
    let schema_text = std::str::from_utf8(schema_bytes).map_err(|_| "pinned schema is invalid")?;
    let validator = validator_slot
        .get_or_init(|| {
            let schema = serde_json::from_str(schema_text).map_err(|_| ())?;
            jsonschema::draft202012::options()
                .build(&schema)
                .map_err(|_| ())
        })
        .as_ref()
        .map_err(|_| "pinned schema validator is unavailable")?;
    let value: serde_json::Value =
        serde_json::from_str(&value.to_json()).map_err(|_| "frame JSON is invalid")?;
    if validator.is_valid(&value) {
        Ok(())
    } else {
        Err("frame does not match its pinned schema")
    }
}
