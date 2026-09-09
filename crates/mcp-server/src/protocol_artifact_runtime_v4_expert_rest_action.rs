// SPDX-License-Identifier: MIT

//! Owner-local copy and integrity gate for the candidate REST-action artifact.
//!
//! The profile is deliberately kept separate from the admitted potion profile.  The
//! MCP executable verifies the copied bytes before selecting this profile so a stale
//! or silently substituted schema cannot become an executable contract.

use crate::json::{self, JsonValue};

pub const RUNTIME_V4_EXPERT_REST_ACTION_PROTOCOL_VERSION: &str = "runtime-v4-expert-rest-action-v1";
pub const RUNTIME_V4_EXPERT_REST_ACTION_ARTIFACT: &str =
    "sts2-protocol/runtime-v4-expert-rest-action";
pub const RUNTIME_V4_EXPERT_REST_ACTION_SCHEMA_SOURCE: &str =
    "schemas/runtime-v4-expert-rest-action-v1.schema.json";
pub const RUNTIME_V4_EXPERT_REST_ACTION_GENERATOR: &str = "hand-authored";
pub const RUNTIME_V4_EXPERT_REST_ACTION_PROFILE: &str = "expert-rest-action";
pub const RUNTIME_V4_EXPERT_REST_ACTION_SCHEMA_DIGEST: &str =
    "bb3555fae28eb1f79d08a15e9884696a579e4c20836f5016509f17e0f4c36fbd";
pub const RUNTIME_V4_EXPERT_REST_ACTION_EFFECT_WITNESS_VERSION: &str = "rest-effect-witness-v1";
pub const RUNTIME_V4_EXPERT_REST_ACTION_MAX_GENERATION: i64 = 9_007_199_254_740_991;
pub const RUNTIME_V4_EXPERT_REST_ACTION_MAX_SELECTOR_CHOICES: usize = 256;
pub const RUNTIME_V4_EXPERT_REST_ACTION_MAX_IDENTITY_BYTES: usize = 512;

const MANIFEST: &[u8] =
    include_bytes!("../../../protocol-artifact/runtime-v4-expert-rest-action/manifest.json");
const SCHEMA: &[u8] =
    include_bytes!("../../../protocol-artifact/runtime-v4-expert-rest-action/schema.json");
const SOURCE_SCHEMA: &[u8] =
    include_bytes!("../../../schemas/runtime-v4-expert-rest-action-v1.schema.json");
const CHECKSUMS: &str =
    include_str!("../../../protocol-artifact/runtime-v4-expert-rest-action/SHA256SUMS");
const CONFORMANCE: &[u8] =
    include_bytes!("../../../conformance/cases/runtime-v4-expert-rest-action-v1.json");

#[path = "protocol_artifact_runtime_v4_expert_rest_action_files.rs"]
mod files;

use files::ARTIFACT_FILES;

