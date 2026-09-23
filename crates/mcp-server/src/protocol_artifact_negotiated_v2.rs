// SPDX-License-Identifier: MIT

use std::sync::OnceLock;

use crate::json::JsonValue;

pub const NEGOTIATED_CAPABILITIES_V2_PROTOCOL_VERSION: &str =
    "sts2-gateway-negotiated-capabilities-v2";
pub const NEGOTIATED_CAPABILITIES_V2_SCHEMA_DIGEST: &str =
    "c6453f1a760675c7492261eb7b50be76cf87d8c7d4762a070d26693b15225b7f";
/// Gateway source pin carrying the immutable artifact consumed here.
pub const NEGOTIATED_CAPABILITIES_V2_SOURCE_COMMIT: &str =
    "bfe28e455de48d6d9db466bbcf6062ab5d85e9af";
pub const NEGOTIATED_CAPABILITIES_V2_MAX_BYTES: usize = 16 * 1024;

const MANIFEST: &str =
    include_str!("../../../protocol-artifact/negotiated-capabilities-v2/manifest.json");
const SCHEMA: &str =
    include_str!("../../../protocol-artifact/negotiated-capabilities-v2/schema.json");
const CHECKSUMS: &str =
    include_str!("../../../protocol-artifact/negotiated-capabilities-v2/SHA256SUMS");

struct ArtifactFile {
    path: &'static str,
    bytes: &'static [u8],
}

const FILES: &[ArtifactFile] = &[
    ArtifactFile {
        path: "README.md",
        bytes: include_bytes!("../../../protocol-artifact/negotiated-capabilities-v2/README.md"),
    },
    ArtifactFile {
        path: "limit-examples.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/negotiated-capabilities-v2/limit-examples.json"
        ),
    },
    ArtifactFile {
        path: "manifest.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/negotiated-capabilities-v2/manifest.json"
        ),
    },
    ArtifactFile {
        path: "schema.json",
        bytes: include_bytes!("../../../protocol-artifact/negotiated-capabilities-v2/schema.json"),
    },
];

static VALIDATOR: OnceLock<Result<jsonschema::Validator, ()>> = OnceLock::new();

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NegotiatedCapabilitiesV2ArtifactError {
    ChecksumMismatch,
    InvalidJson,
    ManifestMismatch,
    SchemaMismatch,
    SnapshotTooLarge,
    SnapshotInvalid,
}

impl std::fmt::Display for NegotiatedCapabilitiesV2ArtifactError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("negotiated capabilities artifact or snapshot is invalid")
    }
}

impl std::error::Error for NegotiatedCapabilitiesV2ArtifactError {}

pub fn verify_negotiated_capabilities_v2_artifact()
-> Result<(), NegotiatedCapabilitiesV2ArtifactError> {
    let manifest = crate::json::parse(MANIFEST)
        .map_err(|_| NegotiatedCapabilitiesV2ArtifactError::InvalidJson)?;
    if member(&manifest, "artifact")
        != Some(&JsonValue::string(
            "sts2-gateway/negotiated-capabilities-v2",
        ))
        || member(&manifest, "protocol_version")
            != Some(&JsonValue::string(
                NEGOTIATED_CAPABILITIES_V2_PROTOCOL_VERSION,
            ))
        || member(&manifest, "schema_digest")
            != Some(&JsonValue::string(NEGOTIATED_CAPABILITIES_V2_SCHEMA_DIGEST))
        || member(&manifest, "schema") != Some(&JsonValue::string("schema.json"))
        || crate::protocol_artifact_hash::sha256_hex(SCHEMA.as_bytes())
            != NEGOTIATED_CAPABILITIES_V2_SCHEMA_DIGEST
        || crate::json::parse(SCHEMA)
            .ok()
            .and_then(|schema| member(&schema, "title").cloned())
            != Some(JsonValue::string("Gateway negotiated capabilities v2"))
    {
        return Err(NegotiatedCapabilitiesV2ArtifactError::ManifestMismatch);
    }
    verify_checksums()
}

pub fn validate_negotiated_capabilities_v2_snapshot(
    input: &str,
) -> Result<JsonValue, NegotiatedCapabilitiesV2ArtifactError> {
    verify_negotiated_capabilities_v2_artifact()?;
    if input.len() > NEGOTIATED_CAPABILITIES_V2_MAX_BYTES {
        return Err(NegotiatedCapabilitiesV2ArtifactError::SnapshotTooLarge);
    }
    let value = crate::json::parse(input)
        .map_err(|_| NegotiatedCapabilitiesV2ArtifactError::InvalidJson)?;
    let validator = VALIDATOR
        .get_or_init(|| {
            let schema = serde_json::from_str(SCHEMA).map_err(|_| ())?;
            jsonschema::draft202012::options()
                .build(&schema)
                .map_err(|_| ())
        })
        .as_ref()
        .map_err(|_| NegotiatedCapabilitiesV2ArtifactError::SchemaMismatch)?;
    let json: serde_json::Value = serde_json::from_str(&value.to_json())
        .map_err(|_| NegotiatedCapabilitiesV2ArtifactError::InvalidJson)?;
    if !validator.is_valid(&json) {
        return Err(NegotiatedCapabilitiesV2ArtifactError::SnapshotInvalid);
    }
    Ok(value)
}

fn verify_checksums() -> Result<(), NegotiatedCapabilitiesV2ArtifactError> {
    let mut verified = Vec::new();
    for line in CHECKSUMS.lines() {
        let (digest, checksum_path) = line
            .split_once("  ")
            .ok_or(NegotiatedCapabilitiesV2ArtifactError::ChecksumMismatch)?;
        // sha256sum emitted these paths from the artifact directory and keeps
        // the conventional "./" prefix. Accept that exact equivalent spelling
        // while still requiring every packaged file exactly once.
        let path = checksum_path.strip_prefix("./").unwrap_or(checksum_path);
        let file = FILES
            .iter()
            .find(|file| file.path == path)
            .ok_or(NegotiatedCapabilitiesV2ArtifactError::ChecksumMismatch)?;
        if digest.len() != 64
            || verified.contains(&path)
            || crate::protocol_artifact_hash::sha256_hex(file.bytes) != digest
        {
            return Err(NegotiatedCapabilitiesV2ArtifactError::ChecksumMismatch);
        }
        verified.push(path);
    }
    if verified.len() != FILES.len() || FILES.iter().any(|file| !verified.contains(&file.path)) {
        return Err(NegotiatedCapabilitiesV2ArtifactError::ChecksumMismatch);
    }
    Ok(())
}

fn member<'a>(value: &'a JsonValue, name: &str) -> Option<&'a JsonValue> {
    value.as_object()?.get(name)
}

#[cfg(test)]
#[path = "protocol_artifact_negotiated_v2_tests.rs"]
mod tests;
