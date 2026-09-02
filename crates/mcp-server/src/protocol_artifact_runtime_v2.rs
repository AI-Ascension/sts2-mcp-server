// SPDX-License-Identifier: MIT

//! Owner-local release metadata for the handed-off `runtime-v2` artifact.
//!
//! The protocol target owns the final schema bytes and digest. Keep this
//! module deliberately small so the protocol captain can replace this one
//! module, plus the copied artifact, without changing the mapping seam.

use crate::json::JsonValue;

#[path = "protocol_artifact_runtime_v2_hash.rs"]
mod hash;

/// Version consumed by the Runtime-v2 MCP mapping.
pub const RUNTIME_V2_PROTOCOL_VERSION: &str = "runtime-v2";
/// SHA-256 of the canonical Runtime-v2 schema source bytes.
pub const RUNTIME_V2_SCHEMA_DIGEST: &str =
    "f7963b19c8ed5bbdc02c08e83c7a2e16c4771ed5eb798b29a8208d7a917a86c2";
/// Release-like artifact identity owned by `sts2-protocol`.
pub const RUNTIME_V2_ARTIFACT: &str = "sts2-protocol/runtime-v2";
/// Repository-relative source recorded in the final artifact provenance.
pub const RUNTIME_V2_SCHEMA_SOURCE: &str = "schemas/runtime-v2.schema.json";
/// Generator recorded in the final artifact provenance.
pub const RUNTIME_V2_GENERATOR: &str = "hand-authored";
/// The only mutation admitted by the Runtime-v2 MCP profile.
pub const RUNTIME_V2_ACTION_ID: &str = "end_turn";
/// The witness required for an authoritative end-turn settlement.
pub const RUNTIME_V2_EFFECT_KIND: &str = "turn_end_settled";
/// The only observation phase in which `end_turn` is legal.
pub const RUNTIME_V2_PLAYER_TURN_PHASE: &str = "combat/player_turn";
/// Maximum generation and lease epoch represented exactly by the contract.
pub const RUNTIME_V2_MAX_GENERATION: i64 = 9_007_199_254_740_991;
/// Maximum turn index represented by the bounded contract.
pub const RUNTIME_V2_MAX_TURN_INDEX: i64 = 1024;

const MANIFEST: &str = include_str!("../../../protocol-artifact/runtime-v2/manifest.json");
const SCHEMA: &str = include_str!("../../../protocol-artifact/runtime-v2/schema.json");
const CHECKSUMS: &str = include_str!("../../../protocol-artifact/runtime-v2/SHA256SUMS");

struct ArtifactFile {
    path: &'static str,
    bytes: &'static [u8],
}

const ARTIFACT_FILES: &[ArtifactFile] = &[
    ArtifactFile {
        path: "../../conformance/cases/runtime-v2.json",
        bytes: include_bytes!("../../../conformance/cases/runtime-v2.json"),
    },
    ArtifactFile {
        path: "../../schemas/runtime-v2.schema.json",
        bytes: include_bytes!("../../../schemas/runtime-v2.schema.json"),
    },
    ArtifactFile {
        path: "manifest.json",
        bytes: include_bytes!("../../../protocol-artifact/runtime-v2/manifest.json"),
    },
    ArtifactFile {
        path: "schema.json",
        bytes: include_bytes!("../../../protocol-artifact/runtime-v2/schema.json"),
    },
    ArtifactFile {
        path: "golden/cancelled-before-dispatch.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/runtime-v2/golden/cancelled-before-dispatch.json"
        ),
    },
    ArtifactFile {
        path: "golden/duplicate-replay.json",
        bytes: include_bytes!("../../../protocol-artifact/runtime-v2/golden/duplicate-replay.json"),
    },
    ArtifactFile {
        path: "golden/enemy-turn-request.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/runtime-v2/golden/enemy-turn-request.json"
        ),
    },
    ArtifactFile {
        path: "golden/enemy-turn-response.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/runtime-v2/golden/enemy-turn-response.json"
        ),
    },
    ArtifactFile {
        path: "golden/idempotency-conflict-request.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/runtime-v2/golden/idempotency-conflict-request.json"
        ),
    },
    ArtifactFile {
        path: "golden/idempotency-conflict-response.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/runtime-v2/golden/idempotency-conflict-response.json"
        ),
    },
    ArtifactFile {
        path: "golden/legal-action-accepted.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/runtime-v2/golden/legal-action-accepted.json"
        ),
    },
    ArtifactFile {
        path: "golden/legal-action-request.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/runtime-v2/golden/legal-action-request.json"
        ),
    },
    ArtifactFile {
        path: "golden/legal-action-settled.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/runtime-v2/golden/legal-action-settled.json"
        ),
    },
    ArtifactFile {
        path: "golden/outside-combat-request.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/runtime-v2/golden/outside-combat-request.json"
        ),
    },
    ArtifactFile {
        path: "golden/outside-combat-response.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/runtime-v2/golden/outside-combat-response.json"
        ),
    },
    ArtifactFile {
        path: "golden/reconcile-request.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/runtime-v2/golden/reconcile-request.json"
        ),
    },
    ArtifactFile {
        path: "golden/reconcile-settled-response.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/runtime-v2/golden/reconcile-settled-response.json"
        ),
    },
    ArtifactFile {
        path: "golden/stale-generation-request.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/runtime-v2/golden/stale-generation-request.json"
        ),
    },
    ArtifactFile {
        path: "golden/stale-generation-response.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/runtime-v2/golden/stale-generation-response.json"
        ),
    },
    ArtifactFile {
        path: "golden/state-request.json",
        bytes: include_bytes!("../../../protocol-artifact/runtime-v2/golden/state-request.json"),
    },
    ArtifactFile {
        path: "golden/state-response.json",
        bytes: include_bytes!("../../../protocol-artifact/runtime-v2/golden/state-response.json"),
    },
    ArtifactFile {
        path: "golden/timeout-action-request.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/runtime-v2/golden/timeout-action-request.json"
        ),
    },
    ArtifactFile {
        path: "golden/timeout-unknown-response.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/runtime-v2/golden/timeout-unknown-response.json"
        ),
    },
];

