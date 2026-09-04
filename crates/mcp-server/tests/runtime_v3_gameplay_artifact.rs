// SPDX-License-Identifier: MIT

#[path = "../src/protocol_artifact_runtime_v2_hash.rs"]
mod hash;

use std::collections::BTreeSet;
use std::path::Path;

use serde_json::Value;
use sts2_mcp_server::{
    GatewayAdapter, GatewayError, GatewayRequest, GatewayResponse, McpServer,
    RUNTIME_V3_GAMEPLAY_SCHEMA_DIGEST, ToolCatalog, parse_json,
};

const SCHEMA: &str = include_str!("../../../protocol-artifact/runtime-v3-gameplay/schema.json");
const MANIFEST: &str = include_str!("../../../protocol-artifact/runtime-v3-gameplay/manifest.json");
const CHECKSUMS: &str = include_str!("../../../protocol-artifact/runtime-v3-gameplay/SHA256SUMS");

#[test]
fn canonical_package_checksums_schema_and_all_goldens_agree() -> Result<(), String> {
    let root =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../protocol-artifact/runtime-v3-gameplay");
    let manifest: Value = serde_json::from_str(MANIFEST).map_err(|error| error.to_string())?;
    let schema: Value = serde_json::from_str(SCHEMA).map_err(|error| error.to_string())?;
    assert_eq!(manifest["schema_digest"], RUNTIME_V3_GAMEPLAY_SCHEMA_DIGEST);
    assert_eq!(
        hash::sha256_hex(SCHEMA.as_bytes()),
        RUNTIME_V3_GAMEPLAY_SCHEMA_DIGEST
    );
    assert_eq!(
        SCHEMA,
        include_str!("../../../schemas/runtime-v3-gameplay.schema.json")
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
        assert!(validator.is_valid(&value), "{path}");
    }
    Ok(())
}

struct CanonicalGateway {
    response: sts2_mcp_server::JsonValue,
    expected: Value,
}

impl GatewayAdapter for CanonicalGateway {
    fn forward(&mut self, request: GatewayRequest) -> Result<GatewayResponse, GatewayError> {
        let Some(body) = request.body else {
            return Err(GatewayError::Rejected);
        };
        let actual: Value =
            serde_json::from_str(&body.to_json()).map_err(|_| GatewayError::Rejected)?;
        assert_eq!(actual, self.expected);
        Ok(GatewayResponse {
            status: 200,
            body: self.response.clone(),
        })
    }
}

#[test]
fn canonical_dispatch_request_and_receipt_cross_both_adapter_boundaries() -> Result<(), String> {
    let expected: Value = serde_json::from_str(include_str!(
        "../../../protocol-artifact/runtime-v3-gameplay/golden/dispatch-action-request.json"
    ))
    .map_err(|error| error.to_string())?;
    let response = parse_json(include_str!(
        "../../../protocol-artifact/runtime-v3-gameplay/golden/dispatch-action-settled.json"
    ))
    .map_err(|error| error.to_string())?;
    let mut server = McpServer::with_catalog(
        CanonicalGateway { expected, response },
        ToolCatalog::runtime_v3_gameplay(),
    );
    let output = server.handle_frame(r#"{"jsonrpc":"2.0","id":"corr-2","method":"tools/call","params":{"name":"sts2.dispatch_action","arguments":{"instance_id":"instance-1","mcp_session_id":"session-1","lease_id":"lease-1","lease_epoch":1,"generation":0,"state_id":"combat-1","operation_id":"op-1","action":{"action_id":"combat.end-turn","action":{"kind":"end_turn"}}}}}"#);
    let wire: Value = serde_json::from_str(&output).map_err(|error| error.to_string())?;
    assert_eq!(wire["result"]["isError"], false, "{output}");
    let projected: Value = serde_json::from_str(
        wire["result"]["content"][0]["text"]
            .as_str()
            .ok_or("missing projection")?,
    )
    .map_err(|error| error.to_string())?;
    let expected: Value = serde_json::from_str(include_str!(
        "../../../protocol-artifact/runtime-v3-gameplay/golden/dispatch-action-settled.json"
    ))
    .map_err(|error| error.to_string())?;
    assert_eq!(projected, expected);
    Ok(())
}
