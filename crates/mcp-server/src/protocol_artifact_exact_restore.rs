// SPDX-License-Identifier: MIT

use crate::json::{self, JsonValue};

#[path = "protocol_artifact_exact_restore_files.rs"]
mod files;

pub const EXACT_RESTORE_PROTOCOL_VERSION: &str = "exact-restore-v1";
pub const EXACT_RESTORE_PROTOCOL_ARTIFACT: &str = "sts2-protocol/exact-restore-v1";
pub const EXACT_RESTORE_PROTOCOL_PRODUCER_COMMIT: &str = "5d5a368ef8a89fd1cb356b04dbf9d8a056adbf05";
pub const EXACT_RESTORE_PROTOCOL_SCHEMA_DIGEST: &str =
    "2289d888c33eac46873408303c4423eab762e3f7bd6132ae8ae88d0d3b1858e4";
pub const EXACT_RESTORE_GATEWAY_CONTRACT: &str = "sts2-exact-restore-gateway-v1";
pub const EXACT_RESTORE_GATEWAY_SCHEMA_DIGEST: &str =
    "0b181dc30524c8b14dea73e490da55538f2d57fe87bf58ed9fe33223406a7d89";
pub const EXACT_RESTORE_MAX_FRAME_BYTES: usize = 16 * 1024;
pub const EXACT_RESTORE_MAX_CHUNK_RAW_BYTES: usize = 8192;
pub const EXACT_RESTORE_MAX_CHUNK_BASE64_BYTES: usize = 10_924;

const NEUTRAL_MANIFEST: &[u8] =
    include_bytes!("../../../protocol-artifact/exact-restore-v1/manifest.json");
const NEUTRAL_SCHEMA: &[u8] =
    include_bytes!("../../../protocol-artifact/exact-restore-v1/schema.json");
const NEUTRAL_SOURCE_SCHEMA: &[u8] =
    include_bytes!("../../../schemas/exact-restore-v1.schema.json");
const NEUTRAL_CHECKSUMS: &str =
    include_str!("../../../protocol-artifact/exact-restore-v1/SHA256SUMS");
const GATEWAY_MANIFEST: &[u8] =
    include_bytes!("../../../protocol-artifact/exact-restore-gateway-v1/manifest.json");
const GATEWAY_SCHEMA: &[u8] =
    include_bytes!("../../../protocol-artifact/exact-restore-gateway-v1/schema.json");
const GATEWAY_SOURCE_SCHEMA: &[u8] =
    include_bytes!("../../../schemas/exact-restore-gateway-v1.schema.json");
const GATEWAY_CHECKSUMS: &str =
    include_str!("../../../protocol-artifact/exact-restore-gateway-v1/SHA256SUMS");

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExactRestoreArtifactError {
    ChecksumMismatch,
    InvalidJson,
    ManifestMismatch,
    SchemaMismatch,
}

impl std::fmt::Display for ExactRestoreArtifactError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("copied exact-restore artifact is invalid")
    }
}

impl std::error::Error for ExactRestoreArtifactError {}

