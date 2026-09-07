// SPDX-License-Identifier: MIT

//! Release metadata for the additive watchdog recovery sideband.
//!
//! The bytes are copied from the approved protocol artifact.  This module is
//! deliberately metadata-only: it does not import a gateway implementation or
//! create recovery authority.

use crate::json::{self, JsonValue};

use crate::protocol_artifact_runtime_v2::sha256_hex_for_recovery as sha256_hex;

pub const RECOVERY_PROTOCOL_VERSION: &str = "watchdog-recovery-v1";
pub const RECOVERY_CONTRACT: &str = "watchdog-recovery-v1";
pub const RECOVERY_ARTIFACT: &str = "sts2-protocol/watchdog-recovery-v1";
pub const RECOVERY_SCHEMA_SOURCE: &str = "schemas/watchdog-recovery-v1.schema.json";
pub const RECOVERY_SCHEMA_DIGEST: &str =
    "fb934d3157485aaf6e13e6ebbb213ec8a14c7fc6f5eeebc06b7a22c1f0009217";
pub const RECOVERY_RUNTIME_V3_SCHEMA_DIGEST: &str =
    "8e99cea36b7ede97532348fd8efe302ca79260895265a7bf14ddf7e006d8ff63";
pub const RECOVERY_MAX_FRAME_BYTES: usize = 262_144;
pub const RECOVERY_MAX_ACTION_BYTES: usize = 65_536;
pub const RECOVERY_MAX_AUTH_PROOF_BYTES: usize = 512;
pub const RECOVERY_MAX_WIRE_INTEGER: i64 = 9_007_199_254_740_991;
pub const RECOVERY_MAX_RESPONSE_BYTES: usize = 256 * 1024;

const MANIFEST: &str =
    include_str!("../../../protocol-artifact/watchdog-recovery-v1/manifest.json");
const SCHEMA: &[u8] = include_bytes!("../../../protocol-artifact/watchdog-recovery-v1/schema.json");
const CHECKSUMS: &str = include_str!("../../../protocol-artifact/watchdog-recovery-v1/SHA256SUMS");
const CONFORMANCE: &str =
    include_str!("../../../protocol-artifact/watchdog-recovery-v1/conformance.json");
const RCJ_VECTORS: &str =
    include_str!("../../../protocol-artifact/watchdog-recovery-v1/rcj-vectors.json");
const README: &[u8] = include_bytes!("../../../protocol-artifact/watchdog-recovery-v1/README.md");
const FIXTURE_README: &[u8] =
    include_bytes!("../../../protocol-artifact/watchdog-recovery-v1/fixtures/README.md");

const SOURCE_SCHEMA_PATH: &str = "../../schemas/watchdog-recovery-v1.schema.json";
const SOURCE_CONFORMANCE_PATH: &str = "../../conformance/cases/watchdog-recovery-v1.json";

const FIXTURES: &[(&str, &[u8])] = &[
    (
        "fixtures/valid/bootstrap-request.json",
        include_bytes!(
            "../../../protocol-artifact/watchdog-recovery-v1/fixtures/valid/bootstrap-request.json"
        ),
    ),
    (
        "fixtures/valid/bootstrap-response.json",
        include_bytes!(
            "../../../protocol-artifact/watchdog-recovery-v1/fixtures/valid/bootstrap-response.json"
        ),
    ),
    (
        "fixtures/valid/host-fence-request.json",
        include_bytes!(
            "../../../protocol-artifact/watchdog-recovery-v1/fixtures/valid/host-fence-request.json"
        ),
    ),
    (
        "fixtures/valid/host-fence-response.json",
        include_bytes!(
            "../../../protocol-artifact/watchdog-recovery-v1/fixtures/valid/host-fence-response.json"
        ),
    ),
    (
        "fixtures/valid/lease-acquire-request.json",
        include_bytes!(
            "../../../protocol-artifact/watchdog-recovery-v1/fixtures/valid/lease-acquire-request.json"
        ),
    ),
    (
        "fixtures/valid/lease-acquire-response.json",
        include_bytes!(
            "../../../protocol-artifact/watchdog-recovery-v1/fixtures/valid/lease-acquire-response.json"
        ),
    ),
    (
        "fixtures/valid/lease-renew-request.json",
        include_bytes!(
            "../../../protocol-artifact/watchdog-recovery-v1/fixtures/valid/lease-renew-request.json"
        ),
    ),
    (
        "fixtures/valid/lease-renew-response.json",
        include_bytes!(
            "../../../protocol-artifact/watchdog-recovery-v1/fixtures/valid/lease-renew-response.json"
        ),
    ),
    (
        "fixtures/valid/lease-revoke-request.json",
        include_bytes!(
            "../../../protocol-artifact/watchdog-recovery-v1/fixtures/valid/lease-revoke-request.json"
        ),
    ),
    (
        "fixtures/valid/lease-revoke-response.json",
        include_bytes!(
            "../../../protocol-artifact/watchdog-recovery-v1/fixtures/valid/lease-revoke-response.json"
        ),
    ),
    (
        "fixtures/valid/operation-intent-request.json",
        include_bytes!(
            "../../../protocol-artifact/watchdog-recovery-v1/fixtures/valid/operation-intent-request.json"
        ),
    ),
    (
        "fixtures/valid/operation-intent-response.json",
        include_bytes!(
            "../../../protocol-artifact/watchdog-recovery-v1/fixtures/valid/operation-intent-response.json"
        ),
    ),
    (
        "fixtures/valid/operation-dispatch-request.json",
        include_bytes!(
            "../../../protocol-artifact/watchdog-recovery-v1/fixtures/valid/operation-dispatch-request.json"
        ),
    ),
    (
        "fixtures/valid/operation-dispatch-response.json",
        include_bytes!(
            "../../../protocol-artifact/watchdog-recovery-v1/fixtures/valid/operation-dispatch-response.json"
        ),
    ),
    (
        "fixtures/valid/operation-lookup-request.json",
        include_bytes!(
            "../../../protocol-artifact/watchdog-recovery-v1/fixtures/valid/operation-lookup-request.json"
        ),
    ),
    (
        "fixtures/valid/operation-lookup-response.json",
        include_bytes!(
            "../../../protocol-artifact/watchdog-recovery-v1/fixtures/valid/operation-lookup-response.json"
        ),
    ),
    (
        "fixtures/valid/operation-reconcile-request.json",
        include_bytes!(
            "../../../protocol-artifact/watchdog-recovery-v1/fixtures/valid/operation-reconcile-request.json"
        ),
    ),
    (
        "fixtures/valid/operation-reconcile-response.json",
        include_bytes!(
            "../../../protocol-artifact/watchdog-recovery-v1/fixtures/valid/operation-reconcile-response.json"
        ),
    ),
    (
        "fixtures/invalid/oversized-action.json",
        include_bytes!(
            "../../../protocol-artifact/watchdog-recovery-v1/fixtures/invalid/oversized-action.json"
        ),
    ),
    (
        "fixtures/invalid/stale-contract.json",
        include_bytes!(
            "../../../protocol-artifact/watchdog-recovery-v1/fixtures/invalid/stale-contract.json"
        ),
    ),
    (
        "fixtures/invalid/unknown-field.json",
        include_bytes!(
            "../../../protocol-artifact/watchdog-recovery-v1/fixtures/invalid/unknown-field.json"
        ),
    ),
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecoveryArtifactError {
    InvalidJson,
    ManifestMismatch,
    SchemaMismatch,
    ChecksumMismatch,
}

impl std::fmt::Display for RecoveryArtifactError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("copied watchdog recovery artifact is invalid")
    }
}

