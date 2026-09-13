// SPDX-License-Identifier: MIT

use crate::json::JsonValue;

pub(crate) fn validate_guidance(value: Option<&JsonValue>) -> Result<(), &'static str> {
    let Some(value) = value else {
        return Ok(());
    };
    if matches!(value, JsonValue::Null) {
        return Ok(());
    }
    let object = value
        .as_object()
        .ok_or("save-profile guidance is not an object")?;
    if object.len() != 2
        || object
            .keys()
            .any(|key| !["code", "action"].contains(&key.as_str()))
    {
        return Err("save-profile guidance has unsupported fields");
    }
    let code = object
        .get("code")
        .and_then(JsonValue::as_string)
        .ok_or("save-profile guidance is invalid")?;
    if !safe_identity(code) {
        return Err("save-profile guidance code is invalid");
    }
    let action = object
        .get("action")
        .and_then(JsonValue::as_string)
        .ok_or("save-profile guidance is invalid")?;
    if !safe_text(action) {
        return Err("save-profile guidance action is invalid");
    }
    Ok(())
}

pub(crate) fn validate_error_code(value: Option<&JsonValue>) -> Result<(), &'static str> {
    let Some(value) = value else {
        return Ok(());
    };
    if matches!(value, JsonValue::Null) {
        return Ok(());
    }
    let value = value
        .as_string()
        .ok_or("save-profile error_code is invalid")?;
    validate_error_string(value)
}

pub(crate) fn validate_error_string(value: &str) -> Result<(), &'static str> {
    if value.is_empty()
        || value.len() > 128
        || !value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':' | b'/')
        })
    {
        return Err("save-profile error_code is unsafe or oversized");
    }
    Ok(())
}

pub(crate) fn safe_operation_id(value: &str) -> bool {
    safe_identity(value) && !value.contains('/')
}

pub(crate) fn safe_identity(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && !value.contains("..")
        && !value.contains("://")
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}

fn safe_text(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value.chars().all(|character| !character.is_control())
}
