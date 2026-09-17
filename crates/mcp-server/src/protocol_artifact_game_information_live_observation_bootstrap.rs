// SPDX-License-Identifier: MIT

//! Owner-local copy of the candidate live-observation bootstrap artifact.
//! The MCP adapter pins the artifact bytes and digest before exposing the tool.

use crate::json::{self, JsonValue};

pub const LIVE_BOOTSTRAP_PROTOCOL_VERSION: &str = "game-information-live-observation-bootstrap-v1";
pub const LIVE_BOOTSTRAP_ARTIFACT: &str =
    "sts2-protocol/game-information-live-observation-bootstrap-v1";
pub const LIVE_BOOTSTRAP_SCHEMA_SOURCE: &str =
    "schemas/game-information-live-observation-bootstrap-v1.schema.json";
pub const LIVE_BOOTSTRAP_GENERATOR: &str = "hand-authored";
pub const LIVE_BOOTSTRAP_SCHEMA_DIGEST: &str =
    "6041a282ffda8757af4e3eb6ab551e082f136fe53138ab8ac17db9fab52765c2";
pub const LIVE_BOOTSTRAP_MAX_VISIBLE_ENTITIES: i64 = 64;
pub const LIVE_BOOTSTRAP_MAX_ITEM_BYTES: usize = 65_536;
pub const LIVE_BOOTSTRAP_MAX_MESSAGE_BYTES: usize = 262_144;
pub const LIVE_BOOTSTRAP_MAX_BODY_BYTES: usize = 262_144;

const MANIFEST: &[u8] = include_bytes!(
    "../../../protocol-artifact/game-information-live-observation-bootstrap-v1/manifest.json"
);
const SCHEMA: &[u8] = include_bytes!(
    "../../../protocol-artifact/game-information-live-observation-bootstrap-v1/schema.json"
);
const CHECKSUMS: &str = include_str!(
    "../../../protocol-artifact/game-information-live-observation-bootstrap-v1/SHA256SUMS"
);
const ARTIFACT_FILES: &[(&str, &[u8])] = &[
    (
        "README.md",
        include_bytes!(
            "../../../protocol-artifact/game-information-live-observation-bootstrap-v1/README.md"
        ),
    ),
    (
        "golden/bootstrap-request.json",
        include_bytes!(
            "../../../protocol-artifact/game-information-live-observation-bootstrap-v1/golden/bootstrap-request.json"
        ),
    ),
    (
        "golden/bootstrap-response.json",
        include_bytes!(
            "../../../protocol-artifact/game-information-live-observation-bootstrap-v1/golden/bootstrap-response.json"
        ),
    ),
    (
        "golden/error-native-unavailable.json",
        include_bytes!(
            "../../../protocol-artifact/game-information-live-observation-bootstrap-v1/golden/error-native-unavailable.json"
        ),
    ),
    (
        "golden/live-query-v1-request.json",
        include_bytes!(
            "../../../protocol-artifact/game-information-live-observation-bootstrap-v1/golden/live-query-v1-request.json"
        ),
    ),
    (
        "manifest.json",
        include_bytes!(
            "../../../protocol-artifact/game-information-live-observation-bootstrap-v1/manifest.json"
        ),
    ),
    (
        "schema.json",
        include_bytes!(
            "../../../protocol-artifact/game-information-live-observation-bootstrap-v1/schema.json"
        ),
    ),
];
pub fn verify_live_bootstrap_artifact() -> Result<(), LiveBootstrapArtifactError> {
    let manifest = parse(MANIFEST)?;
    let expected_provenance = JsonValue::object([
        (
            "source".to_owned(),
            JsonValue::string(LIVE_BOOTSTRAP_SCHEMA_SOURCE),
        ),
        (
            "generator".to_owned(),
            JsonValue::string(LIVE_BOOTSTRAP_GENERATOR),
        ),
        ("license".to_owned(), JsonValue::string("MIT")),
    ]);
    let expected_consumers = JsonValue::Array(Vec::new());
    let expected_prospective = JsonValue::Array(
        ["sts2-gateway", "sts2-harness", "sts2-mcp-server"]
            .into_iter()
            .map(JsonValue::string)
            .collect(),
    );
    if field(&manifest, "artifact") != Some(&JsonValue::string(LIVE_BOOTSTRAP_ARTIFACT))
        || field(&manifest, "protocol_version")
            != Some(&JsonValue::string(LIVE_BOOTSTRAP_PROTOCOL_VERSION))
        || field(&manifest, "schema") != Some(&JsonValue::string("schema.json"))
        || field(&manifest, "schema_digest")
            != Some(&JsonValue::string(LIVE_BOOTSTRAP_SCHEMA_DIGEST))
        || field(&manifest, "status") != Some(&JsonValue::string("candidate"))
        || field(&manifest, "provenance") != Some(&expected_provenance)
        || field(&manifest, "consumers") != Some(&expected_consumers)
        || field(&manifest, "prospective_consumers") != Some(&expected_prospective)
        || field(&manifest, "checksums") != Some(&JsonValue::string("SHA256SUMS"))
    {
        return Err(LiveBootstrapArtifactError::ManifestMismatch);
    }
    if field(&parse(SCHEMA)?, "$id")
        != Some(&JsonValue::string(
            "sts2-game-information-live-observation-bootstrap-v1",
        ))
    {
        return Err(LiveBootstrapArtifactError::SchemaMismatch);
    }
    if crate::protocol_artifact_hash::sha256_hex(SCHEMA) != LIVE_BOOTSTRAP_SCHEMA_DIGEST {
        return Err(LiveBootstrapArtifactError::ChecksumMismatch);
    }
    for line in CHECKSUMS.lines() {
        let Some((expected, path)) = line.split_once("  ") else {
            return Err(LiveBootstrapArtifactError::ChecksumMismatch);
        };
        let Some((_, bytes)) = ARTIFACT_FILES.iter().find(|(name, _)| *name == path) else {
            return Err(LiveBootstrapArtifactError::ChecksumMismatch);
        };
        if crate::protocol_artifact_hash::sha256_hex(bytes) != expected {
            return Err(LiveBootstrapArtifactError::ChecksumMismatch);
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LiveBootstrapArtifactError {
    InvalidJson,
    ChecksumMismatch,
    ManifestMismatch,
    SchemaMismatch,
}

impl std::fmt::Display for LiveBootstrapArtifactError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("copied live-observation bootstrap artifact is invalid")
    }
}

impl std::error::Error for LiveBootstrapArtifactError {}

fn field<'a>(value: &'a JsonValue, key: &str) -> Option<&'a JsonValue> {
    value.as_object()?.get(key)
}

fn parse(bytes: &[u8]) -> Result<JsonValue, LiveBootstrapArtifactError> {
    let text = std::str::from_utf8(bytes).map_err(|_| LiveBootstrapArtifactError::InvalidJson)?;
    json::parse_json(text).map_err(|_| LiveBootstrapArtifactError::InvalidJson)
}

#[cfg(test)]
mod tests {
    use super::verify_live_bootstrap_artifact;

    #[test]
    fn copied_live_bootstrap_artifact_is_pinned() {
        assert_eq!(verify_live_bootstrap_artifact(), Ok(()));
    }
}
