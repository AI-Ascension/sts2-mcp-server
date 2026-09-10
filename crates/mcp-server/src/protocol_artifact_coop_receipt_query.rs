// SPDX-License-Identifier: MIT

//! Owner-local copy of the proposed neutral retained-receipt query artifact.
//!
//! This profile is intentionally additive and remains unadmitted.  Keeping the
//! bytes and their checksum inventory in the MCP repository makes a stale or
//! silently substituted protocol copy fail before the tool is exposed.

use crate::json::{self, JsonValue};

pub const COOP_RECEIPT_QUERY_PROTOCOL_VERSION: &str = "coop-receipt-query-v1";
pub const COOP_RECEIPT_QUERY_ARTIFACT: &str = "sts2-protocol/coop-receipt-query-v1";
pub const COOP_RECEIPT_QUERY_SCHEMA_SOURCE: &str = "schemas/coop-receipt-query-v1.schema.json";
pub const COOP_RECEIPT_QUERY_GENERATOR: &str = "hand-authored";
pub const COOP_RECEIPT_QUERY_SCHEMA_DIGEST: &str =
    "3e3eaedb93926b26025abb09d8028491e2632896753688c1182c698fed7d3f7c";
pub const COOP_RECEIPT_QUERY_MAX_GENERATION: i64 = 9_007_199_254_740_991;
pub const COOP_RECEIPT_QUERY_MAX_BODY_BYTES: usize = 16 * 1024;

const MANIFEST: &[u8] =
    include_bytes!("../../../protocol-artifact/coop-receipt-query-v1/manifest.json");
const SCHEMA: &[u8] =
    include_bytes!("../../../protocol-artifact/coop-receipt-query-v1/schema.json");
const CHECKSUMS: &str = include_str!("../../../protocol-artifact/coop-receipt-query-v1/SHA256SUMS");
const CONFORMANCE: &[u8] = include_bytes!("../../../conformance/cases/coop-receipt-query-v1.json");

struct ArtifactFile {
    path: &'static str,
    bytes: &'static [u8],
}

const ARTIFACT_FILES: &[ArtifactFile] = &[
    ArtifactFile {
        path: "../../conformance/cases/coop-receipt-query-v1.json",
        bytes: CONFORMANCE,
    },
    ArtifactFile {
        path: "../../schemas/coop-receipt-query-v1.schema.json",
        bytes: include_bytes!("../../../schemas/coop-receipt-query-v1.schema.json"),
    },
    ArtifactFile {
        path: "README.md",
        bytes: include_bytes!("../../../protocol-artifact/coop-receipt-query-v1/README.md"),
    },
    ArtifactFile {
        path: "conformance.json",
        bytes: include_bytes!("../../../protocol-artifact/coop-receipt-query-v1/conformance.json"),
    },
    ArtifactFile {
        path: "fixtures/invalid-request-participant-count.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/coop-receipt-query-v1/fixtures/invalid-request-participant-count.json"
        ),
    },
    ArtifactFile {
        path: "fixtures/invalid-request-unknown-member.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/coop-receipt-query-v1/fixtures/invalid-request-unknown-member.json"
        ),
    },
    ArtifactFile {
        path: "fixtures/invalid-response-fresh-scope.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/coop-receipt-query-v1/fixtures/invalid-response-fresh-scope.json"
        ),
    },
    ArtifactFile {
        path: "fixtures/invalid-response-status-receipt.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/coop-receipt-query-v1/fixtures/invalid-response-status-receipt.json"
        ),
    },
    ArtifactFile {
        path: "golden/receipt-query-request.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/coop-receipt-query-v1/golden/receipt-query-request.json"
        ),
    },
    ArtifactFile {
        path: "golden/receipt-query-response-accepted.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/coop-receipt-query-v1/golden/receipt-query-response-accepted.json"
        ),
    },
    ArtifactFile {
        path: "golden/receipt-query-response-rejected.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/coop-receipt-query-v1/golden/receipt-query-response-rejected.json"
        ),
    },
    ArtifactFile {
        path: "golden/receipt-query-response-settled.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/coop-receipt-query-v1/golden/receipt-query-response-settled.json"
        ),
    },
    ArtifactFile {
        path: "golden/receipt-query-response-unknown.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/coop-receipt-query-v1/golden/receipt-query-response-unknown.json"
        ),
    },
    ArtifactFile {
        path: "manifest.json",
        bytes: MANIFEST,
    },
    ArtifactFile {
        path: "schema.json",
        bytes: SCHEMA,
    },
];

