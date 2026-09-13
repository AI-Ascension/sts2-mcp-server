// SPDX-License-Identifier: MIT

use crate::json::{self, JsonValue};

#[path = "protocol_artifact_game_information_files.rs"]
mod files;

/// The merged protocol PR is the single update point for this immutable source
/// pin and schema digest.
pub const GAME_INFORMATION_PROTOCOL_VERSION: &str = "game-information-query-v1";
pub const GAME_INFORMATION_PROTOCOL_SOURCE_COMMIT: &str =
    "34f68b182c09472c3a0573ff478e17e6ed53c91f";
pub const GAME_INFORMATION_SCHEMA_DIGEST: &str =
    "376845b0c86b4afcd2c79ffba753eb7e7e416f5410da26b4dae970cfee2221d9";
pub const GAME_INFORMATION_ARTIFACT: &str = "sts2-protocol/game-information-query-v1";
pub const GAME_INFORMATION_SCHEMA_SOURCE: &str = "schemas/game-information-query-v1.schema.json";
pub const GAME_INFORMATION_GENERATOR: &str = "hand-authored";
pub const GAME_INFORMATION_MAX_MESSAGE_BYTES: usize = 262_144;
pub const GAME_INFORMATION_MAX_CURSOR_BYTES: usize = 512;
pub const GAME_INFORMATION_MAX_PAGE_ITEMS: i64 = 128;
pub const GAME_INFORMATION_MAX_PAGE_BYTES: i64 = 262_144;
pub const GAME_INFORMATION_MAX_TEXT_BYTES: i64 = 65_536;

const MANIFEST: &str =
    include_str!("../../../protocol-artifact/game-information-query-v1/manifest.json");
const SCHEMA: &str =
    include_str!("../../../protocol-artifact/game-information-query-v1/schema.json");
const SOURCE_SCHEMA: &str = include_str!("../../../schemas/game-information-query-v1.schema.json");
const GOLDENS: [&str; 8] = [
    include_str!(
        "../../../protocol-artifact/game-information-query-v1/golden/capabilities-response.json"
    ),
    include_str!(
        "../../../protocol-artifact/game-information-query-v1/golden/error-stale-cursor.json"
    ),
    include_str!(
        "../../../protocol-artifact/game-information-query-v1/golden/live-detail-request.json"
    ),
    include_str!(
        "../../../protocol-artifact/game-information-query-v1/golden/live-detail-response.json"
    ),
    include_str!(
        "../../../protocol-artifact/game-information-query-v1/golden/static-page-1-request.json"
    ),
    include_str!(
        "../../../protocol-artifact/game-information-query-v1/golden/static-page-1-response.json"
    ),
    include_str!(
        "../../../protocol-artifact/game-information-query-v1/golden/static-page-2-request.json"
    ),
    include_str!(
        "../../../protocol-artifact/game-information-query-v1/golden/static-page-2-response.json"
    ),
];

/// A deterministic failure while loading the copied game-information artifact.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GameInformationArtifactError {
    ChecksumMismatch,
    InvalidJson,
    ManifestMismatch,
    SchemaMismatch,
}

impl std::fmt::Display for GameInformationArtifactError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("copied game-information artifact is invalid")
    }
}

impl std::error::Error for GameInformationArtifactError {}

/// Checks the checked-in artifact identity and all packaged golden JSON values.
pub fn verify_game_information_artifact() -> Result<(), GameInformationArtifactError> {
    let manifest = parse(MANIFEST)?;
    let expected_provenance = JsonValue::object([
        (
            "source".to_owned(),
            JsonValue::string(GAME_INFORMATION_SCHEMA_SOURCE),
        ),
        (
            "generator".to_owned(),
            JsonValue::string(GAME_INFORMATION_GENERATOR),
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
    if field(&manifest, "artifact") != Some(&JsonValue::string(GAME_INFORMATION_ARTIFACT))
        || field(&manifest, "protocol_version")
            != Some(&JsonValue::string(GAME_INFORMATION_PROTOCOL_VERSION))
        || field(&manifest, "schema") != Some(&JsonValue::string("schema.json"))
        || field(&manifest, "schema_digest")
            != Some(&JsonValue::string(GAME_INFORMATION_SCHEMA_DIGEST))
        || field(&manifest, "provenance") != Some(&expected_provenance)
        || field(&manifest, "consumers") != Some(&expected_consumers)
        || field(&manifest, "checksums") != Some(&JsonValue::string("SHA256SUMS"))
    {
        return Err(GameInformationArtifactError::ManifestMismatch);
    }
    if SCHEMA != SOURCE_SCHEMA
        || crate::protocol_artifact_hash::sha256_hex(SCHEMA.as_bytes())
            != GAME_INFORMATION_SCHEMA_DIGEST
        || field(&parse(SCHEMA)?, "$id")
            != Some(&JsonValue::string("sts2-game-information-query-v1"))
    {
        return Err(GameInformationArtifactError::SchemaMismatch);
    }
    for golden in GOLDENS {
        parse(golden)?;
    }
    files::verify()?;
    Ok(())
}

fn field<'a>(value: &'a JsonValue, key: &str) -> Option<&'a JsonValue> {
    value.as_object()?.get(key)
}

fn parse(text: &str) -> Result<JsonValue, GameInformationArtifactError> {
    json::parse(text).map_err(|_| GameInformationArtifactError::InvalidJson)
}
