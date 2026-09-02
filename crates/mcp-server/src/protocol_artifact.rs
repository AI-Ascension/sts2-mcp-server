// SPDX-License-Identifier: MIT

use crate::json::{self, JsonValue};

/// Version consumed by the MCP POC mapping.
pub const POC_PROTOCOL_VERSION: &str = "poc-v1";
/// Schema digest supplied by the protocol release-like artifact.
pub const POC_SCHEMA_DIGEST: &str =
    "adb434d119a51b00d968e71bf0bf774f2a08de7c875a5479900aa34b3c02e027";
/// Release-like artifact identity, not a Rust package dependency.
pub const POC_ARTIFACT: &str = "sts2-protocol/poc-v1";
/// Repository-relative source recorded in the artifact provenance.
pub const POC_SCHEMA_SOURCE: &str = "schemas/poc-v1.schema.json";
/// Generator recorded in the hand-authored artifact.
pub const POC_GENERATOR: &str = "hand-authored";
/// Maximum fake budget represented by the bounded contract.
pub const POC_MAX_UNITS: u16 = 8;
/// Maximum settled-effect count represented by the bounded contract.
pub const POC_MAX_SETTLED_EFFECTS: u16 = 4;

const MANIFEST: &str = include_str!("../../../protocol-artifact/poc-v1/manifest.json");
const SCHEMA: &str = include_str!("../../../protocol-artifact/poc-v1/schema.json");
const STATE_RESPONSE: &str =
    include_str!("../../../protocol-artifact/poc-v1/golden/state-response.json");
const ACTION_RESPONSE: &str =
    include_str!("../../../protocol-artifact/poc-v1/golden/action-accepted.json");
const INVALID_ACTION: &str =
    include_str!("../../../protocol-artifact/poc-v1/fixtures/invalid-action.json");

/// Verifies the local copied artifact before POC mapping tests use it.
pub fn verify_poc_artifact() -> Result<(), ArtifactError> {
    let manifest = parse(MANIFEST)?;
    if field(&manifest, "artifact") != Some(&JsonValue::string(POC_ARTIFACT))
        || field(&manifest, "protocol_version") != Some(&JsonValue::string(POC_PROTOCOL_VERSION))
        || field(&manifest, "schema_digest") != Some(&JsonValue::string(POC_SCHEMA_DIGEST))
    {
        return Err(ArtifactError::ManifestMismatch);
    }
    if field(&parse(SCHEMA)?, "$id") != Some(&JsonValue::string("sts2-poc-v1")) {
        return Err(ArtifactError::SchemaMismatch);
    }
    parse(STATE_RESPONSE)?;
    parse(ACTION_RESPONSE)?;
    parse(INVALID_ACTION)?;
    Ok(())
}

/// A deterministic failure while loading the copied artifact.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArtifactError {
    InvalidJson,
    ManifestMismatch,
    SchemaMismatch,
}

impl std::fmt::Display for ArtifactError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("copied POC artifact is invalid")
    }
}

impl std::error::Error for ArtifactError {}

fn field<'a>(value: &'a JsonValue, key: &str) -> Option<&'a JsonValue> {
    value.as_object()?.get(key)
}

fn parse(text: &str) -> Result<JsonValue, ArtifactError> {
    json::parse(text).map_err(|_| ArtifactError::InvalidJson)
}
