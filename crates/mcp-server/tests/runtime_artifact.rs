// SPDX-License-Identifier: MIT

use serde_json::Value;
use sts2_mcp_server::{RUNTIME_ARTIFACT, RUNTIME_PROTOCOL_VERSION, RUNTIME_SCHEMA_DIGEST};

const MANIFEST: &str = include_str!("../../../protocol-artifact/runtime-v1/manifest.json");
const SOURCE_SCHEMA: &str = include_str!("../../../protocol-artifact/runtime-v1/schema.json");

#[test]
fn runtime_mapping_uses_the_named_schema_artifact() {
    let manifest_result: Result<Value, _> = serde_json::from_str(MANIFEST);
    let schema_result: Result<Value, _> = serde_json::from_str(SOURCE_SCHEMA);
    assert!(manifest_result.is_ok());
    assert!(schema_result.is_ok());
    let manifest = manifest_result.unwrap_or(Value::Null);
    let schema = schema_result.unwrap_or(Value::Null);
    assert_eq!(manifest["artifact"], RUNTIME_ARTIFACT);
    assert_eq!(manifest["protocol_version"], RUNTIME_PROTOCOL_VERSION);
    assert_eq!(manifest["schema_digest"], RUNTIME_SCHEMA_DIGEST);
    assert_eq!(schema["$id"], "sts2-runtime-v1");
    assert_eq!(
        SOURCE_SCHEMA.as_bytes(),
        include_bytes!("../../../protocol-artifact/runtime-v1/schema.json")
    );
}
