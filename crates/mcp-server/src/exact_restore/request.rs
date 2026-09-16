// SPDX-License-Identifier: MIT

use crate::json::JsonValue;
use crate::protocol_artifact_hash::sha256_hex;
use crate::{
    EXACT_RESTORE_GATEWAY_CONTRACT, EXACT_RESTORE_GATEWAY_SCHEMA_DIGEST,
    EXACT_RESTORE_MAX_FRAME_BYTES,
};

use super::schema;

const OWNER_FIELDS: [&str; 4] = ["instance_id", "session_id", "lease_id", "lease_epoch"];
const REQUEST_KINDS: [&str; 5] = [
    "exact_restore_begin_request",
    "exact_restore_chunk_request",
    "exact_restore_finish_blob_request",
    "exact_restore_commit_request",
    "exact_restore_lookup_request",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExactRestorePhase {
    Begin,
    PutChunk,
    FinishBlob,
    Commit,
    Lookup,
}

impl ExactRestorePhase {
    pub fn route(self) -> &'static str {
        match self {
            Self::Begin => "begin",
            Self::PutChunk => "chunk",
            Self::FinishBlob => "finish",
            Self::Commit => "commit",
            Self::Lookup => "lookup",
        }
    }

    pub fn tool(self) -> &'static str {
        match self {
            Self::Begin => super::EXACT_RESTORE_BEGIN_TOOL,
            Self::PutChunk => super::EXACT_RESTORE_PUT_CHUNK_TOOL,
            Self::FinishBlob => super::EXACT_RESTORE_FINISH_BLOB_TOOL,
            Self::Commit => super::EXACT_RESTORE_COMMIT_TOOL,
            Self::Lookup => super::EXACT_RESTORE_LOOKUP_TOOL,
        }
    }

    pub(super) fn response_kind(self) -> &'static str {
        match self {
            Self::Begin => "exact_restore_begin_response",
            Self::PutChunk => "exact_restore_chunk_response",
            Self::FinishBlob => "exact_restore_finish_blob_response",
            Self::Commit => "exact_restore_commit_response",
            Self::Lookup => "exact_restore_lookup_response",
        }
    }

    fn request_kind(self) -> &'static str {
        match self {
            Self::Begin => REQUEST_KINDS[0],
            Self::PutChunk => REQUEST_KINDS[1],
            Self::FinishBlob => REQUEST_KINDS[2],
            Self::Commit => REQUEST_KINDS[3],
            Self::Lookup => REQUEST_KINDS[4],
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExactRestoreTransportOwner {
    pub instance_id: String,
    pub session_id: String,
    pub lease_id: String,
    pub lease_epoch: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExactRestoreRequestBinding {
    pub phase: ExactRestorePhase,
    pub operation_id: String,
    pub message_id: String,
    pub correlation_id: String,
    pub request_digest: String,
    pub expected_owner: JsonValue,
    pub frame: JsonValue,
}

pub fn exact_restore_request_input_schema() -> JsonValue {
    schema::gateway_request_schema()
}

pub fn validate_exact_restore_request(
    envelope: &JsonValue,
    expected_tool: &str,
    principal_id: &str,
    owner: &ExactRestoreTransportOwner,
) -> Result<ExactRestoreRequestBinding, &'static str> {
    if envelope.to_json().len() > EXACT_RESTORE_MAX_FRAME_BYTES {
        return Err("exact-restore wrapper exceeds 16384 bytes");
    }
    schema::validate_gateway(envelope)?;
    if super::string_member(envelope, "contract") != Some(EXACT_RESTORE_GATEWAY_CONTRACT)
        || super::string_member(envelope, "schema_digest")
            != Some(EXACT_RESTORE_GATEWAY_SCHEMA_DIGEST)
        || super::string_member(envelope, "kind") != Some("exact_restore_request")
    {
        return Err("exact-restore wrapper contract is unsupported");
    }
    let actor = super::object_member(envelope, "actor").ok_or("wrapper actor is missing")?;
    let auth = super::object_member(envelope, "auth").ok_or("wrapper auth is missing")?;
    if super::string_member(actor, "principal_id") != Some(principal_id)
        || super::string_member(actor, "role") != Some("harness")
        || super::string_member(auth, "principal_id") != Some(principal_id)
        || super::string_member(auth, "capability") != Some("exact_restore")
        || super::object_member(auth, "proof") != Some(&JsonValue::Null)
    {
        return Err("exact-restore wrapper identity is not configured");
    }
    let frame = super::object_member(
        super::object_member(envelope, "payload").ok_or("wrapper payload is missing")?,
        "frame",
    )
    .ok_or("neutral frame is missing")?;
    if frame.to_json().len() > EXACT_RESTORE_MAX_FRAME_BYTES {
        return Err("exact-restore neutral frame exceeds 16384 bytes");
    }
    schema::validate_neutral(frame)?;
    let kind = super::string_member(frame, "kind").ok_or("neutral request kind is missing")?;
    let phase = phase_for_kind(kind).ok_or("exact-restore request frame kind is unsupported")?;
    if phase.tool() != expected_tool || kind != phase.request_kind() {
        return Err("exact-restore route does not match the request phase");
    }
    let message_id = require_string(envelope, "message_id")?;
    let correlation_id = require_string(envelope, "correlation_id")?;
    if message_id != require_string(frame, "message_id")?
        || correlation_id != require_string(frame, "correlation_id")?
    {
        return Err("wrapper and neutral request identifiers differ");
    }
    let payload = super::object_member(frame, "payload").ok_or("request payload is missing")?;
    let operation_id = require_string(payload, "operation_id")?.to_owned();
    let expected_owner =
        super::object_member(payload, "expected_owner").ok_or("expected owner is missing")?;
    validate_transport_owner(expected_owner, owner)?;
    validate_phase_payload(phase, payload)?;
    let request_digest = format!("sha256:{}", sha256_hex(frame.to_json().as_bytes()));
    Ok(ExactRestoreRequestBinding {
        phase,
        operation_id,
        message_id: message_id.to_owned(),
        correlation_id: correlation_id.to_owned(),
        request_digest,
        expected_owner: expected_owner.clone(),
        frame: frame.clone(),
    })
}

pub(super) fn phase_for_kind(kind: &str) -> Option<ExactRestorePhase> {
    match kind {
        "exact_restore_begin_request" => Some(ExactRestorePhase::Begin),
        "exact_restore_chunk_request" => Some(ExactRestorePhase::PutChunk),
        "exact_restore_finish_blob_request" => Some(ExactRestorePhase::FinishBlob),
        "exact_restore_commit_request" => Some(ExactRestorePhase::Commit),
        "exact_restore_lookup_request" => Some(ExactRestorePhase::Lookup),
        _ => None,
    }
}

fn validate_transport_owner(
    expected: &JsonValue,
    configured: &ExactRestoreTransportOwner,
) -> Result<(), &'static str> {
    if super::string_member(expected, OWNER_FIELDS[0]) != Some(configured.instance_id.as_str())
        || super::string_member(expected, OWNER_FIELDS[1]) != Some(configured.session_id.as_str())
        || super::string_member(expected, OWNER_FIELDS[2]) != Some(configured.lease_id.as_str())
        || super::object_member(expected, OWNER_FIELDS[3])
            != Some(&JsonValue::Number(configured.lease_epoch))
    {
        return Err("exact-restore owner does not match the configured gateway lease");
    }
    Ok(())
}

