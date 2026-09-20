// SPDX-License-Identifier: MIT

//! Closed wire scalars shared by the recovery envelope and payload checks.

/// A 64-character lowercase hex digest, as the recovery contract fixes it.
#[must_use]
pub fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// Any lowercase RFC 4122 hyphenated UUID, matching the gateway request rule.
#[must_use]
pub fn valid_uuid(value: &str) -> bool {
    value.len() == 36
        && value.as_bytes().iter().enumerate().all(|(index, byte)| {
            if matches!(index, 8 | 13 | 18 | 23) {
                *byte == b'-'
            } else {
                byte.is_ascii_digit() || (b'a'..=b'f').contains(byte)
            }
        })
        && matches!(value.as_bytes().get(19), Some(b'8' | b'9' | b'a' | b'b'))
}

#[must_use]
pub fn valid_uuid_v4(value: &str) -> bool {
    valid_uuid(value) && value.as_bytes().get(14) == Some(&b'4')
}

/// Strict UTC timestamp with real calendar validation, as the harness reads it.
#[must_use]
pub fn valid_timestamp(value: &str) -> bool {
    let bytes = value.as_bytes();
    if !(20..=30).contains(&bytes.len())
        || bytes.get(4) != Some(&b'-')
        || bytes.get(7) != Some(&b'-')
        || bytes.get(10) != Some(&b'T')
        || bytes.get(13) != Some(&b':')
        || bytes.get(16) != Some(&b':')
        || bytes.last() != Some(&b'Z')
        || !bytes[..19]
            .iter()
            .enumerate()
            .all(|(index, byte)| matches!(index, 4 | 7 | 10 | 13 | 16) || byte.is_ascii_digit())
    {
        return false;
    }
    if bytes.len() > 20
        && (bytes.get(19) != Some(&b'.')
            || !bytes[20..bytes.len() - 1].iter().all(u8::is_ascii_digit)
            || !(1..=9).contains(&(bytes.len() - 21)))
    {
        return false;
    }
    timestamp_parts(bytes)
}

fn timestamp_parts(bytes: &[u8]) -> bool {
    let number = |range: std::ops::Range<usize>| {
        bytes
            .get(range)
            .filter(|slice| slice.iter().all(u8::is_ascii_digit))
            .map(|slice| {
                slice
                    .iter()
                    .fold(0_u32, |value, byte| value * 10 + u32::from(byte - b'0'))
            })
    };
    let (Some(year), Some(month), Some(day)) = (number(0..4), number(5..7), number(8..10)) else {
        return false;
    };
    let (Some(hour), Some(minute), Some(second)) = (number(11..13), number(14..16), number(17..19))
    else {
        return false;
    };
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        _ => return false,
    };
    (1..=12).contains(&month)
        && (1..=days).contains(&day)
        && hour < 24
        && minute < 60
        && second < 60
}