pub fn verify_coop_receipt_query_artifact() -> Result<(), CoopReceiptQueryArtifactError> {
    let manifest = parse(MANIFEST)?;
    let expected_provenance = JsonValue::object([
        (
            String::from("source"),
            JsonValue::string(COOP_RECEIPT_QUERY_SCHEMA_SOURCE),
        ),
        (
            String::from("generator"),
            JsonValue::string(COOP_RECEIPT_QUERY_GENERATOR),
        ),
        (String::from("license"), JsonValue::string("MIT")),
    ]);
    let expected_consumers = JsonValue::Array(Vec::new());
    let expected_prospective = JsonValue::Array(
        [
            "sts2-game-mod",
            "sts2-gateway",
            "sts2-mcp-server",
            "sts2-harness",
        ]
        .into_iter()
        .map(JsonValue::string)
        .collect(),
    );
    if field(&manifest, "artifact") != Some(&JsonValue::string(COOP_RECEIPT_QUERY_ARTIFACT))
        || field(&manifest, "protocol_version")
            != Some(&JsonValue::string(COOP_RECEIPT_QUERY_PROTOCOL_VERSION))
        || field(&manifest, "schema") != Some(&JsonValue::string("schema.json"))
        || field(&manifest, "schema_digest")
            != Some(&JsonValue::string(COOP_RECEIPT_QUERY_SCHEMA_DIGEST))
        || field(&manifest, "status") != Some(&JsonValue::string("proposed_unadmitted"))
        || field(&manifest, "provenance") != Some(&expected_provenance)
        || field(&manifest, "consumers") != Some(&expected_consumers)
        || field(&manifest, "prospective_consumers") != Some(&expected_prospective)
        || field(&manifest, "checksums") != Some(&JsonValue::string("SHA256SUMS"))
    {
        return Err(CoopReceiptQueryArtifactError::ManifestMismatch);
    }
    if field(&parse(SCHEMA)?, "$id") != Some(&JsonValue::string("sts2-coop-receipt-query-v1")) {
        return Err(CoopReceiptQueryArtifactError::SchemaMismatch);
    }
    for file in ARTIFACT_FILES {
        let text = std::str::from_utf8(file.bytes)
            .map_err(|_| CoopReceiptQueryArtifactError::ChecksumMismatch)?;
        if file.path.ends_with(".json") {
            parse(text.as_bytes())?;
        }
    }
    verify_checksums()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CoopReceiptQueryArtifactError {
    ChecksumMismatch,
    InvalidJson,
    ManifestMismatch,
    SchemaMismatch,
}

impl std::fmt::Display for CoopReceiptQueryArtifactError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("copied co-op receipt-query artifact is invalid")
    }
}

impl std::error::Error for CoopReceiptQueryArtifactError {}

fn field<'a>(value: &'a JsonValue, key: &str) -> Option<&'a JsonValue> {
    value.as_object()?.get(key)
}

fn parse(bytes: &[u8]) -> Result<JsonValue, CoopReceiptQueryArtifactError> {
    let text =
        std::str::from_utf8(bytes).map_err(|_| CoopReceiptQueryArtifactError::InvalidJson)?;
    json::parse(text).map_err(|_| CoopReceiptQueryArtifactError::InvalidJson)
}

fn verify_checksums() -> Result<(), CoopReceiptQueryArtifactError> {
    let mut verified = Vec::new();
    for line in CHECKSUMS.lines() {
        let (expected, path) = line
            .split_once("  ")
            .ok_or(CoopReceiptQueryArtifactError::ChecksumMismatch)?;
        if expected.len() != 64
            || !expected
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            || verified.contains(&path)
        {
            return Err(CoopReceiptQueryArtifactError::ChecksumMismatch);
        }
        let file = ARTIFACT_FILES
            .iter()
            .find(|file| file.path == path)
            .ok_or(CoopReceiptQueryArtifactError::ChecksumMismatch)?;
        if crate::protocol_artifact_hash::sha256_hex(file.bytes) != expected {
            return Err(CoopReceiptQueryArtifactError::ChecksumMismatch);
        }
        verified.push(path);
    }
    if verified.len() != ARTIFACT_FILES.len()
        || ARTIFACT_FILES
            .iter()
            .any(|file| !verified.contains(&file.path))
    {
        return Err(CoopReceiptQueryArtifactError::ChecksumMismatch);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::verify_coop_receipt_query_artifact;

    #[test]
    fn copied_neutral_artifact_and_checksum_inventory_are_frozen() {
        assert_eq!(verify_coop_receipt_query_artifact(), Ok(()));
    }
}
