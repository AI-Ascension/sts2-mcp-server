// SPDX-License-Identifier: MIT

//! Owner-local copy of the whole-manifest artifact.
//!
//! The MCP adapter pins the artifact bytes and digest before exposing the read, so a drifted
//! contract cannot be served under this pin.

use crate::json::{self, JsonValue};

pub const CONTENT_MANIFEST_PROTOCOL_VERSION: &str = "game-information-content-manifest-v1";
pub const CONTENT_MANIFEST_ARTIFACT: &str = "sts2-protocol/game-information-content-manifest-v1";
pub const CONTENT_MANIFEST_SCHEMA_SOURCE: &str =
    "schemas/game-information-content-manifest-v1.schema.json";
pub const CONTENT_MANIFEST_GENERATOR: &str = "hand-authored";
pub const CONTENT_MANIFEST_SCHEMA_DIGEST: &str =
    "416a39769445e6e462c5d5b5504f29010c255e2116a73094e55c7268e47f2ba6";

/// The profile's own serialization ceiling, declared by the pinned manifest.
pub const CONTENT_MANIFEST_MAX_MESSAGE_BYTES: i64 = 16_777_216;
/// The smaller bound the gateway route admits for one complete manifest.
///
/// The gateway frames 128 KiB, so a manifest the producer declares larger than this is answered
/// with the protocol's closed oversize arm rather than a shortened catalog. The adapter keeps the
/// same number so it cannot advertise or accept more than the gateway would relay.
pub const CONTENT_MANIFEST_GATEWAY_MAX_RESPONSE_BYTES: usize = 128 * 1024;

/// The conformance case the pinned manifest names, and the fixture directory it negates.
const CONFORMANCE_CASE: &str = "../../conformance/cases/game-information-content-manifest-v1.json";
const CONFORMANCE_FIXTURES_ROOT: &str =
    "../../conformance/fixtures/game-information-content-manifest-v1";
const CONFORMANCE_INVALID_ROOT: &str =
    "../../conformance/fixtures/game-information-content-manifest-v1/invalid";

const MANIFEST: &[u8] =
    include_bytes!("../../../protocol-artifact/game-information-content-manifest-v1/manifest.json");
const SCHEMA: &[u8] =
    include_bytes!("../../../protocol-artifact/game-information-content-manifest-v1/schema.json");
const SOURCE_SCHEMA: &[u8] =
    include_bytes!("../../../schemas/game-information-content-manifest-v1.schema.json");
const CHECKSUMS: &str =
    include_str!("../../../protocol-artifact/game-information-content-manifest-v1/SHA256SUMS");
const ARTIFACT_FILES: &[(&str, &[u8])] = &[
    (
        "README.md",
        include_bytes!("../../../protocol-artifact/game-information-content-manifest-v1/README.md"),
    ),
    (
        "golden/canonical-manifest-response.json",
        include_bytes!(
            "../../../protocol-artifact/game-information-content-manifest-v1/golden/canonical-manifest-response.json"
        ),
    ),
    (
        "golden/access-denied-error-response.json",
        include_bytes!(
            "../../../protocol-artifact/game-information-content-manifest-v1/golden/access-denied-error-response.json"
        ),
    ),
    (
        "manifest.json",
        include_bytes!(
            "../../../protocol-artifact/game-information-content-manifest-v1/manifest.json"
        ),
    ),
    (
        "schema.json",
        include_bytes!(
            "../../../protocol-artifact/game-information-content-manifest-v1/schema.json"
        ),
    ),
];

