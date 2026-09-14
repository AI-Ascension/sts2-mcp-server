// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::catalog::{SAVE_PROFILE_CONTRACT, SAVE_PROFILE_MAX_BODY_BYTES};
use crate::json::JsonValue;

use super::Context;
use super::validation;

const FIELDS: [&str; 14] = [
    "revision",
    "schema_revision",
    "status",
    "operation_id",
    "instance_id",
    "session_id",
    "mcp_session_id",
    "lease_id",
    "correlation_id",
    "lease_epoch",
    "profile_id",
    "save_profile_id",
    "baseline",
    "error_code",
];
const STATUSES: [&str; 9] = [
    "accepted",
    "settled",
    "rejected",
    "unknown",
    "blocked",
    "cancelled",
    "pending",
    "created",
    "selected",
];
const REVISIONS: [&str; 2] = [SAVE_PROFILE_CONTRACT, "save-profile-downstream-v1"];
const SENSITIVE_KEY_MARKERS: [&str; 11] = [
    "credential",
    "token",
    "secret",
    "password",
    "private",
    "path",
    "filesystem",
    "directory",
    "command",
    "authorization",
    "api_key",
];

#[derive(Clone, Debug)]
struct DownstreamProjection {
    revision: Option<String>,
    status: Option<String>,
    operation_id: Option<String>,
    instance_id: Option<String>,
    session_id: Option<String>,
    mcp_session_id: Option<String>,
    lease_id: Option<String>,
    correlation_id: Option<String>,
    lease_epoch: Option<i64>,
    profile_id: Option<String>,
    baseline: Option<JsonValue>,
    error_code: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum DownstreamError {
    Malformed(&'static str),
    TooLarge,
}

impl From<&'static str> for DownstreamError {
    fn from(message: &'static str) -> Self {
        Self::Malformed(message)
    }
}

/// Projects one `downstream` value exactly as it arrives on the wire.
///
/// The gateway serializes a save-profile result body as a JSON array of byte
/// values, so this decodes that opaque carrier once and then hands the decoded
/// content to [`project_decoded`]. Decoding and projection stay separate: the
/// decoded content is never reinterpreted as a byte array, so a body such as
/// `[123,125]` is rejected instead of being decoded a second time into `{}`.
pub(super) fn project(value: &JsonValue, context: &Context) -> Result<JsonValue, DownstreamError> {
    match value {
        // The gateway uses an empty array for a result that carries no body.
        JsonValue::Array(values) if values.is_empty() => Ok(JsonValue::Array(Vec::new())),
        JsonValue::Array(values) => project_byte_array(values, context),
        JsonValue::Null | JsonValue::Object(_) => project_decoded(value, context),
        _ => {
            bounded(value.to_json().len())?;
            Err("save-profile downstream must be an object or null".into())
        }
    }
}

/// Decodes the gateway's opaque downstream body.
///
/// The gateway serializes a save-profile result body as a JSON array of byte
/// values, so the body limit applies to the decoded bytes rather than to the
/// numeric-array text that carries them.
fn project_byte_array(
    values: &[JsonValue],
    context: &Context,
) -> Result<JsonValue, DownstreamError> {
    let bytes = values
        .iter()
        .map(|value| match value {
            JsonValue::Number(byte) => {
                u8::try_from(*byte).map_err(|_| "save-profile downstream is not a byte array")
            }
            _ => Err("save-profile downstream is not a byte array"),
        })
        .collect::<Result<Vec<u8>, _>>()?;
    bounded(bytes.len())?;
    let text = std::str::from_utf8(&bytes)
        .map_err(|_| "save-profile downstream bytes are not valid UTF-8")?;
    let decoded = crate::json::parse_json(text)
        .map_err(|_| "save-profile downstream bytes are not valid JSON")?;
    project_decoded(&decoded, context)
}

/// Projects one already-decoded downstream body.
///
/// A decoded body is a closed object or `null`. Every other shape, including
/// any array, is rejected: an unrecognized body must keep mutation uncertainty
/// instead of being projected as a settled result.
fn project_decoded(value: &JsonValue, context: &Context) -> Result<JsonValue, DownstreamError> {
    match value {
        JsonValue::Null => Ok(JsonValue::Null),
        JsonValue::Object(object) if object.is_empty() => Ok(JsonValue::object([])),
        JsonValue::Object(object) => {
            bounded(value.to_json().len())?;
            reject_sensitive_material(value)?;
            validate_fields(object)?;
            let projection = DownstreamProjection::parse(object, context)?;
            Ok(projection.to_json())
        }
        _ => {
            bounded(value.to_json().len())?;
            Err("save-profile downstream content must be an object or null".into())
        }
    }
}

fn bounded(length: usize) -> Result<(), DownstreamError> {
    if length > SAVE_PROFILE_MAX_BODY_BYTES {
        Err(DownstreamError::TooLarge)
    } else {
        Ok(())
    }
}

impl DownstreamProjection {
    fn parse(
        object: &BTreeMap<String, JsonValue>,
        context: &Context,
    ) -> Result<Self, &'static str> {
        let revision = optional_revision(object)?;
        let status = optional_status(object)?;
        let operation_id = optional_operation_id(object, context)?;
        let instance_id = optional_echo(object, "instance_id", &context.instance_id)?;
        let session_id = optional_echo(object, "session_id", &context.gateway_session_id)?;
        let mcp_session_id = optional_echo(object, "mcp_session_id", &context.mcp_session_id)?;
        let lease_id = optional_echo(object, "lease_id", &context.lease_id)?;
        let correlation_id = optional_echo(object, "correlation_id", &context.correlation_id)?;
        let lease_epoch = optional_epoch(object, context.lease_epoch)?;
        let profile_id = optional_profile(object)?;
        validation::validate_baseline(object.get("baseline"))?;
        validation::validate_error_code(object.get("error_code"))?;
        let baseline = optional_value(object.get("baseline"));
        let error_code = optional_text(object.get("error_code"));
        Ok(Self {
            revision,
            status,
            operation_id,
            instance_id,
            session_id,
            mcp_session_id,
            lease_id,
            correlation_id,
            lease_epoch,
            profile_id,
            baseline,
            error_code,
        })
    }

