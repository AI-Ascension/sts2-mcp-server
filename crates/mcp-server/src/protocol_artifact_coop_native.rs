// SPDX-License-Identifier: MIT

//! Owner-local copy of the source-derived native co-op protocol candidate.
//!
//! The candidate is deliberately copied as bytes rather than represented by a
//! Rust protocol implementation.  This adapter can therefore prove that it is
//! mapping the reviewed artifact, while the admission gate remains explicit
//! until the producer is rebuilt and named consumers provide evidence.

use crate::json::{self, JsonValue};

pub const COOP_NATIVE_PROTOCOL_VERSION: &str = "coop-native-v1";
pub const COOP_NATIVE_PROFILE: &str = "coop-native-v1-mcp";
pub const COOP_NATIVE_ARTIFACT: &str = "sts2-protocol/coop-native-v1";
pub const COOP_NATIVE_SCHEMA_SOURCE: &str = "schemas/coop-native-v1.schema.json";
pub const COOP_NATIVE_GENERATOR: &str = "hand-authored";
pub const COOP_NATIVE_SCHEMA_DIGEST: &str =
    "3e555563023804383534d92118c3863aa2aee3d0d24b932f484d8fd97e452ca8";
pub const COOP_NATIVE_PRODUCER_SCHEMA_DIGEST: &str =
    "afe9bf3674f3e69b0f2454ec3fb1d6265a8e83ebccd996b6b0437208531d72b5";
pub const COOP_NATIVE_MAX_GENERATION: i64 = 9_007_199_254_740_991;
pub const COOP_NATIVE_MAX_BODY_BYTES: usize = 16 * 1024;

const MANIFEST: &[u8] = include_bytes!("../../../protocol-artifact/coop-native-v1/manifest.json");
const SCHEMA: &[u8] = include_bytes!("../../../protocol-artifact/coop-native-v1/schema.json");
const SOURCE_SCHEMA: &[u8] = include_bytes!("../../../schemas/coop-native-v1.schema.json");
const CHECKSUMS: &str = include_str!("../../../protocol-artifact/coop-native-v1/SHA256SUMS");

struct ArtifactFile {
    path: &'static str,
    bytes: &'static [u8],
}

const ARTIFACT_FILES: &[ArtifactFile] = &[
    ArtifactFile {
        path: "../../conformance/cases/coop-native-v1.json",
        bytes: include_bytes!("../../../conformance/cases/coop-native-v1.json"),
    },
    ArtifactFile {
        path: "../../schemas/coop-native-v1.schema.json",
        bytes: SOURCE_SCHEMA,
    },
    ArtifactFile {
        path: "README.md",
        bytes: include_bytes!("../../../protocol-artifact/coop-native-v1/README.md"),
    },
    ArtifactFile {
        path: "conformance.json",
        bytes: include_bytes!("../../../protocol-artifact/coop-native-v1/conformance.json"),
    },
    ArtifactFile {
        path: "golden/local-action-recovered-request.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/coop-native-v1/golden/local-action-recovered-request.json"
        ),
    },
    ArtifactFile {
        path: "golden/local-action-recovered-response.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/coop-native-v1/golden/local-action-recovered-response.json"
        ),
    },
    ArtifactFile {
        path: "golden/local-action-rejected-request.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/coop-native-v1/golden/local-action-rejected-request.json"
        ),
    },
    ArtifactFile {
        path: "golden/local-action-rejected-response.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/coop-native-v1/golden/local-action-rejected-response.json"
        ),
    },
    ArtifactFile {
        path: "golden/local-action-settled-request.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/coop-native-v1/golden/local-action-settled-request.json"
        ),
    },
    ArtifactFile {
        path: "golden/local-action-settled-response.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/coop-native-v1/golden/local-action-settled-response.json"
        ),
    },
    ArtifactFile {
        path: "golden/local-action-unknown-request.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/coop-native-v1/golden/local-action-unknown-request.json"
        ),
    },
    ArtifactFile {
        path: "golden/local-action-unknown-response.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/coop-native-v1/golden/local-action-unknown-response.json"
        ),
    },
    ArtifactFile {
        path: "golden/observation-response.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/coop-native-v1/golden/observation-response.json"
        ),
    },
    ArtifactFile {
        path: "golden/rejoin-pending-request.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/coop-native-v1/golden/rejoin-pending-request.json"
        ),
    },
    ArtifactFile {
        path: "golden/rejoin-pending-response.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/coop-native-v1/golden/rejoin-pending-response.json"
        ),
    },
    ArtifactFile {
        path: "golden/rejoin-recovered-request.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/coop-native-v1/golden/rejoin-recovered-request.json"
        ),
    },
    ArtifactFile {
        path: "golden/rejoin-recovered-response.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/coop-native-v1/golden/rejoin-recovered-response.json"
        ),
    },
    ArtifactFile {
        path: "golden/shared-vote-settled-request.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/coop-native-v1/golden/shared-vote-settled-request.json"
        ),
    },
    ArtifactFile {
        path: "golden/shared-vote-settled-response.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/coop-native-v1/golden/shared-vote-settled-response.json"
        ),
    },
    ArtifactFile {
        path: "manifest.json",
        bytes: MANIFEST,
    },
    ArtifactFile {
        path: "producer-capture.json",
        bytes: include_bytes!("../../../protocol-artifact/coop-native-v1/producer-capture.json"),
    },
    ArtifactFile {
        path: "schema.json",
        bytes: SCHEMA,
    },
];

