// SPDX-License-Identifier: MIT

use super::*;

#[test]
fn rejects_direction_swaps_unknown_fields_duplicate_keys_and_oversized_decoded_chunk()
-> Result<(), Box<dyn std::error::Error>> {
    let frames = fixtures()?;
    let response_wrapper = wrapper("exact_restore_request", frames[12].clone());
    assert!(
        validate_exact_restore_request(
            &response_wrapper,
            crate::EXACT_RESTORE_BEGIN_TOOL,
            "harness",
            &owner()
        )
        .is_err()
    );
    let request_wrapper = wrapper("exact_restore_response", frames[0].clone());
    assert!(crate::exact_restore::schema::validate_gateway(&request_wrapper).is_err());
    let swaps = parse_json(SWAPPED_DIRECTIONS)?;
    assert_eq!(
        swaps
            .as_object()
            .and_then(|value| value.get("invalid"))
            .and_then(JsonValue::as_array)
            .map(Vec::len),
        Some(2)
    );
    assert!(crate::parse_json(r#"{"key":1,"key":2}"#).is_err());

    let mut chunk = frames[2].clone();
    let oversized_data = "a".repeat(8193);
    let encoded = base64_encode(oversized_data.as_bytes());
    set_nested_string(&mut chunk, &["payload", "data_base64"], &encoded);
    let envelope = wrapper("exact_restore_request", chunk);
    assert!(
        validate_exact_restore_request(
            &envelope,
            crate::EXACT_RESTORE_PUT_CHUNK_TOOL,
            "harness",
            &owner()
        )
        .is_err()
    );

    let mut noncanonical_chunk = frames[2].clone();
    set_nested_string(
        &mut noncanonical_chunk,
        &["payload", "data_base64"],
        "aGVsbG9=",
    );
    let envelope = wrapper("exact_restore_request", noncanonical_chunk);
    assert!(
        validate_exact_restore_request(
            &envelope,
            crate::EXACT_RESTORE_PUT_CHUNK_TOOL,
            "harness",
            &owner()
        )
        .is_err()
    );
    Ok(())
}

#[test]
fn receipt_must_match_commit_checkpoint_and_state_bindings()
-> Result<(), Box<dyn std::error::Error>> {
    let frames = fixtures()?;
    let request_wrapper = wrapper("exact_restore_request", frames[4].clone());
    let request = validate_exact_restore_request(
        &request_wrapper,
        crate::EXACT_RESTORE_COMMIT_TOOL,
        "harness",
        &owner(),
    )?;

    let mut matching = frames[21].clone();
    set_string(&mut matching, "kind", "exact_restore_commit_response");
    set_nested_string(
        &mut matching,
        &["payload", "request_digest"],
        &request.request_digest,
    );
    set_string(&mut matching, "correlation_id", &request.message_id);
    let matching_wrapper = wrapper("exact_restore_response", matching);
    validate_exact_restore_response(&matching_wrapper, &request, "harness")?;

    let mut mismatched = frames[21].clone();
    set_string(&mut mismatched, "kind", "exact_restore_commit_response");
    set_nested_string(
        &mut mismatched,
        &["payload", "request_digest"],
        &request.request_digest,
    );
    set_string(&mut mismatched, "correlation_id", &request.message_id);
    set_nested_string(
        &mut mismatched,
        &["payload", "receipt", "checkpoint_id"],
        &format!("asc-checkpoint:v1:sha256:{}1", "0".repeat(63)),
    );
    let receipt = mismatched
        .as_object()
        .and_then(|root| root.get("payload"))
        .and_then(JsonValue::as_object)
        .and_then(|payload| payload.get("receipt"))
        .and_then(JsonValue::as_object)
        .ok_or("receipt fixture is missing")?;
    let mut unsigned = receipt.clone();
    unsigned.remove("receipt_digest");
    let receipt_digest = format!(
        "sha256:{}",
        crate::protocol_artifact_hash::sha256_hex(JsonValue::Object(unsigned).to_json().as_bytes())
    );
    set_nested_string(
        &mut mismatched,
        &["payload", "receipt", "receipt_digest"],
        &receipt_digest,
    );
    let mismatched_wrapper = wrapper("exact_restore_response", mismatched);
    assert!(validate_exact_restore_response(&mismatched_wrapper, &request, "harness").is_err());
    Ok(())
}
