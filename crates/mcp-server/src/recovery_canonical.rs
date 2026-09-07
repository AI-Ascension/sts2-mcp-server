// SPDX-License-Identifier: MIT

//! RCJ-1 validation for the frozen Runtime-v3 legal-action payload.

use crate::json::JsonValue;
use crate::protocol_artifact_recovery::{
    RECOVERY_MAX_ACTION_BYTES, RECOVERY_RUNTIME_V3_SCHEMA_DIGEST,
};
use crate::protocol_artifact_runtime_v2::sha256_hex_for_recovery;

pub(crate) fn validate_action(
    value: &JsonValue,
    expected_digest: &str,
) -> Result<(), &'static str> {
    let object = value
        .as_object()
        .ok_or("recovery action must be an object")?;
    exact(
        object,
        &["schema_digest", "canonical_json_b64", "payload_digest"],
    )?;
    let schema = string(object, "schema_digest")?;
    if schema != RECOVERY_RUNTIME_V3_SCHEMA_DIGEST {
        return Err("recovery action uses an unsupported Runtime-v3 schema");
    }
    let encoded = string(object, "canonical_json_b64")?;
    if encoded.len() > RECOVERY_MAX_ACTION_BYTES || encoded.is_empty() {
        return Err("recovery action exceeds the canonical byte bound");
    }
    let bytes = decode_base64(encoded)?;
    if bytes.len() > RECOVERY_MAX_ACTION_BYTES {
        return Err("recovery action exceeds the decoded byte bound");
    }
    if bytes.is_empty()
        || bytes.iter().any(|byte| {
            *byte < 0x20 || *byte >= 0x80 || *byte == b'\\' || byte.is_ascii_whitespace()
        })
    {
        return Err("recovery action is not RCJ-1 canonical UTF-8");
    }
    let text = std::str::from_utf8(&bytes).map_err(|_| "recovery action is not UTF-8")?;
    let parsed = crate::json::parse(text).map_err(|_| "recovery action JSON is invalid")?;
    validate_legal_action(&parsed)?;
    if parsed.to_json().as_bytes() != bytes.as_slice() {
        return Err("recovery action is not RCJ-1 canonical");
    }
    if sha256_hex_for_recovery(&bytes) != string(object, "payload_digest")?
        || string(object, "payload_digest")? != expected_digest
    {
        return Err("recovery action payload digest does not match");
    }
    Ok(())
}

fn validate_legal_action(value: &JsonValue) -> Result<(), &'static str> {
    let root = value
        .as_object()
        .ok_or("recovery action root must be an object")?;
    exact(root, &["action_id", "action"])?;
    identity(string(root, "action_id")?)?;
    let action = root
        .get("action")
        .and_then(JsonValue::as_object)
        .ok_or("recovery action payload must be an object")?;
    let kind = string(action, "kind")?;
    let fields: &[&str] = match kind {
        "start_run" => &["kind", "character_id"],
        "select_map_node" => &["kind", "node_id"],
        "play_card" => &["kind", "card_id", "target_id"],
        "choose_reward" => &["kind", "reward_id"],
        "shop_purchase" => &["kind", "item_id"],
        "shop_remove" | "smith" | "select_card" => &["kind", "card_id"],
        "event_choice" => &["kind", "choice_id"],
        "end_turn" | "skip_reward" | "rest" | "confirm_victory" | "save_quit" | "proceed"
        | "confirm_selection" | "cancel_selection" => &["kind"],
        _ => return Err("recovery action kind is not in Runtime-v3 grammar"),
    };
    exact(action, fields)?;
    for field in fields.iter().copied().filter(|field| *field != "kind") {
        match action.get(field) {
            Some(JsonValue::String(value)) => identity(value)?,
            Some(JsonValue::Null) if field == "target_id" => {}
            _ => return Err("recovery action identity is invalid"),
        }
    }
    Ok(())
}

fn decode_base64(value: &str) -> Result<Vec<u8>, &'static str> {
    if !value.bytes().all(|byte| {
        byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/' | b'=' | b'-' | b'_')
    }) {
        return Err("recovery action base64 contains an unsafe character");
    }
    let mut clean = value.as_bytes().to_vec();
    for byte in &mut clean {
        if *byte == b'-' {
            *byte = b'+';
        } else if *byte == b'_' {
            *byte = b'/';
        }
    }
    let padding = clean
        .iter()
        .position(|byte| *byte == b'=')
        .unwrap_or(clean.len());
    if !clean.len().is_multiple_of(4)
        || clean[padding..].iter().any(|byte| *byte != b'=')
        || (clean.len() - padding) > 2
    {
        return Err("recovery action base64 padding is invalid");
    }
    let mut output = Vec::with_capacity(clean.len() / 4 * 3);
    for chunk in clean.chunks_exact(4) {
        let a = six_bits(chunk[0]).ok_or("recovery action base64 is invalid")?;
        let b = six_bits(chunk[1]).ok_or("recovery action base64 is invalid")?;
        let c = if chunk[2] == b'=' {
            0
        } else {
            six_bits(chunk[2]).ok_or("recovery action base64 is invalid")?
        };
        let d = if chunk[3] == b'=' {
            0
        } else {
            six_bits(chunk[3]).ok_or("recovery action base64 is invalid")?
        };
        if (chunk[2] == b'=' && b & 0x0f != 0) || (chunk[3] == b'=' && c & 0x03 != 0) {
            return Err("recovery action base64 has non-zero unused bits");
        }
        output.push((a << 2) | (b >> 4));
        if chunk[2] != b'=' {
            output.push((b << 4) | (c >> 2));
        }
        if chunk[3] != b'=' {
            output.push((c << 6) | d);
        }
    }
    Ok(output)
}

fn six_bits(byte: u8) -> Option<u8> {
    match byte {
        b'A'..=b'Z' => Some(byte - b'A'),
        b'a'..=b'z' => Some(byte - b'a' + 26),
        b'0'..=b'9' => Some(byte - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

pub(crate) fn exact(
    object: &std::collections::BTreeMap<String, JsonValue>,
    allowed: &[&str],
) -> Result<(), &'static str> {
    if object.keys().all(|key| allowed.contains(&key.as_str()))
        && allowed.iter().all(|key| object.contains_key(*key))
    {
        Ok(())
    } else {
        Err("recovery object has missing or unsupported fields")
    }
}

pub(crate) fn string<'a>(
    object: &'a std::collections::BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<&'a str, &'static str> {
    object
        .get(key)
        .and_then(JsonValue::as_string)
        .ok_or("recovery field must be a string")
}

pub(crate) fn identity(value: &str) -> Result<(), &'static str> {
    if value.is_empty()
        || value.len() > 512
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_.:/".contains(&byte))
    {
        Err("recovery identity is invalid")
    } else {
        Ok(())
    }
}
