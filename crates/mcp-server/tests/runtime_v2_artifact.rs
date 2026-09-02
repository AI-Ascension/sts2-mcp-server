// SPDX-License-Identifier: MIT

use serde_json::Value;
use sts2_mcp_server::{
    RUNTIME_V2_ARTIFACT, RUNTIME_V2_PROTOCOL_VERSION, RUNTIME_V2_SCHEMA_DIGEST,
    verify_runtime_v2_artifact,
};

const MANIFEST: &str = include_str!("../../../protocol-artifact/runtime-v2/manifest.json");
const SCHEMA: &str = include_str!("../../../protocol-artifact/runtime-v2/schema.json");
const SOURCE_SCHEMA: &str = include_str!("../../../schemas/runtime-v2.schema.json");
const CONFORMANCE: &str = include_str!("../../../conformance/cases/runtime-v2.json");

#[test]
fn runtime_v2_mapping_consumes_the_handed_off_artifact_identity() -> Result<(), String> {
    verify_runtime_v2_artifact().map_err(|error| error.to_string())?;
    let manifest: Value = serde_json::from_str(MANIFEST).map_err(|error| error.to_string())?;
    let schema: Value = serde_json::from_str(SCHEMA).map_err(|error| error.to_string())?;
    assert_eq!(manifest["artifact"], RUNTIME_V2_ARTIFACT);
    assert_eq!(manifest["protocol_version"], RUNTIME_V2_PROTOCOL_VERSION);
    assert_eq!(manifest["schema_digest"], RUNTIME_V2_SCHEMA_DIGEST);
    assert_eq!(schema["$id"], "sts2-runtime-v2");
    assert_eq!(SCHEMA.as_bytes(), SOURCE_SCHEMA.as_bytes());
    serde_json::from_str::<Value>(CONFORMANCE).map_err(|error| error.to_string())?;
    assert_eq!(
        SCHEMA.as_bytes(),
        include_bytes!("../../../protocol-artifact/runtime-v2/schema.json")
    );
    Ok(())
}