    fn to_json(&self) -> JsonValue {
        let mut fields = Vec::new();
        push_text(&mut fields, "revision", self.revision.as_deref());
        push_text(&mut fields, "status", self.status.as_deref());
        push_text(&mut fields, "operation_id", self.operation_id.as_deref());
        push_text(&mut fields, "instance_id", self.instance_id.as_deref());
        push_text(&mut fields, "session_id", self.session_id.as_deref());
        push_text(
            &mut fields,
            "mcp_session_id",
            self.mcp_session_id.as_deref(),
        );
        push_text(&mut fields, "lease_id", self.lease_id.as_deref());
        push_text(
            &mut fields,
            "correlation_id",
            self.correlation_id.as_deref(),
        );
        if let Some(epoch) = self.lease_epoch {
            fields.push((String::from("lease_epoch"), JsonValue::Number(epoch)));
        }
        push_text(&mut fields, "profile_id", self.profile_id.as_deref());
        if let Some(baseline) = &self.baseline {
            fields.push((String::from("baseline"), baseline.clone()));
        }
        push_text(&mut fields, "error_code", self.error_code.as_deref());
        JsonValue::object(fields)
    }
}

fn validate_fields(object: &BTreeMap<String, JsonValue>) -> Result<(), &'static str> {
    if object.keys().any(|key| !FIELDS.contains(&key.as_str())) {
        return Err("save-profile downstream has unknown fields");
    }
    if object.contains_key("revision") && object.contains_key("schema_revision") {
        return Err("save-profile downstream has duplicate revisions");
    }
    if object.contains_key("profile_id") && object.contains_key("save_profile_id") {
        return Err("save-profile downstream has duplicate profile identities");
    }
    Ok(())
}

fn optional_revision(object: &BTreeMap<String, JsonValue>) -> Result<Option<String>, &'static str> {
    let value = object
        .get("revision")
        .or_else(|| object.get("schema_revision"));
    let Some(value) = value else {
        return Ok(None);
    };
    let revision = value
        .as_string()
        .ok_or("save-profile downstream revision is invalid")?;
    if !REVISIONS.contains(&revision) {
        return Err("save-profile downstream revision is unsupported");
    }
    Ok(Some(String::from(revision)))
}