fn validate_phase_payload(
    phase: ExactRestorePhase,
    payload: &JsonValue,
) -> Result<(), &'static str> {
    match phase {
        ExactRestorePhase::Begin => validate_begin(payload),
        ExactRestorePhase::PutChunk => super::chunk::validate(payload),
        ExactRestorePhase::FinishBlob | ExactRestorePhase::Commit | ExactRestorePhase::Lookup => {
            Ok(())
        }
    }
}

fn validate_begin(payload: &JsonValue) -> Result<(), &'static str> {
    let artifacts = super::object_member(payload, "artifacts")
        .and_then(JsonValue::as_array)
        .ok_or("begin artifacts are missing")?;
    let references = integer(payload, "artifact_reference_count")?;
    let distinct = integer(payload, "distinct_blob_count")?;
    let manifest_size = integer(payload, "manifest_size_bytes")?;
    let aggregate = integer(payload, "aggregate_closure_bytes")?;
    if references != artifacts.len() as i64 || artifacts.len() > 64 || artifacts.len() < 2 {
        return Err("begin artifact-reference count is inconsistent");
    }
    let mut digests: Vec<(&str, i64)> = Vec::new();
    let mut sum = manifest_size;
    for artifact in artifacts {
        let digest = require_string(artifact, "digest")?;
        let size = integer(artifact, "size_bytes")?;
        if let Some((_, previous_size)) = digests.iter().find(|(known, _)| *known == digest) {
            if *previous_size != size {
                return Err("aliased artifact references disagree on size");
            }
        } else {
            digests.push((digest, size));
            sum = sum
                .checked_add(size)
                .ok_or("begin aggregate size overflows")?;
        }
    }
    if distinct != digests.len() as i64 || aggregate != sum || sum > 64 * 1024 * 1024 {
        return Err("begin closure byte or distinct-blob count is inconsistent");
    }
    Ok(())
}

fn integer(value: &JsonValue, key: &str) -> Result<i64, &'static str> {
    match super::object_member(value, key) {
        Some(JsonValue::Number(value)) => Ok(*value),
        _ => Err("exact-restore integer field is missing"),
    }
}

fn require_string<'a>(value: &'a JsonValue, key: &str) -> Result<&'a str, &'static str> {
    super::string_member(value, key).ok_or("exact-restore string field is missing")
}
