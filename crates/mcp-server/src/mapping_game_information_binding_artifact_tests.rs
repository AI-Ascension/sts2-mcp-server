// SPDX-License-Identifier: MIT

use super::vector_data::{INVALID_FIXTURES, VALID_GOLDENS, descriptor_ids, indexed_ids, string};
use super::*;

#[test]
fn copied_lbr_artifact_and_vector_membership_are_pinned() -> Result<(), String> {
    let index = parse_json(include_str!(
        "../../../protocol-artifact/game-information-lookup-binding-v1/conformance.json"
    ))?;
    let case = parse_json(include_str!(
        "../../../protocol-artifact/game-information-lookup-binding-v1/conformance/cases/game-information-lookup-binding-v1.json"
    ))?;
    let expected_valid = [
        "LBR-VALID-DISCOVERY-INITIAL",
        "LBR-VALID-DISCOVERY-OBSERVED",
        "LBR-VALID-REOBSERVE-REQUIRED",
        "LBR-VALID-REOBSERVE-EXHAUSTED",
        "LBR-VALID-REOBSERVE-UNAVAILABLE",
        "LBR-VALID-REOBSERVED",
        "LBR-VALID-BINDING-IDENTITY-INPUT",
        "LBR-VALID-BINDING-IDENTITY-EPOCH",
    ];
    let expected_invalid = [
        "LBR-INVALID-WRONG-SCOPE",
        "LBR-INVALID-WRONG-INSTANCE",
        "LBR-INVALID-FORGED-BINDING-ID",
        "LBR-INVALID-MIXED-BINDING",
        "LBR-INVALID-STALE-OBSERVATION",
        "LBR-INVALID-MISSING-CAPABILITY",
        "LBR-INVALID-SUPERSEDES-MISMATCH",
        "LBR-INVALID-UNKNOWN-VERSION",
        "LBR-INVALID-REOBSERVE-UNAVAILABLE-SHAPE",
        "LBR-INVALID-EXHAUSTED-WITH-OBSERVATION",
        "LBR-INVALID-STATE-SHAPE",
        "LBR-INVALID-DUPLICATE-KEY",
    ];
    if indexed_ids(&index, "valid_vectors")? != expected_valid.map(String::from)
        || indexed_ids(&index, "invalid_vectors")? != expected_invalid.map(String::from)
        || descriptor_ids(&case, "valid_vectors")? != expected_valid.map(String::from)
        || descriptor_ids(&case, "invalid_vectors")? != expected_invalid.map(String::from)
    {
        return Err(String::from(
            "copied LBR conformance vector membership changed",
        ));
    }
    for (expected_id, source) in INVALID_FIXTURES {
        let vector = parse_json(source)?;
        if string(&vector, "id")? != expected_id {
            return Err(format!("LBR vector identity changed: {expected_id}"));
        }
    }
    for (expected_id, source) in VALID_GOLDENS {
        let body = parse_json(source)?;
        let expected_kind = if expected_id == "LBR-VALID-DISCOVERY-INITIAL" {
            "lookup_binding_discovery_response"
        } else if expected_id == "LBR-VALID-REOBSERVE-UNAVAILABLE" {
            "error_response"
        } else {
            "lookup_binding_observation_response"
        };
        if string(&body, "kind")? != expected_kind {
            return Err(format!("LBR golden kind changed: {expected_id}"));
        }
    }
    Ok(())
}
