// SPDX-License-Identifier: MIT

use crate::json::{self, JsonValue};

pub const RUNTIME_MAP_V1_PROTOCOL_VERSION: &str = "runtime-map-v1";
pub const RUNTIME_MAP_V1_ARTIFACT: &str = "sts2-protocol/runtime-map-v1";
pub const RUNTIME_MAP_V1_SCHEMA_SOURCE: &str = "schemas/runtime-map-v1.schema.json";
pub const RUNTIME_MAP_V1_GENERATOR: &str = "hand-authored";
pub const RUNTIME_MAP_V1_SCHEMA_DIGEST: &str =
    "ceab0d2dfc471d1ec36d12edaf4654b8c7fdced06548bf47265e11c63f98115b";
pub const RUNTIME_MAP_V1_MAX_GENERATION: i64 = 9_007_199_254_740_991;
pub const RUNTIME_MAP_V1_MAX_NODES: usize = 256;
pub const RUNTIME_MAP_V1_MAX_EDGES: usize = 1_024;
pub const RUNTIME_MAP_V1_MAX_BINDINGS: usize = 256;
pub const RUNTIME_MAP_V1_MAX_HISTORY: usize = 256;
pub const RUNTIME_MAP_V1_MAX_MESSAGE_BYTES: usize = 256 * 1024;

const MANIFEST: &str = include_str!("../../../protocol-artifact/runtime-map-v1/manifest.json");
const SCHEMA: &str = include_str!("../../../protocol-artifact/runtime-map-v1/schema.json");
const SNAPSHOT_REQUEST: &str =
    include_str!("../../../protocol-artifact/runtime-map-v1/golden/snapshot-request.json");
const SNAPSHOT_RESPONSE: &str =
    include_str!("../../../protocol-artifact/runtime-map-v1/golden/snapshot-response.json");
const VISIBLE_MAP: &str =
    include_str!("../../../protocol-artifact/runtime-map-v1/golden/visible-map.json");

pub fn verify_runtime_map_artifact() -> Result<(), RuntimeMapArtifactError> {
    let manifest = parse(MANIFEST)?;
    let expected_provenance = JsonValue::object([
        (
            "source".to_owned(),
            JsonValue::string(RUNTIME_MAP_V1_SCHEMA_SOURCE),
        ),
        (
            "generator".to_owned(),
            JsonValue::string(RUNTIME_MAP_V1_GENERATOR),
        ),
        ("license".to_owned(), JsonValue::string("MIT")),
    ]);
    let expected_consumers = JsonValue::Array(
        [
            "sts2-game-mod",
            "sts2-gateway",
            "sts2-harness",
            "sts2-mcp-server",
            "ascension-map-visualizer",
        ]
        .into_iter()
        .map(JsonValue::string)
        .collect(),
    );
    if field(&manifest, "artifact") != Some(&JsonValue::string(RUNTIME_MAP_V1_ARTIFACT))
        || field(&manifest, "protocol_version")
            != Some(&JsonValue::string(RUNTIME_MAP_V1_PROTOCOL_VERSION))
        || field(&manifest, "schema") != Some(&JsonValue::string("schema.json"))
        || field(&manifest, "schema_digest")
            != Some(&JsonValue::string(RUNTIME_MAP_V1_SCHEMA_DIGEST))
        || field(&manifest, "provenance") != Some(&expected_provenance)
        || field(&manifest, "consumers") != Some(&expected_consumers)
    {
        return Err(RuntimeMapArtifactError::ManifestMismatch);
    }
    if field(&parse(SCHEMA)?, "$id") != Some(&JsonValue::string("sts2-runtime-map-v1")) {
        return Err(RuntimeMapArtifactError::SchemaMismatch);
    }
    for fixture in [SNAPSHOT_REQUEST, SNAPSHOT_RESPONSE, VISIBLE_MAP] {
        parse(fixture)?;
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeMapArtifactError {
    InvalidJson,
    ManifestMismatch,
    SchemaMismatch,
}

impl std::fmt::Display for RuntimeMapArtifactError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("copied Runtime-map-v1 artifact is invalid")
    }
}

impl std::error::Error for RuntimeMapArtifactError {}

fn field<'a>(value: &'a JsonValue, key: &str) -> Option<&'a JsonValue> {
    value.as_object()?.get(key)
}

fn parse(text: &str) -> Result<JsonValue, RuntimeMapArtifactError> {
    json::parse(text).map_err(|_| RuntimeMapArtifactError::InvalidJson)
}
