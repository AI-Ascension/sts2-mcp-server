// SPDX-License-Identifier: MIT

//! Shared primitive checks for watchdog-recovery-v1 payloads.

use std::collections::BTreeMap;

use crate::json::JsonValue;
use crate::protocol_artifact_recovery::RECOVERY_MAX_WIRE_INTEGER;

pub(super) fn obj(value: &JsonValue) -> Result<&BTreeMap<String, JsonValue>, &'static str> {
    value.as_object().ok_or("recovery value must be an object")
}

pub(super) fn nullable<F>(value: Option<&JsonValue>, check: F) -> Result<(), &'static str>
where
    F: FnOnce(&JsonValue) -> Result<(), &'static str>,
{
    match value {
        Some(JsonValue::Null) => Ok(()),
        Some(value) => check(value),
        None => Err("recovery nullable field is missing"),
    }
}

pub(super) fn number(object: &BTreeMap<String, JsonValue>, key: &str) -> Result<i64, &'static str> {
    number_value(object.get(key).ok_or("recovery number is missing")?)
}

pub(super) fn number_value(value: &JsonValue) -> Result<i64, &'static str> {
    match value {
        JsonValue::Number(value) if *value >= 0 && *value <= RECOVERY_MAX_WIRE_INTEGER => {
            Ok(*value)
        }
        _ => Err("recovery integer is outside the wire bound"),
    }
}

pub(super) fn positive(value: i64) -> Result<(), &'static str> {
    if value > 0 && value <= RECOVERY_MAX_WIRE_INTEGER {
        Ok(())
    } else {
        Err("recovery integer must be positive and wire-safe")
    }
}

pub(super) fn wire(value: i64) -> Result<(), &'static str> {
    if (0..=RECOVERY_MAX_WIRE_INTEGER).contains(&value) {
        Ok(())
    } else {
        Err("recovery integer is outside the wire bound")
    }
}

pub(super) fn ttl(ttl: i64, renewal: i64) -> Result<(), &'static str> {
    if (5..=300).contains(&ttl) && (1..=100).contains(&renewal) {
        Ok(())
    } else {
        Err("recovery lease policy is outside bounds")
    }
}

pub(super) fn digest(value: &str) -> Result<(), &'static str> {
    if value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Ok(())
    } else {
        Err("recovery digest is invalid")
    }
}

pub(super) fn token(value: &str) -> Result<(), &'static str> {
    if value.len() == 43
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    {
        Ok(())
    } else {
        Err("recovery fence token is invalid")
    }
}

pub(super) fn enum_value(value: &str, allowed: &[&str]) -> Result<(), &'static str> {
    if allowed.contains(&value) {
        Ok(())
    } else {
        Err("recovery enum value is unsupported")
    }
}

pub(super) fn uuid(value: &str) -> Result<(), &'static str> {
    let bytes = value.as_bytes();
    if bytes.len() == 36
        && bytes.iter().enumerate().all(|(index, byte)| {
            (matches!(index, 8 | 13 | 18 | 23) && *byte == b'-')
                || (!matches!(index, 8 | 13 | 18 | 23)
                    && (byte.is_ascii_digit() || (b'a'..=b'f').contains(byte)))
        })
        && matches!(bytes[19], b'8' | b'9' | b'a' | b'b')
    {
        Ok(())
    } else {
        Err("recovery UUID is invalid")
    }
}

pub(super) fn uuid4(value: &str) -> Result<(), &'static str> {
    uuid(value)?;
    if value.as_bytes().get(14).is_some_and(|byte| *byte == b'4') {
        Ok(())
    } else {
        Err("recovery identity must be UUIDv4")
    }
}

pub(super) fn timestamp(value: &str) -> Result<(), &'static str> {
    let bytes = value.as_bytes();
    let Some(without_zone) = bytes.strip_suffix(b"Z") else {
        return Err("recovery timestamp is invalid");
    };
    if without_zone.len() < 19 {
        return Err("recovery timestamp is invalid");
    }
    let (date_time, fraction) = without_zone.split_at(19);
    let valid_shape = date_time[4] == b'-'
        && date_time[7] == b'-'
        && date_time[10] == b'T'
        && date_time[13] == b':'
        && date_time[16] == b':'
        && date_time[..4]
            .iter()
            .chain(&date_time[5..7])
            .chain(&date_time[8..10])
            .chain(&date_time[11..13])
            .chain(&date_time[14..16])
            .chain(&date_time[17..19])
            .all(u8::is_ascii_digit);
    let valid_fraction = fraction.is_empty()
        || (2..=10).contains(&fraction.len())
            && fraction[0] == b'.'
            && fraction[1..].iter().all(u8::is_ascii_digit);
    if !valid_shape || !valid_fraction {
        return Err("recovery timestamp is invalid");
    }
    let year = decimal(&date_time[0..4]);
    let month = decimal(&date_time[5..7]);
    let day = decimal(&date_time[8..10]);
    let hour = decimal(&date_time[11..13]);
    let minute = decimal(&date_time[14..16]);
    let second = decimal(&date_time[17..19]);
    if year == 0
        || !(1..=12).contains(&month)
        || day == 0
        || day > days_in_month(year, month)
        || hour > 23
        || minute > 59
        || second > 59
    {
        Err("recovery timestamp is invalid")
    } else {
        Ok(())
    }
}

fn decimal(value: &[u8]) -> u32 {
    value
        .iter()
        .fold(0, |total, byte| total * 10 + u32::from(byte - b'0'))
}

fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        2 if year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400)) => {
            29
        }
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}
