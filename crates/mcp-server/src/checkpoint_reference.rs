// SPDX-License-Identifier: MIT

use crate::json::JsonValue;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CheckpointReferenceContext {
    pub(crate) instance_id: String,
    pub(crate) caller_id: String,
    pub(crate) session_id: String,
    pub(crate) lease_id: String,
    pub(crate) lease_epoch: i64,
    pub(crate) correlation_id: String,
}

pub(crate) fn project<'a>(
    value: &'a JsonValue,
    context: &CheckpointReferenceContext,
) -> Result<&'a JsonValue, &'static str> {
    let invalid = "invalid checkpoint reference response";
    if value.to_json().len() > 8192 {
        return Err(invalid);
    }
    let object = value.as_object().ok_or(invalid)?;
    if object.len() != 8 {
        return Err(invalid);
    }
    for (key, expected) in [
        ("schema", "ascension.checkpoint_reference_response.v1"),
        ("instance_id", context.instance_id.as_str()),
        ("caller_id", context.caller_id.as_str()),
        ("session_id", context.session_id.as_str()),
        ("lease_id", context.lease_id.as_str()),
        ("correlation_id", context.correlation_id.as_str()),
    ] {
        if object.get(key).and_then(JsonValue::as_string) != Some(expected) {
            return Err(invalid);
        }
    }
    if object.get("lease_epoch") != Some(&JsonValue::Number(context.lease_epoch)) {
        return Err(invalid);
    }
    let reference = object.get("reference").ok_or(invalid)?;
    validate_reference(reference)?;
    Ok(reference)
}

fn validate_reference(value: &JsonValue) -> Result<(), &'static str> {
    let invalid = "invalid public checkpoint reference";
    let object = value.as_object().ok_or(invalid)?;
    const KEYS: [&str; 8] = [
        "schema",
        "reference_version",
        "handle",
        "occurrence",
        "boundary_kind",
        "boundary_phase",
        "assurance",
        "restore_verified",
    ];
    if object.len() != KEYS.len() || !KEYS.iter().all(|key| object.contains_key(*key)) {
        return Err(invalid);
    }
    for (key, expected) in [
        ("schema", "ascension.exact_checkpoint_reference.v1"),
        ("reference_version", "exact-checkpoint-reference-v1"),
    ] {
        if object.get(key).and_then(JsonValue::as_string) != Some(expected) {
            return Err(invalid);
        }
    }
    let string = |key: &str| {
        object
            .get(key)
            .and_then(JsonValue::as_string)
            .ok_or(invalid)
    };
    let handle = string("handle")?.strip_prefix("ckpt-h1:").ok_or(invalid)?;
    if handle.len() != 64
        || !handle
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(invalid);
    }
    let occurrence = string("occurrence")?;
    if occurrence.is_empty()
        || occurrence.len() > 256
        || !occurrence
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b':' | b'-'))
    {
        return Err(invalid);
    }
    for key in ["boundary_kind", "boundary_phase"] {
        let label = string(key)?;
        if label.is_empty() || label.len() > 256 || label.contains('\0') {
            return Err(invalid);
        }
    }
    let assurance = string("assurance")?;
    if ![
        "public_observation_only",
        "capture_only",
        "restore_supported",
        "restore_verified",
        "continuation_certified",
    ]
    .contains(&assurance)
    {
        return Err(invalid);
    }
    if object.get("restore_verified")
        != Some(&JsonValue::Bool(matches!(
            assurance,
            "restore_verified" | "continuation_certified"
        )))
    {
        return Err(invalid);
    }
    Ok(())
}