/// The conformance case and the vectors it declares, vendored byte-for-byte.
///
/// The artifact's own `SHA256SUMS` covers only the files beside it, so the copies the manifest
/// names are pinned by digest here. The two valid vectors carry their goldens' own checksum
/// values, so a copy that drifted from a checksum-pinned golden cannot verify.
const CONFORMANCE_FILES: &[(&str, &[u8], &str)] = &[
    (
        CONFORMANCE_CASE,
        include_bytes!("../../../conformance/cases/game-information-content-manifest-v1.json"),
        "c9f24a504e892eb509a07a90d33a77d287b14809ae9d2eedca22d402b86bf2ed",
    ),
    (
        "../../conformance/fixtures/game-information-content-manifest-v1/valid/canonical-manifest-response.json",
        include_bytes!(
            "../../../conformance/fixtures/game-information-content-manifest-v1/valid/canonical-manifest-response.json"
        ),
        "7e9865f6347953f04af109d865fb4133f2ef3b781dc255d4445bc3d91a73b0a0",
    ),
    (
        "../../conformance/fixtures/game-information-content-manifest-v1/valid/access-denied-error-response.json",
        include_bytes!(
            "../../../conformance/fixtures/game-information-content-manifest-v1/valid/access-denied-error-response.json"
        ),
        "7e1e95b4e316962bcd39ab87a91679150b573604de7de6bd55fc62cc18212511",
    ),
    (
        "../../conformance/fixtures/game-information-content-manifest-v1/invalid/error-with-null-payload.json",
        include_bytes!(
            "../../../conformance/fixtures/game-information-content-manifest-v1/invalid/error-with-null-payload.json"
        ),
        "70806349122254959c7f3a4af6b9601320f8b19aeae3aa8f7b580ecb85a73371",
    ),
    (
        "../../conformance/fixtures/game-information-content-manifest-v1/invalid/mismatched-error-code-reason.json",
        include_bytes!(
            "../../../conformance/fixtures/game-information-content-manifest-v1/invalid/mismatched-error-code-reason.json"
        ),
        "409889730aaf6ef89c3d58b8c6e4d2e31e6350085bc04b2ddbdfa70d099e7d4f",
    ),
    (
        "../../conformance/fixtures/game-information-content-manifest-v1/invalid/raw-error-reason.json",
        include_bytes!(
            "../../../conformance/fixtures/game-information-content-manifest-v1/invalid/raw-error-reason.json"
        ),
        "ee7a263b7062f987be1a975211efcf4c128a09b42451e12d2c59f099607e0bdf",
    ),
    (
        "../../conformance/fixtures/game-information-content-manifest-v1/invalid/success-with-null-manifest.json",
        include_bytes!(
            "../../../conformance/fixtures/game-information-content-manifest-v1/invalid/success-with-null-manifest.json"
        ),
        "81ea7d8802f975d2414611e5168bdec43b5d09a8945a749c20240871419e2439",
    ),
    (
        "../../conformance/fixtures/game-information-content-manifest-v1/invalid/unknown-member.json",
        include_bytes!(
            "../../../conformance/fixtures/game-information-content-manifest-v1/invalid/unknown-member.json"
        ),
        "5053fa2a5eb4e0b42443cd93fe49ccc6b68bbc8c699b66af714f7e88a10008b7",
    ),
    (
        "../../conformance/fixtures/game-information-content-manifest-v1/invalid/unsupported-schema-digest.json",
        include_bytes!(
            "../../../conformance/fixtures/game-information-content-manifest-v1/invalid/unsupported-schema-digest.json"
        ),
        "9266a35c0aab63c5d973c8d83b3b1a773a8250ed056f1380577c660b8727f5b8",
    ),
];

