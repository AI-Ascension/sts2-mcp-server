// SPDX-License-Identifier: MIT

use super::{
    ExactRestorePhase, ExactRestoreTransportOwner, validate_exact_restore_request,
    validate_exact_restore_response,
};
use crate::json::JsonValue;
use crate::{EXACT_RESTORE_GATEWAY_SCHEMA_DIGEST, parse_json, verify_exact_restore_artifact};

const FRAMES: &str =
    include_str!("../../../../protocol-artifact/exact-restore-v1/golden/frames.json");
const UNKNOWN_RESPONSE: &str = include_str!(
    "../../../../protocol-artifact/exact-restore-gateway-v1/golden/commit-unknown-response.json"
);
const BEGIN_REQUEST_WRAPPER: &str = include_str!(
    "../../../../protocol-artifact/exact-restore-gateway-v1/golden/begin-request.json"
);
const SWAPPED_DIRECTIONS: &str =
    include_str!("../../../../protocol-artifact/exact-restore-gateway-v1/golden/rejections.json");

fn fixtures() -> Result<Vec<JsonValue>, String> {
    parse_json(FRAMES)?
        .as_object()
        .and_then(|root| root.get("frames"))
        .and_then(JsonValue::as_array)
        .cloned()
        .ok_or_else(|| String::from("exact-restore golden frames are missing"))
}

fn owner() -> ExactRestoreTransportOwner {
    ExactRestoreTransportOwner {
        instance_id: String::from("02ab8278-c166-4557-bd6d-8f7575484a55"),
        session_id: String::from("session-example"),
        lease_id: String::from("c0f5a147-1ba1-4a1c-9a25-1b46935403ef"),
        lease_epoch: 8,
    }
}

fn wrapper(kind: &str, frame: JsonValue) -> JsonValue {
    let message_id = frame
        .as_object()
        .and_then(|value| value.get("message_id"))
        .cloned()
        .unwrap_or(JsonValue::Null);
    let correlation_id = frame
        .as_object()
        .and_then(|value| value.get("correlation_id"))
        .cloned()
        .unwrap_or(JsonValue::Null);
    let role = if kind == "exact_restore_request" {
        "harness"
    } else {
        "gateway"
    };
    JsonValue::object([
        (
            String::from("contract"),
            JsonValue::string("sts2-exact-restore-gateway-v1"),
        ),
        (
            String::from("schema_digest"),
            JsonValue::string(EXACT_RESTORE_GATEWAY_SCHEMA_DIGEST),
        ),
        (String::from("message_id"), message_id),
        (String::from("correlation_id"), correlation_id),
        (
            String::from("actor"),
            JsonValue::object([
                (String::from("principal_id"), JsonValue::string("harness")),
                (String::from("role"), JsonValue::string(role)),
            ]),
        ),
        (
            String::from("auth"),
            JsonValue::object([
                (String::from("principal_id"), JsonValue::string("harness")),
                (
                    String::from("capability"),
                    JsonValue::string("exact_restore"),
                ),
                (String::from("proof"), JsonValue::Null),
            ]),
        ),
        (String::from("kind"), JsonValue::string(kind)),
        (
            String::from("payload"),
            JsonValue::object([(String::from("frame"), frame)]),
        ),
    ])
}

fn frame_from_wrapper(value: &JsonValue) -> Result<JsonValue, String> {
    value
        .as_object()
        .and_then(|root| root.get("payload"))
        .and_then(JsonValue::as_object)
        .and_then(|payload| payload.get("frame"))
        .cloned()
        .ok_or_else(|| String::from("wrapped frame is missing"))
}

fn phase_for_index(index: usize) -> ExactRestorePhase {
    match index {
        0 => ExactRestorePhase::Begin,
        2 => ExactRestorePhase::PutChunk,
        3 => ExactRestorePhase::FinishBlob,
        4 => ExactRestorePhase::Commit,
        6 | 8 => ExactRestorePhase::Lookup,
        _ => unreachable!("fixture index must name a request"),
    }
}

#[test]
fn canonical_artifacts_and_five_phase_request_response_pairs_validate()
-> Result<(), Box<dyn std::error::Error>> {
    verify_exact_restore_artifact()?;
    let frames = fixtures()?;
    for (request_index, response_index) in [(0, 12), (2, 14), (3, 15), (4, 16), (6, 18), (8, 20)] {
        let phase = phase_for_index(request_index);
        let request_wrapper = wrapper("exact_restore_request", frames[request_index].clone());
        let request =
            validate_exact_restore_request(&request_wrapper, phase.tool(), "harness", &owner())?;
        let response_wrapper = wrapper("exact_restore_response", frames[response_index].clone());
        let response = validate_exact_restore_response(&response_wrapper, &request, "harness")?;
        assert_eq!(
            response
                .frame
                .as_object()
                .and_then(|frame| frame.get("kind")),
            Some(&JsonValue::string(phase.response_kind()))
        );
    }
    Ok(())
}

