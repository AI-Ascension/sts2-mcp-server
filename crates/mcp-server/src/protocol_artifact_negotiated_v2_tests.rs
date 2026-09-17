// SPDX-License-Identifier: MIT
#![allow(clippy::unwrap_used)]

use super::{
    NEGOTIATED_CAPABILITIES_V2_MAX_BYTES, NEGOTIATED_CAPABILITIES_V2_SCHEMA_DIGEST,
    NEGOTIATED_CAPABILITIES_V2_SOURCE_COMMIT, NegotiatedCapabilitiesV2ArtifactError,
    validate_negotiated_capabilities_v2_snapshot, verify_negotiated_capabilities_v2_artifact,
};

#[test]
fn verifies_the_frozen_gateway_artifact_and_schema_pin() {
    verify_negotiated_capabilities_v2_artifact().unwrap();
    assert_eq!(
        NEGOTIATED_CAPABILITIES_V2_SOURCE_COMMIT,
        "bfe28e455de48d6d9db466bbcf6062ab5d85e9af"
    );
    assert_eq!(
        NEGOTIATED_CAPABILITIES_V2_SCHEMA_DIGEST,
        "848ecaf673645a42e09357d3014c812a826d7938f4b723beeef19a81b3b8341a"
    );
}

#[test]
fn rejects_duplicate_keys_and_oversized_capability_documents() {
    let duplicate = r#"{"schema_version":"first","schema_version":"second"}"#;
    assert_eq!(
        validate_negotiated_capabilities_v2_snapshot(duplicate),
        Err(NegotiatedCapabilitiesV2ArtifactError::InvalidJson)
    );
    let oversized = format!("{{}}{}", " ".repeat(NEGOTIATED_CAPABILITIES_V2_MAX_BYTES));
    assert_eq!(
        validate_negotiated_capabilities_v2_snapshot(&oversized),
        Err(NegotiatedCapabilitiesV2ArtifactError::SnapshotTooLarge)
    );
}