/// Validate the copied candidate, including every path listed by its checksum
/// inventory.  This succeeds only for the exact unadmitted candidate; it does
/// not make the candidate an admitted runtime profile.
pub fn verify_coop_native_artifact() -> Result<(), CoopNativeArtifactError> {
    let manifest = parse(MANIFEST)?;
    let expected_provenance = JsonValue::object([
        (
            String::from("source"),
            JsonValue::string(COOP_NATIVE_SCHEMA_SOURCE),
        ),
        (
            String::from("generator"),
            JsonValue::string(COOP_NATIVE_GENERATOR),
        ),
        (String::from("license"), JsonValue::string("MIT")),
    ]);
    let empty_consumers = JsonValue::Array(Vec::new());
    if field(&manifest, "artifact") != Some(&JsonValue::string(COOP_NATIVE_ARTIFACT))
        || field(&manifest, "protocol_version")
            != Some(&JsonValue::string(COOP_NATIVE_PROTOCOL_VERSION))
        || field(&manifest, "status") != Some(&JsonValue::string("candidate"))
        || field(&manifest, "admission") != Some(&JsonValue::string("unadmitted"))
        || field(&manifest, "schema") != Some(&JsonValue::string("schema.json"))
        || field(&manifest, "schema_digest") != Some(&JsonValue::string(COOP_NATIVE_SCHEMA_DIGEST))
        || field(&manifest, "provenance") != Some(&expected_provenance)
        || field(&manifest, "consumers") != Some(&empty_consumers)
        || field(&manifest, "producer_declared_schema_digest")
            != Some(&JsonValue::string(COOP_NATIVE_PRODUCER_SCHEMA_DIGEST))
        || field(&manifest, "producer_digest_matches_candidate") != Some(&JsonValue::Bool(false))
        || field(&manifest, "checksums") != Some(&JsonValue::string("SHA256SUMS"))
    {
        return Err(CoopNativeArtifactError::ManifestMismatch);
    }
    if SCHEMA != SOURCE_SCHEMA {
        return Err(CoopNativeArtifactError::SchemaMismatch);
    }
    if sha256_hex(SCHEMA) != COOP_NATIVE_SCHEMA_DIGEST
        || field(&parse(SCHEMA)?, "$id")
            != Some(&JsonValue::string("sts2-coop-native-v1-candidate"))
    {
        return Err(CoopNativeArtifactError::SchemaMismatch);
    }
    for file in ARTIFACT_FILES {
        if file.path.ends_with(".json") {
            parse(file.bytes)?;
        } else if std::str::from_utf8(file.bytes).is_err() {
            return Err(CoopNativeArtifactError::InvalidUtf8);
        }
    }
    verify_checksums()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CoopNativeArtifactError {
    ChecksumMismatch,
    InvalidJson,
    InvalidUtf8,
    ManifestMismatch,
    SchemaMismatch,
}

impl std::fmt::Display for CoopNativeArtifactError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("copied native co-op candidate artifact is invalid")
    }
}

impl std::error::Error for CoopNativeArtifactError {}

fn field<'a>(value: &'a JsonValue, key: &str) -> Option<&'a JsonValue> {
    value.as_object()?.get(key)
}

fn parse(bytes: &[u8]) -> Result<JsonValue, CoopNativeArtifactError> {
    let text = std::str::from_utf8(bytes).map_err(|_| CoopNativeArtifactError::InvalidUtf8)?;
    json::parse(text).map_err(|_| CoopNativeArtifactError::InvalidJson)
}

fn verify_checksums() -> Result<(), CoopNativeArtifactError> {
    let mut verified = Vec::new();
    for line in CHECKSUMS.lines() {
        let (expected, path) = line
            .split_once("  ")
            .ok_or(CoopNativeArtifactError::ChecksumMismatch)?;
        if expected.len() != 64
            || !expected
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            || verified.contains(&path)
        {
            return Err(CoopNativeArtifactError::ChecksumMismatch);
        }
        let file = ARTIFACT_FILES
            .iter()
            .find(|file| file.path == path)
            .ok_or(CoopNativeArtifactError::ChecksumMismatch)?;
        if sha256_hex(file.bytes) != expected {
            return Err(CoopNativeArtifactError::ChecksumMismatch);
        }
        verified.push(path);
    }
    if verified.len() != ARTIFACT_FILES.len()
        || ARTIFACT_FILES
            .iter()
            .any(|file| !verified.contains(&file.path))
    {
        return Err(CoopNativeArtifactError::ChecksumMismatch);
    }
    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    crate::protocol_artifact_hash::sha256_hex(bytes)
}

#[cfg(test)]
mod tests {
    use super::{
        COOP_NATIVE_PRODUCER_SCHEMA_DIGEST, COOP_NATIVE_SCHEMA_DIGEST, verify_coop_native_artifact,
    };

    #[test]
    fn copied_candidate_and_checksum_inventory_are_frozen() {
        assert_eq!(verify_coop_native_artifact(), Ok(()));
        assert_ne!(
            COOP_NATIVE_SCHEMA_DIGEST,
            COOP_NATIVE_PRODUCER_SCHEMA_DIGEST
        );
    }
}