impl std::error::Error for RecoveryArtifactError {}

/// Verify the checked-in release-like copy before using its contract identity.
pub fn verify_recovery_artifact() -> Result<(), RecoveryArtifactError> {
    let manifest = parse(MANIFEST)?;
    if field(&manifest, "artifact") != Some(&JsonValue::string(RECOVERY_ARTIFACT))
        || field(&manifest, "contract") != Some(&JsonValue::string(RECOVERY_CONTRACT))
        || field(&manifest, "protocol_version")
            != Some(&JsonValue::string(RECOVERY_PROTOCOL_VERSION))
        || field(&manifest, "schema") != Some(&JsonValue::string("schema.json"))
        || field(&manifest, "schema_digest") != Some(&JsonValue::string(RECOVERY_SCHEMA_DIGEST))
    {
        return Err(RecoveryArtifactError::ManifestMismatch);
    }
    if sha256_hex(SCHEMA) != RECOVERY_SCHEMA_DIGEST || !SCHEMA.starts_with(b"{\n  \"$schema\"") {
        return Err(RecoveryArtifactError::SchemaMismatch);
    }
    for (path, bytes) in FIXTURES {
        let text = std::str::from_utf8(bytes).map_err(|_| RecoveryArtifactError::InvalidJson)?;
        parse(text)?;
        if CHECKSUMS
            .lines()
            .all(|line| !line.ends_with(&format!("  {path}")))
        {
            return Err(RecoveryArtifactError::ChecksumMismatch);
        }
    }
    parse(CONFORMANCE)?;
    parse(RCJ_VECTORS)?;
    verify_checksums()
}

fn verify_checksums() -> Result<(), RecoveryArtifactError> {
    let entries: Vec<(&str, &[u8])> = [
        (SOURCE_CONFORMANCE_PATH, CONFORMANCE.as_bytes()),
        (SOURCE_SCHEMA_PATH, SCHEMA),
        ("manifest.json", MANIFEST.as_bytes()),
        ("schema.json", SCHEMA),
        ("conformance.json", CONFORMANCE.as_bytes()),
        ("rcj-vectors.json", RCJ_VECTORS.as_bytes()),
        ("README.md", README),
        ("fixtures/README.md", FIXTURE_README),
    ]
    .into_iter()
    .chain(FIXTURES.iter().map(|(path, bytes)| (*path, *bytes)))
    .collect();
    if CHECKSUMS.lines().count() != entries.len() {
        return Err(RecoveryArtifactError::ChecksumMismatch);
    }
    let mut seen = std::collections::BTreeSet::new();
    for line in CHECKSUMS.lines() {
        let (digest, path) = line
            .split_once("  ")
            .ok_or(RecoveryArtifactError::ChecksumMismatch)?;
        if digest.len() != 64
            || !digest
                .bytes()
                .all(|byte: u8| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
            || !seen.insert(path)
        {
            return Err(RecoveryArtifactError::ChecksumMismatch);
        }
        let (_, bytes) = entries
            .iter()
            .find(|(candidate, _)| *candidate == path)
            .ok_or(RecoveryArtifactError::ChecksumMismatch)?;
        if sha256_hex(bytes) != digest {
            return Err(RecoveryArtifactError::ChecksumMismatch);
        }
    }
    if seen.len() != entries.len() {
        return Err(RecoveryArtifactError::ChecksumMismatch);
    }
    Ok(())
}

fn field<'a>(value: &'a JsonValue, key: &str) -> Option<&'a JsonValue> {
    value.as_object()?.get(key)
}

fn parse(text: &str) -> Result<JsonValue, RecoveryArtifactError> {
    json::parse(text).map_err(|_| RecoveryArtifactError::InvalidJson)
}