/// Checks the checked-in artifact identity, its schema digest, and its full checksum inventory.
pub fn verify_content_manifest_artifact() -> Result<(), ContentManifestArtifactError> {
    let manifest = parse(MANIFEST)?;
    let expected_provenance = JsonValue::object([
        (
            "source".to_owned(),
            JsonValue::string(CONTENT_MANIFEST_SCHEMA_SOURCE),
        ),
        (
            "generator".to_owned(),
            JsonValue::string(CONTENT_MANIFEST_GENERATOR),
        ),
        ("license".to_owned(), JsonValue::string("MIT")),
    ]);
    let expected_consumers = JsonValue::Array(
        [
            "sts2-game-mod",
            "sts2-gateway",
            "sts2-mcp-server",
            "sts2-harness",
        ]
        .into_iter()
        .map(JsonValue::string)
        .collect(),
    );
    let expected_goldens = JsonValue::Array(
        [
            "golden/canonical-manifest-response.json",
            "golden/access-denied-error-response.json",
        ]
        .into_iter()
        .map(JsonValue::string)
        .collect(),
    );
    let expected_fixtures = JsonValue::Array(
        CONFORMANCE_FILES
            .iter()
            .map(|(path, _, _)| *path)
            .filter(|path| path.contains("/valid/"))
            .map(JsonValue::string)
            .collect(),
    );
    let expected_negative_fixtures = JsonValue::Array(
        [CONFORMANCE_INVALID_ROOT]
            .into_iter()
            .map(JsonValue::string)
            .collect(),
    );
    if field(&manifest, "artifact") != Some(&JsonValue::string(CONTENT_MANIFEST_ARTIFACT))
        || field(&manifest, "protocol_version")
            != Some(&JsonValue::string(CONTENT_MANIFEST_PROTOCOL_VERSION))
        || field(&manifest, "schema") != Some(&JsonValue::string("schema.json"))
        || field(&manifest, "schema_digest")
            != Some(&JsonValue::string(CONTENT_MANIFEST_SCHEMA_DIGEST))
        || field(&manifest, "provenance") != Some(&expected_provenance)
        || field(&manifest, "consumers") != Some(&expected_consumers)
        || field(&manifest, "goldens") != Some(&expected_goldens)
        || field(&manifest, "fixtures") != Some(&expected_fixtures)
        || field(&manifest, "negative_fixtures") != Some(&expected_negative_fixtures)
        || field(&manifest, "conformance") != Some(&JsonValue::string(CONFORMANCE_CASE))
        || field(&manifest, "max_message_bytes")
            != Some(&JsonValue::Number(CONTENT_MANIFEST_MAX_MESSAGE_BYTES))
        || field(&manifest, "checksums") != Some(&JsonValue::string("SHA256SUMS"))
    {
        return Err(ContentManifestArtifactError::ManifestMismatch);
    }
    if SCHEMA != SOURCE_SCHEMA
        || crate::protocol_artifact_hash::sha256_hex(SCHEMA) != CONTENT_MANIFEST_SCHEMA_DIGEST
        || field(&parse(SCHEMA)?, "$id")
            != Some(&JsonValue::string(
                "sts2-game-information-content-manifest-v1",
            ))
    {
        return Err(ContentManifestArtifactError::SchemaMismatch);
    }
    for line in CHECKSUMS.lines() {
        let Some((expected, path)) = line.split_once("  ") else {
            return Err(ContentManifestArtifactError::ChecksumMismatch);
        };
        let Some((_, bytes)) = ARTIFACT_FILES.iter().find(|(name, _)| *name == path) else {
            return Err(ContentManifestArtifactError::ChecksumMismatch);
        };
        if crate::protocol_artifact_hash::sha256_hex(bytes) != expected {
            return Err(ContentManifestArtifactError::ChecksumMismatch);
        }
    }
    for golden in [
        "golden/canonical-manifest-response.json",
        "golden/access-denied-error-response.json",
    ] {
        let Some((_, bytes)) = ARTIFACT_FILES.iter().find(|(name, _)| *name == golden) else {
            return Err(ContentManifestArtifactError::ChecksumMismatch);
        };
        parse(bytes)?;
    }
    for (path, bytes, digest) in CONFORMANCE_FILES {
        if !path.starts_with(CONFORMANCE_FIXTURES_ROOT) && *path != CONFORMANCE_CASE {
            return Err(ContentManifestArtifactError::ChecksumMismatch);
        }
        if crate::protocol_artifact_hash::sha256_hex(bytes) != *digest {
            return Err(ContentManifestArtifactError::ChecksumMismatch);
        }
        parse(bytes)?;
    }
    Ok(())
}

/// The manifest-relative paths of the vendored conformance case and its vectors.
pub fn content_manifest_conformance_paths() -> impl Iterator<Item = &'static str> {
    CONFORMANCE_FILES.iter().map(|(path, _, _)| *path)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContentManifestArtifactError {
    InvalidJson,
    ChecksumMismatch,
    ManifestMismatch,
    SchemaMismatch,
}

impl std::fmt::Display for ContentManifestArtifactError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("copied content-manifest artifact is invalid")
    }
}

impl std::error::Error for ContentManifestArtifactError {}

fn field<'a>(value: &'a JsonValue, key: &str) -> Option<&'a JsonValue> {
    value.as_object()?.get(key)
}

fn parse(bytes: &[u8]) -> Result<JsonValue, ContentManifestArtifactError> {
    let text = std::str::from_utf8(bytes).map_err(|_| ContentManifestArtifactError::InvalidJson)?;
    json::parse(text).map_err(|_| ContentManifestArtifactError::InvalidJson)
}

#[cfg(test)]
mod tests {
    use super::verify_content_manifest_artifact;

    #[test]
    fn copied_content_manifest_artifact_is_pinned() {
        assert_eq!(verify_content_manifest_artifact(), Ok(()));
    }
}