pub fn verify_runtime_v4_expert_rest_action_artifact()
-> Result<(), RuntimeV4ExpertRestActionArtifactError> {
    let manifest = parse(MANIFEST)?;
    let expected_provenance = JsonValue::object([
        (
            String::from("source"),
            JsonValue::string(RUNTIME_V4_EXPERT_REST_ACTION_SCHEMA_SOURCE),
        ),
        (
            String::from("generator"),
            JsonValue::string(RUNTIME_V4_EXPERT_REST_ACTION_GENERATOR),
        ),
        (String::from("license"), JsonValue::string("MIT")),
    ]);
    let empty = JsonValue::Array(Vec::new());
    let prospective = JsonValue::Array(
        [
            "sts2-game-mod",
            "sts2-gateway",
            "sts2-harness",
            "sts2-mcp-server",
        ]
        .into_iter()
        .map(JsonValue::string)
        .collect(),
    );
    if field(&manifest, "artifact")
        != Some(&JsonValue::string(RUNTIME_V4_EXPERT_REST_ACTION_ARTIFACT))
        || field(&manifest, "protocol_version")
            != Some(&JsonValue::string(
                RUNTIME_V4_EXPERT_REST_ACTION_PROTOCOL_VERSION,
            ))
        || field(&manifest, "profile")
            != Some(&JsonValue::string(RUNTIME_V4_EXPERT_REST_ACTION_PROFILE))
        || field(&manifest, "schema") != Some(&JsonValue::string("schema.json"))
        || field(&manifest, "schema_digest")
            != Some(&JsonValue::string(
                RUNTIME_V4_EXPERT_REST_ACTION_SCHEMA_DIGEST,
            ))
        || field(&manifest, "status") != Some(&JsonValue::string("candidate"))
        || field(&manifest, "provenance") != Some(&expected_provenance)
        || field(&manifest, "consumers") != Some(&empty)
        || field(&manifest, "prospective_consumers") != Some(&prospective)
        || field(&manifest, "checksums") != Some(&JsonValue::string("SHA256SUMS"))
    {
        return Err(RuntimeV4ExpertRestActionArtifactError::ManifestMismatch);
    }
    if SCHEMA != SOURCE_SCHEMA
        || field(&parse(SCHEMA)?, "$id")
            != Some(&JsonValue::string("sts2-runtime-v4-expert-rest-action-v1"))
    {
        return Err(RuntimeV4ExpertRestActionArtifactError::SchemaMismatch);
    }
    for file in ARTIFACT_FILES {
        parse(file.bytes)?;
    }
    verify_checksums()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeV4ExpertRestActionArtifactError {
    ChecksumMismatch,
    InvalidJson,
    ManifestMismatch,
    SchemaMismatch,
}

impl std::fmt::Display for RuntimeV4ExpertRestActionArtifactError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("copied Runtime-v4 expert REST-action artifact is invalid")
    }
}

impl std::error::Error for RuntimeV4ExpertRestActionArtifactError {}

fn field<'a>(value: &'a JsonValue, key: &str) -> Option<&'a JsonValue> {
    value.as_object()?.get(key)
}

fn parse(bytes: &[u8]) -> Result<JsonValue, RuntimeV4ExpertRestActionArtifactError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| RuntimeV4ExpertRestActionArtifactError::InvalidJson)?;
    json::parse(text).map_err(|_| RuntimeV4ExpertRestActionArtifactError::InvalidJson)
}

fn verify_checksums() -> Result<(), RuntimeV4ExpertRestActionArtifactError> {
    let mut verified = Vec::new();
    for line in CHECKSUMS.lines().filter(|line| !line.trim().is_empty()) {
        let (expected, path) = line
            .split_once("  ")
            .ok_or(RuntimeV4ExpertRestActionArtifactError::ChecksumMismatch)?;
        if expected.len() != 64
            || !expected
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            || verified.contains(&path)
        {
            return Err(RuntimeV4ExpertRestActionArtifactError::ChecksumMismatch);
        }
        let file = ARTIFACT_FILES
            .iter()
            .find(|file| file.path == path)
            .ok_or(RuntimeV4ExpertRestActionArtifactError::ChecksumMismatch)?;
        if crate::protocol_artifact_hash::sha256_hex(file.bytes) != expected {
            return Err(RuntimeV4ExpertRestActionArtifactError::ChecksumMismatch);
        }
        verified.push(path);
    }
    if verified.len() != ARTIFACT_FILES.len()
        || ARTIFACT_FILES
            .iter()
            .any(|file| !verified.contains(&file.path))
    {
        return Err(RuntimeV4ExpertRestActionArtifactError::ChecksumMismatch);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::verify_runtime_v4_expert_rest_action_artifact;

    #[test]
    fn copied_candidate_artifact_and_checksums_are_frozen() {
        assert_eq!(verify_runtime_v4_expert_rest_action_artifact(), Ok(()));
    }
}
