// SPDX-License-Identifier: MIT

use jsonschema::{PatternOptions, draft202012::options};
use serde_json::Value;
use sts2_mcp_server::{
    RECOVERY_ARTIFACT, RECOVERY_PROTOCOL_VERSION, RECOVERY_SCHEMA_DIGEST, verify_recovery_artifact,
};

const MANIFEST: &str =
    include_str!("../../../protocol-artifact/watchdog-recovery-v1/manifest.json");
const SCHEMA: &[u8] = include_bytes!("../../../protocol-artifact/watchdog-recovery-v1/schema.json");
const CONFORMANCE: &str =
    include_str!("../../../protocol-artifact/watchdog-recovery-v1/conformance.json");
const RCJ_VECTORS: &str =
    include_str!("../../../protocol-artifact/watchdog-recovery-v1/rcj-vectors.json");
const VALID_FIXTURES: [(&str, &str); 18] = [
    (
        "bootstrap-request",
        include_str!(
            "../../../protocol-artifact/watchdog-recovery-v1/fixtures/valid/bootstrap-request.json"
        ),
    ),
    (
        "bootstrap-response",
        include_str!(
            "../../../protocol-artifact/watchdog-recovery-v1/fixtures/valid/bootstrap-response.json"
        ),
    ),
    (
        "host-fence-request",
        include_str!(
            "../../../protocol-artifact/watchdog-recovery-v1/fixtures/valid/host-fence-request.json"
        ),
    ),
    (
        "host-fence-response",
        include_str!(
            "../../../protocol-artifact/watchdog-recovery-v1/fixtures/valid/host-fence-response.json"
        ),
    ),
    (
        "lease-acquire-request",
        include_str!(
            "../../../protocol-artifact/watchdog-recovery-v1/fixtures/valid/lease-acquire-request.json"
        ),
    ),
    (
        "lease-acquire-response",
        include_str!(
            "../../../protocol-artifact/watchdog-recovery-v1/fixtures/valid/lease-acquire-response.json"
        ),
    ),
    (
        "lease-renew-request",
        include_str!(
            "../../../protocol-artifact/watchdog-recovery-v1/fixtures/valid/lease-renew-request.json"
        ),
    ),
    (
        "lease-renew-response",
        include_str!(
            "../../../protocol-artifact/watchdog-recovery-v1/fixtures/valid/lease-renew-response.json"
        ),
    ),
    (
        "lease-revoke-request",
        include_str!(
            "../../../protocol-artifact/watchdog-recovery-v1/fixtures/valid/lease-revoke-request.json"
        ),
    ),
    (
        "lease-revoke-response",
        include_str!(
            "../../../protocol-artifact/watchdog-recovery-v1/fixtures/valid/lease-revoke-response.json"
        ),
    ),
    (
        "operation-dispatch-request",
        include_str!(
            "../../../protocol-artifact/watchdog-recovery-v1/fixtures/valid/operation-dispatch-request.json"
        ),
    ),
    (
        "operation-dispatch-response",
        include_str!(
            "../../../protocol-artifact/watchdog-recovery-v1/fixtures/valid/operation-dispatch-response.json"
        ),
    ),
    (
        "operation-intent-request",
        include_str!(
            "../../../protocol-artifact/watchdog-recovery-v1/fixtures/valid/operation-intent-request.json"
        ),
    ),
    (
        "operation-intent-response",
        include_str!(
            "../../../protocol-artifact/watchdog-recovery-v1/fixtures/valid/operation-intent-response.json"
        ),
    ),
    (
        "operation-lookup-request",
        include_str!(
            "../../../protocol-artifact/watchdog-recovery-v1/fixtures/valid/operation-lookup-request.json"
        ),
    ),
    (
        "operation-lookup-response",
        include_str!(
            "../../../protocol-artifact/watchdog-recovery-v1/fixtures/valid/operation-lookup-response.json"
        ),
    ),
    (
        "operation-reconcile-request",
        include_str!(
            "../../../protocol-artifact/watchdog-recovery-v1/fixtures/valid/operation-reconcile-request.json"
        ),
    ),
    (
        "operation-reconcile-response",
        include_str!(
            "../../../protocol-artifact/watchdog-recovery-v1/fixtures/valid/operation-reconcile-response.json"
        ),
    ),
];
const INVALID_FIXTURES: [(&str, &str); 3] = [
    (
        "oversized-action",
        include_str!(
            "../../../protocol-artifact/watchdog-recovery-v1/fixtures/invalid/oversized-action.json"
        ),
    ),
    (
        "stale-contract",
        include_str!(
            "../../../protocol-artifact/watchdog-recovery-v1/fixtures/invalid/stale-contract.json"
        ),
    ),
    (
        "unknown-field",
        include_str!(
            "../../../protocol-artifact/watchdog-recovery-v1/fixtures/invalid/unknown-field.json"
        ),
    ),
];

#[test]
fn recovery_profile_consumes_the_exact_published_artifact() -> Result<(), String> {
    verify_recovery_artifact().map_err(|error| error.to_string())?;
    let manifest: Value = serde_json::from_str(MANIFEST).map_err(|error| error.to_string())?;
    assert_eq!(manifest["artifact"], RECOVERY_ARTIFACT);
    assert_eq!(manifest["protocol_version"], RECOVERY_PROTOCOL_VERSION);
    assert_eq!(manifest["schema_digest"], RECOVERY_SCHEMA_DIGEST);
    assert_eq!(manifest["schema"], "schema.json");
    assert!(SCHEMA.starts_with(b"{\n  \"$schema\""));
    serde_json::from_str::<Value>(CONFORMANCE).map_err(|error| error.to_string())?;
    serde_json::from_str::<Value>(RCJ_VECTORS).map_err(|error| error.to_string())?;
    Ok(())
}

#[test]
fn recovery_schema_accepts_all_valid_frames_and_rejects_invalid_shapes() -> Result<(), String> {
    let schema: Value = serde_json::from_slice(SCHEMA).map_err(|error| error.to_string())?;
    let validator = options()
        .with_pattern_options(PatternOptions::fancy_regex().size_limit(1_000_000_000))
        .build(&schema)
        .map_err(|error| error.to_string())?;
    for (name, fixture) in VALID_FIXTURES {
        let value: Value = serde_json::from_str(fixture).map_err(|error| error.to_string())?;
        if !validator.is_valid(&value) {
            return Err(format!("valid recovery fixture was rejected: {name}"));
        }
    }
    for (name, fixture) in INVALID_FIXTURES {
        let value: Value = serde_json::from_str(fixture).map_err(|error| error.to_string())?;
        if validator.is_valid(&value) {
            return Err(format!("invalid recovery fixture was accepted: {name}"));
        }
    }
    Ok(())
}
