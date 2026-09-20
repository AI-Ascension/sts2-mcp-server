// SPDX-License-Identifier: MIT

use std::io::Read;
use std::sync::atomic::{AtomicU64, Ordering};

const UUID_HEX: &[u8; 16] = b"0123456789abcdef";
static UUID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// One fresh lowercase UUIDv4 string for a recovery frame identifier.
pub fn uuid_v4() -> Result<String, String> {
    let mut bytes = random_bytes()?;
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let mut hex = String::with_capacity(36);
    for (index, byte) in bytes.iter().enumerate() {
        if matches!(index, 4 | 6 | 8 | 10) {
            hex.push('-');
        }
        hex.push(char::from(UUID_HEX[usize::from(byte >> 4)]));
        hex.push(char::from(UUID_HEX[usize::from(byte & 0x0f)]));
    }
    Ok(hex)
}

/// `/dev/urandom` is preferred. Where it is unavailable the process mixes the
/// clock, its own identity, and a monotonic counter, so a frame identifier stays
/// unique enough to correlate one request/response pair.
fn random_bytes() -> Result<[u8; 16], String> {
    let mut bytes = [0_u8; 16];
    if let Ok(mut file) = std::fs::File::open("/dev/urandom")
        && file.read_exact(&mut bytes).is_ok()
    {
        return Ok(bytes);
    }
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| String::from("system clock is before the Unix epoch"))?
        .as_nanos() as u64;
    let mut state = nanos
        ^ u64::from(std::process::id()).rotate_left(17)
        ^ UUID_COUNTER.fetch_add(1, Ordering::Relaxed).rotate_left(33);
    for chunk in bytes.chunks_mut(8) {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        for (index, byte) in chunk.iter_mut().enumerate() {
            *byte = ((state >> (8 * index)) & 0xff) as u8;
        }
    }
    Ok(bytes)
}

/// Current time as the strict UTC millisecond timestamp the contract carries.
#[must_use]
pub fn utc_timestamp_now() -> String {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| {
            duration.as_millis().min(u128::from(u64::MAX)) as u64
        });
    timestamp_from_millis(millis)
}

fn timestamp_from_millis(millis: u64) -> String {
    let seconds = millis / 1_000;
    let day_seconds = seconds % 86_400;
    let (year, month, day) = civil_from_days((seconds / 86_400) as i64);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{:03}Z",
        day_seconds / 3_600,
        day_seconds / 60 % 60,
        day_seconds % 60,
        millis % 1_000
    )
}

fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let shifted = days + 719_468;
    let era = if shifted >= 0 {
        shifted
    } else {
        shifted - 146_096
    } / 146_097;
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    (year + i64::from(month <= 2), month, day)
}
