// SPDX-License-Identifier: MIT

#[path = "../src/protocol_artifact_runtime_v2_hash.rs"]
mod hash;

use std::collections::BTreeSet;
use std::path::Path;

use serde_json::Value;
use sts2_mcp_server::{RUNTIME_MAP_V1_SCHEMA_DIGEST, verify_runtime_map_artifact};

const SCHEMA: &str = include_str!("../../../protocol-artifact/runtime-map-v1/schema.json");
const MANIFEST: &str = include_str!("../../../protocol-artifact/runtime-map-v1/manifest.json");
const CHECKSUMS: &str = include_str!("../../../protocol-artifact/runtime-map-v1/SHA256SUMS");

#[test]
fn copied_runtime_map_artifact_is_complete_and_schema_valid() -> Result<(), String> {
    verify_runtime_map_artifact().map_err(|error| error.to_string())?;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../protocol-artifact/runtime-map-v1");
    let manifest: Value = serde_json::from_str(MANIFEST).map_err(|error| error.to_string())?;
    let schema: Value = serde_json::from_str(SCHEMA).map_err(|error| error.to_string())?;
    assert_eq!(manifest["schema_digest"], RUNTIME_MAP_V1_SCHEMA_DIGEST);
    assert_eq!(
        hash::sha256_hex(SCHEMA.as_bytes()),
        RUNTIME_MAP_V1_SCHEMA_DIGEST
    );
    let validator = jsonschema::draft202012::options()
        .build(&schema)
        .map_err(|error| error.to_string())?;
    let mut verified = BTreeSet::new();
    for line in CHECKSUMS.lines() {
        let (digest, path) = line.split_once("  ").ok_or("malformed checksum")?;
        assert!(verified.insert(path));
        let bytes = std::fs::read(root.join(path)).map_err(|error| error.to_string())?;
        assert_eq!(hash::sha256_hex(&bytes), digest, "{path}");
    }
    let goldens = manifest["goldens"]
        .as_array()
        .ok_or("missing golden inventory")?;
    assert_eq!(verified.len(), goldens.len() + 4);
    for path in goldens {
        let path = path.as_str().ok_or("invalid golden path")?;
        assert!(verified.contains(path));
        let text = std::fs::read_to_string(root.join(path)).map_err(|error| error.to_string())?;
        let value: Value = serde_json::from_str(&text).map_err(|error| error.to_string())?;
        if path != "golden/visible-map.json" {
            assert!(validator.is_valid(&value), "{path}");
        }
    }
    Ok(())
}