fn optional_status(object: &BTreeMap<String, JsonValue>) -> Result<Option<String>, &'static str> {
    let Some(value) = object.get("status") else {
        return Ok(None);
    };
    let status = value
        .as_string()
        .ok_or("save-profile downstream status is invalid")?;
    if !STATUSES.contains(&status) {
        return Err("save-profile downstream status is unsupported");
    }
    Ok(Some(String::from(status)))
}

fn optional_operation_id(
    object: &BTreeMap<String, JsonValue>,
    context: &Context,
) -> Result<Option<String>, &'static str> {
    let Some(value) = object.get("operation_id") else {
        return Ok(None);
    };
    let operation_id = value
        .as_string()
        .ok_or("save-profile downstream operation identity is invalid")?;
    if operation_id != context.operation_id || !validation::safe_operation_id(operation_id) {
        return Err("save-profile downstream operation identity does not match");
    }
    Ok(Some(String::from(operation_id)))
}

fn optional_echo(
    object: &BTreeMap<String, JsonValue>,
    name: &str,
    expected: &str,
) -> Result<Option<String>, &'static str> {
    let Some(value) = object.get(name) else {
        return Ok(None);
    };
    let actual = value
        .as_string()
        .ok_or("save-profile downstream authority identity is invalid")?;
    if actual != expected || !validation::safe_identity(actual) {
        return Err("save-profile downstream authority identity does not match");
    }
    Ok(Some(String::from(actual)))
}

fn optional_epoch(
    object: &BTreeMap<String, JsonValue>,
    expected: i64,
) -> Result<Option<i64>, &'static str> {
    let Some(value) = object.get("lease_epoch") else {
        return Ok(None);
    };
    let JsonValue::Number(epoch) = value else {
        return Err("save-profile downstream lease epoch is invalid");
    };
    if *epoch != expected || *epoch < 0 {
        return Err("save-profile downstream lease epoch does not match");
    }
    Ok(Some(*epoch))
}

fn optional_profile(object: &BTreeMap<String, JsonValue>) -> Result<Option<String>, &'static str> {
    let value = object
        .get("profile_id")
        .or_else(|| object.get("save_profile_id"));
    validation::validate_optional_profile(value)?;
    Ok(value.and_then(JsonValue::as_string).map(String::from))
}

fn optional_value(value: Option<&JsonValue>) -> Option<JsonValue> {
    value
        .filter(|value| !matches!(value, JsonValue::Null))
        .cloned()
}

fn optional_text(value: Option<&JsonValue>) -> Option<String> {
    value.and_then(JsonValue::as_string).map(String::from)
}

fn push_text(fields: &mut Vec<(String, JsonValue)>, name: &str, value: Option<&str>) {
    if let Some(value) = value {
        fields.push((String::from(name), JsonValue::string(value)));
    }
}

fn reject_sensitive_material(value: &JsonValue) -> Result<(), &'static str> {
    match value {
        JsonValue::String(value) if sensitive_string(value) => {
            Err("save-profile downstream contains private or credential-like content")
        }
        JsonValue::Array(values) => values.iter().try_for_each(reject_sensitive_material),
        JsonValue::Object(object) => {
            for (key, value) in object {
                if sensitive_key(key) {
                    return Err(
                        "save-profile downstream contains a private or credential-like field",
                    );
                }
                reject_sensitive_material(value)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn sensitive_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    SENSITIVE_KEY_MARKERS
        .iter()
        .any(|marker| key.contains(marker))
}

fn sensitive_string(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    value.starts_with('/')
        || value.starts_with('\\')
        || value.contains('\\')
        || lower.starts_with("~/")
        || lower.starts_with("../")
        || lower.contains("://")
        || lower.contains("bearer ")
        || lower.contains("basic ")
        || lower.contains("-----begin ")
        || lower.contains("api_key=")
        || lower.contains("access_token=")
        || lower.contains("password=")
        || lower.contains("secret=")
}
