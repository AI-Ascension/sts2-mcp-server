// SPDX-License-Identifier: MIT

use std::collections::{BTreeMap, BTreeSet};

use crate::json::JsonValue;
use crate::protocol_artifact_runtime_v2::sha256_hex;
#[path = "mapping_seeded_run_digest.rs"]
mod digest;
#[path = "mapping_seeded_run_validation_helpers.rs"]
mod helpers;

pub(super) use helpers::{require_null, require_string, required_object, required_string};

use crate::protocol_artifact_seeded_run::{
    SEEDED_RUN_MAX_ACTS, SEEDED_RUN_MAX_CONTEXT_ID_BYTES, SEEDED_RUN_MAX_CONTEXT_TEXT_BYTES,
    SEEDED_RUN_MAX_GENERATION, SEEDED_RUN_MAX_MODIFIERS, SEEDED_RUN_MAX_SEED_BYTES,
};

pub(super) fn validate_selected_context(value: &JsonValue) -> Result<String, &'static str> {
    let object = exact_object(
        value,
        &[
            "context_id",
            "game_mode",
            "character",
            "ascension",
            "modifiers",
            "acts",
            "selection_policy",
            "profile_baseline",
            "save_policy",
            "compatibility",
            "context_digest",
        ],
    )?;
    identity(object, "context_id", SEEDED_RUN_MAX_CONTEXT_ID_BYTES)?;
    constant(object, "game_mode", "standard")?;
    constant(object, "character", "ironclad")?;
    bounded(object.get("ascension"), 0, 20)?;
    unique_identities(
        object.get("modifiers"),
        SEEDED_RUN_MAX_MODIFIERS,
        false,
        true,
    )?;
    unique_identities(object.get("acts"), SEEDED_RUN_MAX_ACTS, true, false)?;
    identity(
        object,
        "selection_policy",
        SEEDED_RUN_MAX_CONTEXT_TEXT_BYTES,
    )?;
    validate_profile_baseline(object.get("profile_baseline"))?;
    enum_string(object, "save_policy", &["disabled", "enabled"])?;
    validate_compatibility(object.get("compatibility"))?;
    let supplied = digest(object.get("context_digest"))?;
    let canonical = digest::canonical_context(object)?;
    let computed = sha256_hex(canonical.as_bytes());
    if supplied != computed {
        return Err("selected_context digest does not match its canonical fields");
    }
    Ok(supplied.to_owned())
}

fn validate_profile_baseline(value: Option<&JsonValue>) -> Result<(), &'static str> {
    let object = exact_object(
        value.ok_or("selected_context profile_baseline is missing")?,
        &["kind", "identity", "digest"],
    )?;
    enum_string(object, "kind", &["fresh", "existing"])?;
    identity(object, "identity", SEEDED_RUN_MAX_CONTEXT_TEXT_BYTES)?;
    digest(object.get("digest"))?;
    Ok(())
}

fn validate_compatibility(value: Option<&JsonValue>) -> Result<(), &'static str> {
    let object = exact_object(
        value.ok_or("selected_context compatibility is missing")?,
        &["game", "mod"],
    )?;
    for key in ["game", "mod"] {
        let item = exact_object(
            object
                .get(key)
                .ok_or("selected_context compatibility member is missing")?,
            &["identity", "digest"],
        )?;
        identity(item, "identity", SEEDED_RUN_MAX_CONTEXT_TEXT_BYTES)?;
        digest(item.get("digest"))?;
    }
    Ok(())
}

fn unique_identities(
    value: Option<&JsonValue>,
    maximum: usize,
    require_one: bool,
    require_sorted: bool,
) -> Result<(), &'static str> {
    let values = value
        .and_then(JsonValue::as_array)
        .ok_or("selected_context array is missing")?;
    if values.is_empty() && require_one || values.len() > maximum {
        return Err("selected_context array is outside its bound");
    }
    let mut seen = BTreeSet::new();
    let mut previous = None;
    for value in values {
        let value = value
            .as_string()
            .ok_or("selected_context array member is not a string")?;
        if !identity_value(value, SEEDED_RUN_MAX_CONTEXT_TEXT_BYTES)
            || !seen.insert(value)
            || (require_sorted && previous.is_some_and(|previous: &str| previous >= value))
        {
            return Err("selected_context array contains an unsafe or duplicate identity");
        }
        previous = Some(value);
    }
    Ok(())
}

pub(super) fn validate_observation(value: Option<&JsonValue>) -> Result<(), &'static str> {
    let value = value.ok_or("seeded-run observation is missing")?;
    if matches!(value, JsonValue::Null) {
        return Ok(());
    }
    let object = exact_object(
        value,
        &[
            "run_started",
            "host_ready",
            "generation",
            "canonical_seed",
            "selected_context_digest",
            "phase_before",
            "phase_after",
            "compatibility_identity",
        ],
    )?;
    if !matches!(object.get("run_started"), Some(JsonValue::Bool(_)))
        || !matches!(object.get("host_ready"), Some(JsonValue::Bool(_)))
    {
        return Err("seeded-run observation readiness fields are invalid");
    }
    bounded(object.get("generation"), 0, SEEDED_RUN_MAX_GENERATION)?;
    seed(object.get("canonical_seed"))?;
    optional_digest(object.get("selected_context_digest"))?;
    identity(object, "phase_before", SEEDED_RUN_MAX_CONTEXT_TEXT_BYTES)?;
    identity(object, "phase_after", SEEDED_RUN_MAX_CONTEXT_TEXT_BYTES)?;
    identity(
        object,
        "compatibility_identity",
        SEEDED_RUN_MAX_CONTEXT_TEXT_BYTES,
    )?;
    Ok(())
}

