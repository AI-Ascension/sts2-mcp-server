// SPDX-License-Identifier: MIT

use crate::json::JsonValue;
use crate::protocol_artifact_hash::sha256_hex;
use crate::{EXACT_RESTORE_MAX_CHUNK_BASE64_BYTES, EXACT_RESTORE_MAX_CHUNK_RAW_BYTES};

pub(super) fn validate(payload: &JsonValue) -> Result<(), &'static str> {
    let encoded = super::string_member(payload, "data_base64")
        .ok_or("exact-restore chunk data is missing")?;
    if encoded.len() > EXACT_RESTORE_MAX_CHUNK_BASE64_BYTES {
        return Err("encoded exact-restore chunk exceeds its limit");
    }
    let bytes = decode_base64(encoded)?;
    if bytes.is_empty() || bytes.len() > EXACT_RESTORE_MAX_CHUNK_RAW_BYTES {
        return Err("decoded exact-restore chunk exceeds its limit");
    }
    let expected = super::string_member(payload, "chunk_digest")
        .ok_or("exact-restore chunk digest is missing")?;
    if expected != format!("sha256:{}", sha256_hex(&bytes)) {
        return Err("exact-restore chunk digest does not match its bytes");
    }
    Ok(())
}

fn decode_base64(value: &str) -> Result<Vec<u8>, &'static str> {
    if value.is_empty() || !value.len().is_multiple_of(4) {
        return Err("exact-restore chunk base64 is not canonical");
    }
    let bytes = value.as_bytes();
    let padding = bytes.iter().rev().take_while(|byte| **byte == b'=').count();
    if padding > 2 || bytes[..bytes.len() - padding].contains(&b'=') {
        return Err("exact-restore chunk base64 is not canonical");
    }
    let raw_len = value
        .len()
        .checked_div(4)
        .and_then(|blocks| blocks.checked_mul(3))
        .and_then(|length| length.checked_sub(padding))
        .ok_or("exact-restore chunk base64 is invalid")?;
    if raw_len > EXACT_RESTORE_MAX_CHUNK_RAW_BYTES {
        return Err("decoded exact-restore chunk exceeds its limit");
    }
    let mut result = Vec::with_capacity(raw_len);
    for (index, block) in bytes.chunks_exact(4).enumerate() {
        let is_last = index + 1 == bytes.len() / 4;
        let a = base64_value(block[0])?;
        let b = base64_value(block[1])?;
        let c = if block[2] == b'=' {
            if !is_last || block[3] != b'=' || b & 0x0f != 0 {
                return Err("exact-restore chunk base64 has invalid padding");
            }
            0
        } else {
            base64_value(block[2])?
        };
        let d = if block[3] == b'=' {
            if !is_last || (block[2] != b'=' && c & 0x03 != 0) {
                return Err("exact-restore chunk base64 has invalid padding");
            }
            0
        } else {
            base64_value(block[3])?
        };
        let combined =
            (u32::from(a) << 18) | (u32::from(b) << 12) | (u32::from(c) << 6) | u32::from(d);
        result.push((combined >> 16) as u8);
        if block[2] != b'=' {
            result.push((combined >> 8) as u8);
        }
        if block[3] != b'=' {
            result.push(combined as u8);
        }
    }
    if result.len() != raw_len {
        return Err("exact-restore chunk base64 is invalid");
    }
    Ok(result)
}

fn base64_value(byte: u8) -> Result<u8, &'static str> {
    match byte {
        b'A'..=b'Z' => Ok(byte - b'A'),
        b'a'..=b'z' => Ok(byte - b'a' + 26),
        b'0'..=b'9' => Ok(byte - b'0' + 52),
        b'+' => Ok(62),
        b'/' => Ok(63),
        _ => Err("exact-restore chunk base64 is invalid"),
    }
}