/// Verifies the copied release-like Runtime-v2 package metadata and vectors.
pub fn verify_runtime_v2_artifact() -> Result<(), RuntimeV2ArtifactError> {
    let manifest = parse(MANIFEST)?;
    let expected_provenance = JsonValue::object([
        (
            String::from("source"),
            JsonValue::string(RUNTIME_V2_SCHEMA_SOURCE),
        ),
        (
            String::from("generator"),
            JsonValue::string(RUNTIME_V2_GENERATOR),
        ),
        (String::from("license"), JsonValue::string("MIT")),
    ]);
    let expected_consumers = JsonValue::Array(
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
    if field(&manifest, "artifact") != Some(&JsonValue::string(RUNTIME_V2_ARTIFACT))
        || field(&manifest, "protocol_version")
            != Some(&JsonValue::string(RUNTIME_V2_PROTOCOL_VERSION))
        || field(&manifest, "schema") != Some(&JsonValue::string("schema.json"))
        || field(&manifest, "schema_digest") != Some(&JsonValue::string(RUNTIME_V2_SCHEMA_DIGEST))
        || field(&manifest, "provenance") != Some(&expected_provenance)
        || field(&manifest, "consumers") != Some(&expected_consumers)
        || field(&manifest, "checksums") != Some(&JsonValue::string("SHA256SUMS"))
    {
        return Err(RuntimeV2ArtifactError::ManifestMismatch);
    }
    if field(&parse(SCHEMA)?, "$id") != Some(&JsonValue::string("sts2-runtime-v2")) {
        return Err(RuntimeV2ArtifactError::SchemaMismatch);
    }
    for file in ARTIFACT_FILES {
        let text = std::str::from_utf8(file.bytes)
            .map_err(|_| RuntimeV2ArtifactError::ChecksumMismatch)?;
        parse(text)?;
    }
    verify_checksums()?;
    Ok(())
}

/// A deterministic failure while loading the copied Runtime-v2 artifact.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeV2ArtifactError {
    ChecksumMismatch,
    InvalidJson,
    ManifestMismatch,
    SchemaMismatch,
}

impl std::fmt::Display for RuntimeV2ArtifactError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("copied Runtime-v2 artifact is invalid")
    }
}

impl std::error::Error for RuntimeV2ArtifactError {}

fn field<'a>(value: &'a JsonValue, key: &str) -> Option<&'a JsonValue> {
    value.as_object()?.get(key)
}

fn parse(text: &str) -> Result<JsonValue, RuntimeV2ArtifactError> {
    crate::json::parse(text).map_err(|_| RuntimeV2ArtifactError::InvalidJson)
}

fn verify_checksums() -> Result<(), RuntimeV2ArtifactError> {
    let mut verified = Vec::new();
    for line in CHECKSUMS.lines() {
        let (expected, path) = line
            .split_once("  ")
            .ok_or(RuntimeV2ArtifactError::ChecksumMismatch)?;
        if expected.len() != 64
            || !expected
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(RuntimeV2ArtifactError::ChecksumMismatch);
        }
        if verified.contains(&path) {
            return Err(RuntimeV2ArtifactError::ChecksumMismatch);
        }
        let file = ARTIFACT_FILES
            .iter()
            .find(|file| file.path == path)
            .ok_or(RuntimeV2ArtifactError::ChecksumMismatch)?;
        if hash::sha256_hex(file.bytes) != expected {
            return Err(RuntimeV2ArtifactError::ChecksumMismatch);
        }
        verified.push(path);
    }
    if verified.len() != ARTIFACT_FILES.len()
        || ARTIFACT_FILES
            .iter()
            .any(|file| !verified.contains(&file.path))
    {
        return Err(RuntimeV2ArtifactError::ChecksumMismatch);
    }
    Ok(())
}