pub fn verify_exact_restore_artifact() -> Result<(), ExactRestoreArtifactError> {
    let neutral_manifest = parse(NEUTRAL_MANIFEST)?;
    if field(&neutral_manifest, "artifact")
        != Some(&JsonValue::string(EXACT_RESTORE_PROTOCOL_ARTIFACT))
        || field(&neutral_manifest, "protocol_version")
            != Some(&JsonValue::string(EXACT_RESTORE_PROTOCOL_VERSION))
        || field(&neutral_manifest, "schema") != Some(&JsonValue::string("schema.json"))
        || field(&neutral_manifest, "schema_digest")
            != Some(&JsonValue::string(EXACT_RESTORE_PROTOCOL_SCHEMA_DIGEST))
        || field(&neutral_manifest, "checksums") != Some(&JsonValue::string("SHA256SUMS"))
        || NEUTRAL_SCHEMA != NEUTRAL_SOURCE_SCHEMA
        || crate::protocol_artifact_hash::sha256_hex(NEUTRAL_SCHEMA)
            != EXACT_RESTORE_PROTOCOL_SCHEMA_DIGEST
    {
        return Err(ExactRestoreArtifactError::ManifestMismatch);
    }
    let gateway_manifest = parse(GATEWAY_MANIFEST)?;
    if field(&gateway_manifest, "artifact")
        != Some(&JsonValue::string(
            "sts2-mcp-server/exact-restore-gateway-v1",
        ))
        || field(&gateway_manifest, "contract")
            != Some(&JsonValue::string(EXACT_RESTORE_GATEWAY_CONTRACT))
        || field(&gateway_manifest, "schema") != Some(&JsonValue::string("schema.json"))
        || field(&gateway_manifest, "schema_digest")
            != Some(&JsonValue::string(EXACT_RESTORE_GATEWAY_SCHEMA_DIGEST))
        || field(
            field(&gateway_manifest, "neutral_protocol").unwrap_or(&JsonValue::Null),
            "artifact",
        ) != Some(&JsonValue::string(EXACT_RESTORE_PROTOCOL_ARTIFACT))
        || field(
            field(&gateway_manifest, "neutral_protocol").unwrap_or(&JsonValue::Null),
            "producer_commit",
        ) != Some(&JsonValue::string(EXACT_RESTORE_PROTOCOL_PRODUCER_COMMIT))
        || field(
            field(&gateway_manifest, "neutral_protocol").unwrap_or(&JsonValue::Null),
            "schema_digest",
        ) != Some(&JsonValue::string(EXACT_RESTORE_PROTOCOL_SCHEMA_DIGEST))
        || field(&gateway_manifest, "checksums") != Some(&JsonValue::string("SHA256SUMS"))
        || GATEWAY_SCHEMA != GATEWAY_SOURCE_SCHEMA
        || crate::protocol_artifact_hash::sha256_hex(GATEWAY_SCHEMA)
            != EXACT_RESTORE_GATEWAY_SCHEMA_DIGEST
    {
        return Err(ExactRestoreArtifactError::ManifestMismatch);
    }
    for file in files::NEUTRAL_FILES.iter().chain(files::GATEWAY_FILES) {
        if !file.path.ends_with(".json") {
            continue;
        }
        let text =
            std::str::from_utf8(file.bytes).map_err(|_| ExactRestoreArtifactError::InvalidJson)?;
        serde_json::from_str::<serde_json::Value>(text)
            .map_err(|_| ExactRestoreArtifactError::InvalidJson)?;
    }
    verify_checksums(NEUTRAL_CHECKSUMS, files::NEUTRAL_FILES)?;
    verify_checksums(GATEWAY_CHECKSUMS, files::GATEWAY_FILES)?;
    Ok(())
}

fn field<'a>(value: &'a JsonValue, key: &str) -> Option<&'a JsonValue> {
    value.as_object()?.get(key)
}

fn parse(bytes: &[u8]) -> Result<JsonValue, ExactRestoreArtifactError> {
    let text = std::str::from_utf8(bytes).map_err(|_| ExactRestoreArtifactError::InvalidJson)?;
    json::parse(text).map_err(|_| ExactRestoreArtifactError::InvalidJson)
}

fn verify_checksums(
    checksums: &str,
    files: &[files::ArtifactFile],
) -> Result<(), ExactRestoreArtifactError> {
    let mut seen = Vec::new();
    for line in checksums.lines() {
        let (expected, path) = line
            .split_once("  ")
            .ok_or(ExactRestoreArtifactError::ChecksumMismatch)?;
        let file = files
            .iter()
            .find(|file| file.path == path)
            .ok_or(ExactRestoreArtifactError::ChecksumMismatch)?;
        if expected.len() != 64
            || !expected
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            || seen.contains(&path)
            || crate::protocol_artifact_hash::sha256_hex(file.bytes) != expected
        {
            return Err(ExactRestoreArtifactError::ChecksumMismatch);
        }
        seen.push(path);
    }
    if seen.len() != files.len() || files.iter().any(|file| !seen.contains(&file.path)) {
        return Err(ExactRestoreArtifactError::ChecksumMismatch);
    }
    Ok(())
}
