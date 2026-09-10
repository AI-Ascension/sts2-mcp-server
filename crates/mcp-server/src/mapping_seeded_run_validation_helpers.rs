// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::json::JsonValue;

pub(crate) fn required_object<'a>(
    object: &'a BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<&'a BTreeMap<String, JsonValue>, &'static str> {
    object
        .get(key)
        .and_then(JsonValue::as_object)
        .ok_or("seeded-run required object is missing")
}

pub(crate) fn required_string<'a>(
    object: &'a BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<&'a str, &'static str> {
    object
        .get(key)
        .and_then(JsonValue::as_string)
        .ok_or("seeded-run required string is missing")
}

pub(crate) fn require_null(
    object: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<(), &'static str> {
    if matches!(object.get(key), Some(JsonValue::Null)) {
        Ok(())
    } else {
        Err("seeded-run evidence field must be null")
    }
}

pub(crate) fn require_string(
    object: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<(), &'static str> {
    if object
        .get(key)
        .and_then(JsonValue::as_string)
        .is_some_and(safe_identity)
    {
        Ok(())
    } else {
        Err("seeded-run required error string is missing or unsafe")
    }
}

fn safe_identity(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':' | b'/')
        })
}