pub(super) fn validate_witness(value: Option<&JsonValue>) -> Result<(), &'static str> {
    let value = value.ok_or("seeded-run effect_witness is missing")?;
    if matches!(value, JsonValue::Null) {
        return Ok(());
    }
    let object = exact_object(value, &["kind", "generation", "canonical_seed"])?;
    constant(object, "kind", "run_started")?;
    bounded(object.get("generation"), 0, SEEDED_RUN_MAX_GENERATION)?;
    seed(object.get("canonical_seed"))?;
    Ok(())
}

pub(super) fn validate_optional_seed(value: Option<&JsonValue>) -> Result<(), &'static str> {
    match value {
        Some(JsonValue::Null) => Ok(()),
        Some(value) => seed(Some(value)).map(|_| ()),
        None => Err("seeded-run canonical_seed is missing"),
    }
}

pub(super) fn validate_optional_error(value: Option<&JsonValue>) -> Result<(), &'static str> {
    match value {
        Some(JsonValue::Null) => Ok(()),
        Some(JsonValue::String(value))
            if identity_value(value, SEEDED_RUN_MAX_CONTEXT_TEXT_BYTES) =>
        {
            Ok(())
        }
        _ => Err("seeded-run error_code is invalid"),
    }
}

pub(super) fn validate_seed(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= SEEDED_RUN_MAX_SEED_BYTES
        && !value.chars().any(char::is_control)
}

pub(super) fn validate_mode(value: &str) -> bool {
    matches!(value, "seeded_training" | "seeded_replay" | "diagnostic")
}

pub(super) fn validate_identity(value: &str) -> bool {
    identity_value(value, SEEDED_RUN_MAX_CONTEXT_TEXT_BYTES)
}

pub(super) fn validate_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

pub(super) fn bounded_value(value: Option<&JsonValue>) -> Result<i64, &'static str> {
    match value {
        Some(JsonValue::Number(value)) if (0..=SEEDED_RUN_MAX_GENERATION).contains(value) => {
            Ok(*value)
        }
        _ => Err("seeded-run numeric field is outside the protocol bound"),
    }
}

fn exact_object<'a>(
    value: &'a JsonValue,
    keys: &[&str],
) -> Result<&'a BTreeMap<String, JsonValue>, &'static str> {
    let object = value
        .as_object()
        .ok_or("seeded-run value must be an object")?;
    if object.len() != keys.len() || object.keys().any(|key| !keys.contains(&key.as_str())) {
        return Err("seeded-run object has unknown or missing fields");
    }
    Ok(object)
}

fn identity(
    object: &BTreeMap<String, JsonValue>,
    key: &str,
    maximum: usize,
) -> Result<(), &'static str> {
    let value = object
        .get(key)
        .and_then(JsonValue::as_string)
        .ok_or("seeded-run identity is missing")?;
    if identity_value(value, maximum) {
        Ok(())
    } else {
        Err("seeded-run identity is unsafe or oversized")
    }
}

fn identity_value(value: &str, maximum: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':' | b'/')
        })
}

fn constant(
    object: &BTreeMap<String, JsonValue>,
    key: &str,
    expected: &str,
) -> Result<(), &'static str> {
    if object.get(key).and_then(JsonValue::as_string) == Some(expected) {
        Ok(())
    } else {
        Err("seeded-run constant is unsupported")
    }
}

fn enum_string(
    object: &BTreeMap<String, JsonValue>,
    key: &str,
    values: &[&str],
) -> Result<(), &'static str> {
    let value = object
        .get(key)
        .and_then(JsonValue::as_string)
        .ok_or("seeded-run enum is missing")?;
    if values.contains(&value) {
        Ok(())
    } else {
        Err("seeded-run enum is unsupported")
    }
}

fn bounded(value: Option<&JsonValue>, minimum: i64, maximum: i64) -> Result<i64, &'static str> {
    match value {
        Some(JsonValue::Number(value)) if (minimum..=maximum).contains(value) => Ok(*value),
        _ => Err("seeded-run number is outside its protocol bound"),
    }
}

fn seed(value: Option<&JsonValue>) -> Result<&str, &'static str> {
    let value = value
        .and_then(JsonValue::as_string)
        .ok_or("seeded-run seed is missing")?;
    if validate_seed(value) {
        Ok(value)
    } else {
        Err("seeded-run seed is unsafe or oversized")
    }
}

fn digest(value: Option<&JsonValue>) -> Result<&str, &'static str> {
    let value = value
        .and_then(JsonValue::as_string)
        .ok_or("seeded-run digest is missing")?;
    if validate_digest(value) {
        Ok(value)
    } else {
        Err("seeded-run digest is invalid")
    }
}

fn optional_digest(value: Option<&JsonValue>) -> Result<(), &'static str> {
    match value {
        Some(JsonValue::Null) => Ok(()),
        Some(value) => digest(Some(value)).map(|_| ()),
        None => Err("seeded-run optional digest is missing"),
    }
}
