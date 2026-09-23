// SPDX-License-Identifier: MIT

use std::sync::OnceLock;

use crate::json::JsonValue;

pub const NEGOTIATED_CAPABILITIES_PROTOCOL_VERSION: &str =
    "sts2-gateway-negotiated-capabilities-v1";
pub const NEGOTIATED_CAPABILITIES_SCHEMA_DIGEST: &str =
    "447c066568897ef720c07ade7963c97ba037645eccb9a8b71a0724d0d22fd299";
/// Gateway source pin carrying the immutable artifact consumed here.
pub const NEGOTIATED_CAPABILITIES_SOURCE_COMMIT: &str = "e15248cd41f89188706a8a19e974f97bf5880a9f";
pub const NEGOTIATED_CAPABILITIES_MAX_BYTES: usize = 16 * 1024;

const MANIFEST: &str =
    include_str!("../../../protocol-artifact/negotiated-capabilities-v1/manifest.json");
const SCHEMA: &str =
    include_str!("../../../protocol-artifact/negotiated-capabilities-v1/schema.json");
const CHECKSUMS: &str =
    include_str!("../../../protocol-artifact/negotiated-capabilities-v1/SHA256SUMS");

struct ArtifactFile {
    path: &'static str,
    bytes: &'static [u8],
}

const FILES: &[ArtifactFile] = &[
    ArtifactFile {
        path: "README.md",
        bytes: include_bytes!("../../../protocol-artifact/negotiated-capabilities-v1/README.md"),
    },
    ArtifactFile {
        path: "limit-examples.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/negotiated-capabilities-v1/limit-examples.json"
        ),
    },
    ArtifactFile {
        path: "manifest.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/negotiated-capabilities-v1/manifest.json"
        ),
    },
    ArtifactFile {
        path: "schema.json",
        bytes: include_bytes!("../../../protocol-artifact/negotiated-capabilities-v1/schema.json"),
    },
];

static VALIDATOR: OnceLock<Result<jsonschema::Validator, ()>> = OnceLock::new();

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NegotiatedCapabilitiesArtifactError {
    ChecksumMismatch,
    InvalidJson,
    ManifestMismatch,
    SchemaMismatch,
    SnapshotTooLarge,
    SnapshotInvalid,
}

impl std::fmt::Display for NegotiatedCapabilitiesArtifactError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("negotiated capabilities artifact or snapshot is invalid")
    }
}

impl std::error::Error for NegotiatedCapabilitiesArtifactError {}

pub fn verify_negotiated_capabilities_artifact() -> Result<(), NegotiatedCapabilitiesArtifactError>
{
    let manifest = crate::json::parse(MANIFEST)
        .map_err(|_| NegotiatedCapabilitiesArtifactError::InvalidJson)?;
    if member(&manifest, "artifact")
        != Some(&JsonValue::string(
            "sts2-gateway/negotiated-capabilities-v1",
        ))
        || member(&manifest, "protocol_version")
            != Some(&JsonValue::string(NEGOTIATED_CAPABILITIES_PROTOCOL_VERSION))
        || member(&manifest, "schema_digest")
            != Some(&JsonValue::string(NEGOTIATED_CAPABILITIES_SCHEMA_DIGEST))
        || member(&manifest, "schema") != Some(&JsonValue::string("schema.json"))
        || crate::protocol_artifact_hash::sha256_hex(SCHEMA.as_bytes())
            != NEGOTIATED_CAPABILITIES_SCHEMA_DIGEST
        || crate::json::parse(SCHEMA)
            .ok()
            .and_then(|schema| member(&schema, "title").cloned())
            != Some(JsonValue::string("Gateway negotiated capabilities v1"))
    {
        return Err(NegotiatedCapabilitiesArtifactError::ManifestMismatch);
    }
    verify_checksums()
}

pub fn validate_negotiated_capabilities_snapshot(
    input: &str,
) -> Result<JsonValue, NegotiatedCapabilitiesArtifactError> {
    verify_negotiated_capabilities_artifact()?;
    if input.len() > NEGOTIATED_CAPABILITIES_MAX_BYTES {
        return Err(NegotiatedCapabilitiesArtifactError::SnapshotTooLarge);
    }
    let value =
        crate::json::parse(input).map_err(|_| NegotiatedCapabilitiesArtifactError::InvalidJson)?;
    let validator = VALIDATOR
        .get_or_init(|| {
            let schema = serde_json::from_str(SCHEMA).map_err(|_| ())?;
            jsonschema::draft202012::options()
                .build(&schema)
                .map_err(|_| ())
        })
        .as_ref()
        .map_err(|_| NegotiatedCapabilitiesArtifactError::SchemaMismatch)?;
    let json: serde_json::Value = serde_json::from_str(&value.to_json())
        .map_err(|_| NegotiatedCapabilitiesArtifactError::InvalidJson)?;
    if !validator.is_valid(&json) {
        return Err(NegotiatedCapabilitiesArtifactError::SnapshotInvalid);
    }
    Ok(value)
}

fn verify_checksums() -> Result<(), NegotiatedCapabilitiesArtifactError> {
    let mut verified = Vec::new();
    for line in CHECKSUMS.lines() {
        let (digest, checksum_path) = line
            .split_once("  ")
            .ok_or(NegotiatedCapabilitiesArtifactError::ChecksumMismatch)?;
        // sha256sum emitted these paths from the artifact directory and keeps
        // the conventional "./" prefix. Accept that exact equivalent spelling
        // while still requiring every packaged file exactly once.
        let path = checksum_path.strip_prefix("./").unwrap_or(checksum_path);
        let file = FILES
            .iter()
            .find(|file| file.path == path)
            .ok_or(NegotiatedCapabilitiesArtifactError::ChecksumMismatch)?;
        if digest.len() != 64
            || verified.contains(&path)
            || crate::protocol_artifact_hash::sha256_hex(file.bytes) != digest
        {
            return Err(NegotiatedCapabilitiesArtifactError::ChecksumMismatch);
        }
        verified.push(path);
    }
    if verified.len() != FILES.len() || FILES.iter().any(|file| !verified.contains(&file.path)) {
        return Err(NegotiatedCapabilitiesArtifactError::ChecksumMismatch);
    }
    Ok(())
}

fn member<'a>(value: &'a JsonValue, name: &str) -> Option<&'a JsonValue> {
    value.as_object()?.get(name)
}

#[cfg(test)]
#[path = "protocol_artifact_negotiated_tests.rs"]
mod tests;
