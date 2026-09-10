// SPDX-License-Identifier: MIT

//! Owner-local verification for the copied `seeded-run-v1` protocol package.
//!
//! The protocol repository owns the bytes. This module only refuses to map a
//! request or response when the copied package has drifted from its manifest.

use crate::json::{self, JsonValue};
use crate::protocol_artifact_runtime_v2::sha256_hex;

pub const SEEDED_RUN_PROTOCOL_VERSION: &str = "seeded-run-v1";
pub const SEEDED_RUN_ARTIFACT: &str = "sts2-protocol/seeded-run-v1";
pub const SEEDED_RUN_SCHEMA_SOURCE: &str = "schemas/seeded-run-v1.schema.json";
pub const SEEDED_RUN_GENERATOR: &str = "hand-authored";
pub const SEEDED_RUN_SCHEMA_DIGEST: &str =
    "5c659f344be78f84e8d783986925d462714f933cac95d18943358992f7d3e2b8";
pub const SEEDED_RUN_MAX_SEED_BYTES: usize = 64;
pub const SEEDED_RUN_MAX_IDENTITY_BYTES: usize = 128;
pub const SEEDED_RUN_MAX_CONTEXT_ID_BYTES: usize = 128;
pub const SEEDED_RUN_MAX_CONTEXT_TEXT_BYTES: usize = 128;
pub const SEEDED_RUN_MAX_MODIFIERS: usize = 32;
pub const SEEDED_RUN_MAX_ACTS: usize = 8;
pub const SEEDED_RUN_MAX_GENERATION: i64 = 9_007_199_254_740_991;
pub const SEEDED_RUN_EFFECT_KIND: &str = "run_started";

const MANIFEST: &str = include_str!("../../../protocol-artifact/seeded-run-v1/manifest.json");
const SCHEMA: &str = include_str!("../../../protocol-artifact/seeded-run-v1/schema.json");
const CHECKSUMS: &str = include_str!("../../../protocol-artifact/seeded-run-v1/SHA256SUMS");

struct ArtifactFile {
    path: &'static str,
    bytes: &'static [u8],
}

const ARTIFACT_FILES: &[ArtifactFile] = &[
    ArtifactFile {
        path: "../../conformance/cases/seeded-run-v1.json",
        bytes: include_bytes!("../../../conformance/cases/seeded-run-v1.json"),
    },
    ArtifactFile {
        path: "../../schemas/seeded-run-v1.schema.json",
        bytes: include_bytes!("../../../schemas/seeded-run-v1.schema.json"),
    },
    ArtifactFile {
        path: "manifest.json",
        bytes: include_bytes!("../../../protocol-artifact/seeded-run-v1/manifest.json"),
    },
    ArtifactFile {
        path: "schema.json",
        bytes: include_bytes!("../../../protocol-artifact/seeded-run-v1/schema.json"),
    },
    ArtifactFile {
        path: "golden/reconcile-request.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/seeded-run-v1/golden/reconcile-request.json"
        ),
    },
    ArtifactFile {
        path: "golden/reconcile-settled.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/seeded-run-v1/golden/reconcile-settled.json"
        ),
    },
    ArtifactFile {
        path: "golden/start-accepted.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/seeded-run-v1/golden/start-accepted.json"
        ),
    },
    ArtifactFile {
        path: "golden/start-cancelled.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/seeded-run-v1/golden/start-cancelled.json"
        ),
    },
    ArtifactFile {
        path: "golden/start-rejected.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/seeded-run-v1/golden/start-rejected.json"
        ),
    },
    ArtifactFile {
        path: "golden/start-request.json",
        bytes: include_bytes!("../../../protocol-artifact/seeded-run-v1/golden/start-request.json"),
    },
    ArtifactFile {
        path: "golden/start-settled.json",
        bytes: include_bytes!("../../../protocol-artifact/seeded-run-v1/golden/start-settled.json"),
    },
    ArtifactFile {
        path: "golden/start-unknown.json",
        bytes: include_bytes!("../../../protocol-artifact/seeded-run-v1/golden/start-unknown.json"),
    },
];

pub fn verify_seeded_run_artifact() -> Result<(), SeededRunArtifactError> {
    let manifest = parse(MANIFEST)?;
    let expected_provenance = JsonValue::object([
        (
            String::from("source"),
            JsonValue::string(SEEDED_RUN_SCHEMA_SOURCE),
        ),
        (
            String::from("generator"),
            JsonValue::string(SEEDED_RUN_GENERATOR),
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
    if field(&manifest, "artifact") != Some(&JsonValue::string(SEEDED_RUN_ARTIFACT))
        || field(&manifest, "protocol_version")
            != Some(&JsonValue::string(SEEDED_RUN_PROTOCOL_VERSION))
        || field(&manifest, "schema") != Some(&JsonValue::string("schema.json"))
        || field(&manifest, "schema_digest") != Some(&JsonValue::string(SEEDED_RUN_SCHEMA_DIGEST))
        || field(&manifest, "provenance") != Some(&expected_provenance)
        || field(&manifest, "consumers") != Some(&expected_consumers)
        || field(&manifest, "checksums") != Some(&JsonValue::string("SHA256SUMS"))
    {
        return Err(SeededRunArtifactError::ManifestMismatch);
    }
    if field(&parse(SCHEMA)?, "$id") != Some(&JsonValue::string("sts2-seeded-run-v1")) {
        return Err(SeededRunArtifactError::SchemaMismatch);
    }
    for file in ARTIFACT_FILES {
        let text = std::str::from_utf8(file.bytes)
            .map_err(|_| SeededRunArtifactError::ChecksumMismatch)?;
        parse(text)?;
    }
    verify_checksums()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SeededRunArtifactError {
    ChecksumMismatch,
    InvalidJson,
    ManifestMismatch,
    SchemaMismatch,
}

impl std::fmt::Display for SeededRunArtifactError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("copied seeded-run-v1 artifact is invalid")
    }
}

impl std::error::Error for SeededRunArtifactError {}

fn field<'a>(value: &'a JsonValue, key: &str) -> Option<&'a JsonValue> {
    value.as_object()?.get(key)
}

fn parse(text: &str) -> Result<JsonValue, SeededRunArtifactError> {
    json::parse(text).map_err(|_| SeededRunArtifactError::InvalidJson)
}

fn verify_checksums() -> Result<(), SeededRunArtifactError> {
    let mut verified = Vec::new();
    for line in CHECKSUMS.lines() {
        let (expected, path) = line
            .split_once("  ")
            .ok_or(SeededRunArtifactError::ChecksumMismatch)?;
        if expected.len() != 64
            || !expected
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(SeededRunArtifactError::ChecksumMismatch);
        }
        if verified.contains(&path) {
            return Err(SeededRunArtifactError::ChecksumMismatch);
        }
        let file = ARTIFACT_FILES
            .iter()
            .find(|file| file.path == path)
            .ok_or(SeededRunArtifactError::ChecksumMismatch)?;
        if sha256_hex(file.bytes) != expected {
            return Err(SeededRunArtifactError::ChecksumMismatch);
        }
        verified.push(path);
    }
    if verified.len() != ARTIFACT_FILES.len()
        || ARTIFACT_FILES
            .iter()
            .any(|file| !verified.contains(&file.path))
    {
        return Err(SeededRunArtifactError::ChecksumMismatch);
    }
    Ok(())
}
