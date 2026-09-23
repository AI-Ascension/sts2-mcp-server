// SPDX-License-Identifier: MIT
#![allow(clippy::unwrap_used)]

use super::{
    NEGOTIATED_CAPABILITIES_MAX_BYTES, NEGOTIATED_CAPABILITIES_SCHEMA_DIGEST,
    NEGOTIATED_CAPABILITIES_SOURCE_COMMIT, NegotiatedCapabilitiesArtifactError,
    validate_negotiated_capabilities_snapshot, verify_negotiated_capabilities_artifact,
};

#[test]
fn verifies_the_frozen_gateway_artifact_and_schema_pin() {
    verify_negotiated_capabilities_artifact().unwrap();
    assert_eq!(
        NEGOTIATED_CAPABILITIES_SOURCE_COMMIT,
        "e15248cd41f89188706a8a19e974f97bf5880a9f"
    );
    assert_eq!(
        NEGOTIATED_CAPABILITIES_SCHEMA_DIGEST,
        "447c066568897ef720c07ade7963c97ba037645eccb9a8b71a0724d0d22fd299"
    );
}

#[test]
fn rejects_duplicate_keys_and_oversized_capability_documents() {
    let duplicate = r#"{"schema_version":"first","schema_version":"second"}"#;
    assert_eq!(
        validate_negotiated_capabilities_snapshot(duplicate),
        Err(NegotiatedCapabilitiesArtifactError::InvalidJson)
    );
    let oversized = format!("{{}}{}", " ".repeat(NEGOTIATED_CAPABILITIES_MAX_BYTES));
    assert_eq!(
        validate_negotiated_capabilities_snapshot(&oversized),
        Err(NegotiatedCapabilitiesArtifactError::SnapshotTooLarge)
    );
}
