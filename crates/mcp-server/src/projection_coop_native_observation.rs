// SPDX-License-Identifier: MIT

use std::collections::{BTreeMap, BTreeSet};

use crate::json::JsonValue;
use crate::protocol_artifact_coop_native::COOP_NATIVE_MAX_GENERATION;

use super::exact_object;

const OBSERVATION_FIELDS: [&str; 14] = [
    "host_authority_epoch",
    "authority_id",
    "run_id",
    "host_sequence_kind",
    "host_generation",
    "state_digest",
    "checkpoint_id",
    "checksum_algorithm",
    "checksum_status",
    "native_checksum",
    "host_digest_known",
    "host_loading",
    "host_divergent",
    "peers",
];
const PEER_FIELDS: [&str; 13] = [
    "peer_token",
    "authority_id",
    "role",
    "connected",
    "peer_generation",
    "state_digest",
    "rejoin_epoch",
    "authority_epoch",
    "checkpoint_id",
    "digest_known",
    "is_loading",
    "is_divergent",
    "checksum_status",
];
pub(super) fn validate_observation(value: &JsonValue) -> Result<(), &'static str> {
    let object = exact_object(value, &OBSERVATION_FIELDS, "native observation")?;
    for field in [
        "host_authority_epoch",
        "authority_id",
        "run_id",
        "checkpoint_id",
    ] {
        if !object
            .get(field)
            .and_then(JsonValue::as_string)
            .is_some_and(safe_identity)
        {
            return Err("native observation identity is invalid");
        }
    }
    if object
        .get("host_sequence_kind")
        .and_then(JsonValue::as_string)
        != Some("adapter_sequence")
        || object
            .get("checksum_algorithm")
            .and_then(JsonValue::as_string)
            != Some("sha256")
    {
        return Err("native observation algorithm or sequence kind is unsupported");
    }
    generation(object.get("host_generation"))?;
    digest(object.get("state_digest"))?;
    checksum_status(object.get("checksum_status"))?;
    nullable_digest(object.get("native_checksum"))?;
    booleans(
        object,
        &["host_digest_known", "host_loading", "host_divergent"],
    )?;
    let peers = object
        .get("peers")
        .and_then(JsonValue::as_array)
        .ok_or("native observation peers are missing")?;
    if !(2..=4).contains(&peers.len()) {
        return Err("native observation peer count is outside its bound");
    }
    let mut peer_ids = BTreeSet::new();
    let mut local_count = 0;
    for peer in peers {
        let peer_object = exact_object(peer, &PEER_FIELDS, "native peer")?;
        let peer_id = peer_object
            .get("peer_token")
            .and_then(JsonValue::as_string)
            .filter(|value| peer_identity(value))
            .ok_or("native peer token is invalid")?;
        if !peer_ids.insert(peer_id) {
            return Err("native observation peers are not unique");
        }
        if !peer_object
            .get("authority_id")
            .and_then(JsonValue::as_string)
            .is_some_and(safe_identity)
            || !peer_object
                .get("authority_epoch")
                .and_then(JsonValue::as_string)
                .is_some_and(safe_identity)
        {
            return Err("native peer authority identity is invalid");
        }
        match peer_object.get("role").and_then(JsonValue::as_string) {
            Some("local") => local_count += 1,
            Some("ally") => {}
            _ => return Err("native peer role is unsupported"),
        }
        if peer_object.get("role").and_then(JsonValue::as_string) == Some("local") {
            // The local peer is the only peer allowed to own the host action
            // authority in this adapter projection.
            if peer_object.get("connected") != Some(&JsonValue::Bool(true)) {
                return Err("native local peer must be connected");
            }
        }
        booleans(
            peer_object,
            &["connected", "digest_known", "is_loading", "is_divergent"],
        )?;
        generation(peer_object.get("peer_generation"))?;
        generation(peer_object.get("rejoin_epoch"))?;
        digest(peer_object.get("state_digest"))?;
        nullable_identity(peer_object.get("checkpoint_id"))?;
        checksum_status(peer_object.get("checksum_status"))?;
    }
    if local_count != 1 {
        return Err("native observation must contain exactly one local peer");
    }
    Ok(())
}
pub(super) fn generation(value: Option<&JsonValue>) -> Result<i64, &'static str> {
    match value {
        Some(JsonValue::Number(value)) if (0..=COOP_NATIVE_MAX_GENERATION).contains(value) => {
            Ok(*value)
        }
        _ => Err("native generation is outside the protocol bound"),
    }
}

pub(super) fn digest(value: Option<&JsonValue>) -> Result<(), &'static str> {
    match value {
        Some(JsonValue::String(value)) if lower_hex_digest(value) => Ok(()),
        _ => Err("native digest is invalid"),
    }
}

pub(super) fn nullable_digest(value: Option<&JsonValue>) -> Result<(), &'static str> {
    match value {
        Some(JsonValue::Null) => Ok(()),
        Some(value) => digest(Some(value)),
        None => Err("native nullable digest is missing"),
    }
}

fn nullable_identity(value: Option<&JsonValue>) -> Result<(), &'static str> {
    match value {
        Some(JsonValue::Null) => Ok(()),
        Some(JsonValue::String(value)) if safe_identity(value) => Ok(()),
        _ => Err("native nullable identity is invalid"),
    }
}

fn checksum_status(value: Option<&JsonValue>) -> Result<(), &'static str> {
    if matches!(
        value.and_then(JsonValue::as_string),
        Some(
            "available"
                | "unavailable"
                | "unknown"
                | "disabled"
                | "enabled_unread"
                | "enabled"
                | "divergent"
                | "matched"
        )
    ) {
        Ok(())
    } else {
        Err("native checksum status is unsupported")
    }
}

fn booleans(object: &BTreeMap<String, JsonValue>, fields: &[&str]) -> Result<(), &'static str> {
    if fields
        .iter()
        .all(|field| matches!(object.get(*field), Some(JsonValue::Bool(_))))
    {
        Ok(())
    } else {
        Err("native boolean field is missing or malformed")
    }
}

pub(super) fn safe_identity(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 512
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':' | b'/')
        })
}

fn peer_identity(value: &str) -> bool {
    value
        .strip_prefix("peer:")
        .is_some_and(|suffix| (5..=507).contains(&suffix.len()) && safe_identity(value))
}

fn lower_hex_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
