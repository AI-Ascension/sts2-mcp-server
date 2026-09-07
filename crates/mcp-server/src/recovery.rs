// SPDX-License-Identifier: MIT

//! Boundary helpers for the watchdog-recovery-v1 MCP profile.

use crate::json::JsonValue;
use crate::protocol::RequestId;
use crate::protocol_artifact_recovery::{RECOVERY_CONTRACT, RECOVERY_SCHEMA_DIGEST};

pub(crate) use super::recovery_projection::{is_error, project_response, unknown};
pub(crate) use super::recovery_validation::{capability, validate_request};

pub fn recovery_kind_for_path(path: &str) -> Option<&'static str> {
    match path {
        "/v1/recovery/bootstrap" => Some("bootstrap"),
        "/v1/recovery/host-fence" => Some("host_fence"),
        "/v1/recovery/lease/acquire" => Some("lease_acquire"),
        "/v1/recovery/lease/renew" => Some("lease_renew"),
        "/v1/recovery/lease/revoke" => Some("lease_revoke"),
        "/v1/recovery/operation/intent" => Some("operation_intent"),
        "/v1/recovery/operation/dispatch" => Some("operation_dispatch"),
        "/v1/recovery/operation/lookup" => Some("operation_lookup"),
        "/v1/recovery/operation/reconcile" => Some("operation_reconcile"),
        _ => None,
    }
}

pub fn recovery_capability(kind: &str) -> Option<&'static str> {
    capability(kind)
}

pub fn validate_recovery_request(
    frame: &JsonValue,
    kind: &str,
    correlation: &str,
    configured_instance: Option<&str>,
) -> Result<(), &'static str> {
    validate_request(frame, kind, correlation, configured_instance)
}

pub fn validate_recovery_response(
    frame: &JsonValue,
    kind: &str,
    correlation: &str,
) -> Result<(), &'static str> {
    super::recovery_validation::validate_response(frame, kind, correlation)
}

pub(crate) struct FrameIdentity<'a> {
    pub(crate) principal_id: &'a str,
    pub(crate) role: &'a str,
    pub(crate) proof: Option<&'a str>,
}

pub(crate) fn build_request(
    kind: &str,
    payload: JsonValue,
    identity: FrameIdentity<'_>,
) -> Result<(JsonValue, String), &'static str> {
    let capability = capability(kind).ok_or("recovery capability is unsupported")?;
    let correlation = uuid4()?;
    let message_id = uuid4()?;
    let proof = identity.proof.map_or(JsonValue::Null, JsonValue::string);
    let frame = JsonValue::object([
        ("contract".to_owned(), JsonValue::string(RECOVERY_CONTRACT)),
        (
            "schema_digest".to_owned(),
            JsonValue::string(RECOVERY_SCHEMA_DIGEST),
        ),
        ("message_id".to_owned(), JsonValue::string(message_id)),
        (
            "correlation_id".to_owned(),
            JsonValue::string(correlation.as_str()),
        ),
        ("sent_at".to_owned(), JsonValue::string(utc_timestamp()?)),
        (
            "actor".to_owned(),
            JsonValue::object([
                (
                    "principal_id".to_owned(),
                    JsonValue::string(identity.principal_id),
                ),
                ("role".to_owned(), JsonValue::string(identity.role)),
            ]),
        ),
        (
            "auth".to_owned(),
            JsonValue::object([
                (
                    "principal_id".to_owned(),
                    JsonValue::string(identity.principal_id),
                ),
                ("capability".to_owned(), JsonValue::string(capability)),
                ("proof".to_owned(), proof),
            ]),
        ),
        (
            "kind".to_owned(),
            JsonValue::string(format!("{kind}_request")),
        ),
        ("payload".to_owned(), payload),
    ]);
    Ok((frame, correlation))
}

pub(crate) fn correlation_request_id(correlation: String) -> RequestId {
    RequestId::String(correlation)
}

fn uuid4() -> Result<String, &'static str> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|_| "recovery secure identity generation failed")?;
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let mut output = String::with_capacity(36);
    for (index, byte) in bytes.into_iter().enumerate() {
        if matches!(index, 4 | 6 | 8 | 10) {
            output.push('-');
        }
        output.push(char::from(b"0123456789abcdef"[(byte >> 4) as usize]));
        output.push(char::from(b"0123456789abcdef"[(byte & 0x0f) as usize]));
    }
    Ok(output)
}

fn utc_timestamp() -> Result<String, &'static str> {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| "recovery clock is before the Unix epoch")?
        .as_secs();
    let days = seconds / 86_400;
    let day_seconds = seconds % 86_400;
    let (year, month, day) = civil_date(days)?;
    Ok(format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        day_seconds / 3600,
        (day_seconds / 60) % 60,
        day_seconds % 60
    ))
}

fn civil_date(days: u64) -> Result<(i64, u8, u8), &'static str> {
    let z = i64::try_from(days).map_err(|_| "recovery timestamp is out of range")? + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    let year = year + i64::from(month <= 2);
    let month = u8::try_from(month).map_err(|_| "recovery timestamp month is invalid")?;
    let day = u8::try_from(day).map_err(|_| "recovery timestamp day is invalid")?;
    Ok((year, month, day))
}
