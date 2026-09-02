// SPDX-License-Identifier: MIT

use crate::json::{self, JsonValue};

/// Version consumed by the MCP POC mapping.
pub const POC_PROTOCOL_VERSION: &str = "poc-v1";
/// Schema digest supplied by the protocol release-like artifact.
pub const POC_SCHEMA_DIGEST: &str =
    "242b8f9233e915a55ea8d2e72ca476c1258169a67e62de72ee5aed848a6a0a19";
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
/// Maximum generation that remains exact in common JSON number implementations.
pub const POC_MAX_GENERATION: i64 = 9_007_199_254_740_991;

/// Version consumed by the live runtime mapping.
pub const RUNTIME_PROTOCOL_VERSION: &str = "runtime-v1";
/// Schema digest supplied by the runtime protocol artifact.
pub const RUNTIME_SCHEMA_DIGEST: &str =
    "a76086d7a68668fd4cff53999369d2b450b0d6623827393882f458f2aa1f93eb";
/// Release-like runtime artifact identity.
pub const RUNTIME_ARTIFACT: &str = "sts2-protocol/runtime-v1";
/// Runtime schema provenance source.
pub const RUNTIME_SCHEMA_SOURCE: &str = "schemas/runtime-v1.schema.json";
/// Runtime schema generator.
pub const RUNTIME_GENERATOR: &str = "hand-authored";
/// Fixed action admitted by the runtime mapping.
pub const RUNTIME_ACTION_ID: &str = "show_runtime_probe";
/// Maximum runtime generation represented exactly in JSON.
pub const RUNTIME_MAX_GENERATION: i64 = 9_007_199_254_740_991;

const MANIFEST: &str = include_str!("../../../protocol-artifact/poc-v1/manifest.json");
const SCHEMA: &str = include_str!("../../../protocol-artifact/poc-v1/schema.json");
const STATE_REQUEST: &str =
    include_str!("../../../protocol-artifact/poc-v1/golden/state-request.json");
const STATE_RESPONSE: &str =
    include_str!("../../../protocol-artifact/poc-v1/golden/state-response.json");
const ACTION_REQUEST: &str =
    include_str!("../../../protocol-artifact/poc-v1/golden/action-request.json");
const ACTION_RESPONSE: &str =
    include_str!("../../../protocol-artifact/poc-v1/golden/action-accepted.json");
const ACTION_REJECTED: &str =
    include_str!("../../../protocol-artifact/poc-v1/golden/action-rejected.json");
const INVALID_ACTION: &str =
    include_str!("../../../protocol-artifact/poc-v1/fixtures/invalid-action.json");

/// Verifies the local copied artifact before POC mapping tests use it.
pub fn verify_poc_artifact() -> Result<(), ArtifactError> {
    let manifest = parse(MANIFEST)?;
    let expected_provenance = JsonValue::object([
        ("source".to_owned(), JsonValue::string(POC_SCHEMA_SOURCE)),
        ("generator".to_owned(), JsonValue::string(POC_GENERATOR)),
        ("license".to_owned(), JsonValue::string("MIT")),
    ]);
    let expected_consumers = JsonValue::Array(vec![
        JsonValue::string("sts2-game-core"),
        JsonValue::string("sts2-game-mod"),
        JsonValue::string("sts2-gateway"),
        JsonValue::string("sts2-harness"),
        JsonValue::string("sts2-mcp-server"),
    ]);
    if field(&manifest, "artifact") != Some(&JsonValue::string(POC_ARTIFACT))
        || field(&manifest, "protocol_version") != Some(&JsonValue::string(POC_PROTOCOL_VERSION))
        || field(&manifest, "schema") != Some(&JsonValue::string("schema.json"))
        || field(&manifest, "schema_digest") != Some(&JsonValue::string(POC_SCHEMA_DIGEST))
        || field(&manifest, "provenance") != Some(&expected_provenance)
        || field(&manifest, "consumers") != Some(&expected_consumers)
    {
        return Err(ArtifactError::ManifestMismatch);
    }
    if field(&parse(SCHEMA)?, "$id") != Some(&JsonValue::string("sts2-poc-v1")) {
        return Err(ArtifactError::SchemaMismatch);
    }
    for fixture in [
        STATE_REQUEST,
        STATE_RESPONSE,
        ACTION_REQUEST,
        ACTION_RESPONSE,
        ACTION_REJECTED,
        INVALID_ACTION,
    ] {
        parse(fixture)?;
    }
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