#[test]
fn unknown_commit_result_is_preserved_and_forces_lookup_only()
-> Result<(), Box<dyn std::error::Error>> {
    let frames = fixtures()?;
    let request_wrapper = wrapper("exact_restore_request", frames[5].clone());
    let request = validate_exact_restore_request(
        &request_wrapper,
        crate::EXACT_RESTORE_COMMIT_TOOL,
        "harness",
        &owner(),
    )?;
    let response_wrapper = parse_json(UNKNOWN_RESPONSE)?;
    let response = validate_exact_restore_response(&response_wrapper, &request, "harness")?;
    assert!(response.may_have_started);
    assert!(response.lookup_only);
    assert_eq!(
        super::string_member(&response.frame, "kind"),
        Some("exact_restore_commit_response")
    );
    assert_eq!(
        super::string_member(
            response
                .frame
                .as_object()
                .and_then(|frame| frame.get("payload"))
                .ok_or("response payload missing")?,
            "state"
        ),
        Some("UNKNOWN")
    );
    Ok(())
}

#[test]
fn rejects_foreign_lease_correlation_schema_and_oversize_before_forward()
-> Result<(), Box<dyn std::error::Error>> {
    let parsed: serde_json::Value = serde_json::from_str(BEGIN_REQUEST_WRAPPER)?;
    let mut wrapped = parse_json(&parsed.to_string())?;
    let frame = frame_from_wrapper(&wrapped)?;

    let mut foreign_owner = frame.clone();
    set_nested_string(
        &mut foreign_owner,
        &["payload", "expected_owner", "lease_id"],
        "different-lease",
    );
    let foreign = wrapper("exact_restore_request", foreign_owner);
    assert!(
        validate_exact_restore_request(
            &foreign,
            crate::EXACT_RESTORE_BEGIN_TOOL,
            "harness",
            &owner()
        )
        .is_err()
    );

    set_string(
        &mut wrapped,
        "correlation_id",
        "00000000-0000-4000-8000-000000000000",
    );
    assert!(
        validate_exact_restore_request(
            &wrapped,
            crate::EXACT_RESTORE_BEGIN_TOOL,
            "harness",
            &owner()
        )
        .is_err()
    );

    let mut wrong_schema = frame.clone();
    set_string(
        &mut wrong_schema,
        "schema_digest",
        "0000000000000000000000000000000000000000000000000000000000000000",
    );
    assert!(
        validate_exact_restore_request(
            &wrapper("exact_restore_request", wrong_schema),
            crate::EXACT_RESTORE_BEGIN_TOOL,
            "harness",
            &owner()
        )
        .is_err()
    );

    let mut oversized = wrapped;
    if let JsonValue::Object(object) = &mut oversized {
        object.insert(
            String::from("untrusted"),
            JsonValue::string("x".repeat(16_384)),
        );
    }
    assert!(
        validate_exact_restore_request(
            &oversized,
            crate::EXACT_RESTORE_BEGIN_TOOL,
            "harness",
            &owner()
        )
        .is_err()
    );
    let mut unknown = parse_json(BEGIN_REQUEST_WRAPPER)?;
    if let JsonValue::Object(object) = &mut unknown {
        object.insert(String::from("extra"), JsonValue::Bool(true));
    }
    assert!(
        validate_exact_restore_request(
            &unknown,
            crate::EXACT_RESTORE_BEGIN_TOOL,
            "harness",
            &owner()
        )
        .is_err()
    );
    Ok(())
}

fn set_string(value: &mut JsonValue, field: &str, replacement: &str) {
    if let JsonValue::Object(object) = value {
        object.insert(String::from(field), JsonValue::string(replacement));
    }
}

fn set_nested_string(value: &mut JsonValue, fields: &[&str], replacement: &str) {
    let Some((field, rest)) = fields.split_first() else {
        return;
    };
    let JsonValue::Object(object) = value else {
        return;
    };
    let Some(child) = object.get_mut(*field) else {
        return;
    };
    if rest.is_empty() {
        *child = JsonValue::string(replacement);
    } else {
        set_nested_string(child, rest, replacement);
    }
}

fn base64_encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let first = chunk[0];
        let second = *chunk.get(1).unwrap_or(&0);
        let third = *chunk.get(2).unwrap_or(&0);
        output.push(char::from(ALPHABET[usize::from(first >> 2)]));
        output.push(char::from(
            ALPHABET[usize::from(((first & 0x03) << 4) | (second >> 4))],
        ));
        if chunk.len() > 1 {
            output.push(char::from(
                ALPHABET[usize::from(((second & 0x0f) << 2) | (third >> 6))],
            ));
        } else {
            output.push('=');
        }
        if chunk.len() > 2 {
            output.push(char::from(ALPHABET[usize::from(third & 0x3f)]));
        } else {
            output.push('=');
        }
    }
    output
}

#[path = "tests_rejections.rs"]
mod rejections;
