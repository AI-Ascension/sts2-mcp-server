// SPDX-License-Identifier: MIT
// Protocol vectors vendored for the whole-manifest read.
#![allow(clippy::expect_used, clippy::panic)]

use super::content_manifest::{manifest_golden, manifest_server, member};
use super::*;
use sts2_mcp_server::{
    CONTENT_MANIFEST_MAX_MESSAGE_BYTES, CONTENT_MANIFEST_SCHEMA_DIGEST,
    content_manifest_conformance_paths,
};

/// The protocol's own case file, vendored byte-for-byte from `sts2-protocol`.
const CASE_PATH: &str = "../../conformance/cases/game-information-content-manifest-v1.json";
const CASE: &str =
    include_str!("../../../../conformance/cases/game-information-content-manifest-v1.json");

/// Every vector the case declares, vendored byte-for-byte beside it.
const VECTORS: [(&str, &str); 8] = [
    (
        "../../conformance/fixtures/game-information-content-manifest-v1/valid/canonical-manifest-response.json",
        include_str!(
            "../../../../conformance/fixtures/game-information-content-manifest-v1/valid/canonical-manifest-response.json"
        ),
    ),
    (
        "../../conformance/fixtures/game-information-content-manifest-v1/valid/access-denied-error-response.json",
        include_str!(
            "../../../../conformance/fixtures/game-information-content-manifest-v1/valid/access-denied-error-response.json"
        ),
    ),
    (
        "../../conformance/fixtures/game-information-content-manifest-v1/invalid/error-with-null-payload.json",
        include_str!(
            "../../../../conformance/fixtures/game-information-content-manifest-v1/invalid/error-with-null-payload.json"
        ),
    ),
    (
        "../../conformance/fixtures/game-information-content-manifest-v1/invalid/mismatched-error-code-reason.json",
        include_str!(
            "../../../../conformance/fixtures/game-information-content-manifest-v1/invalid/mismatched-error-code-reason.json"
        ),
    ),
    (
        "../../conformance/fixtures/game-information-content-manifest-v1/invalid/raw-error-reason.json",
        include_str!(
            "../../../../conformance/fixtures/game-information-content-manifest-v1/invalid/raw-error-reason.json"
        ),
    ),
    (
        "../../conformance/fixtures/game-information-content-manifest-v1/invalid/success-with-null-manifest.json",
        include_str!(
            "../../../../conformance/fixtures/game-information-content-manifest-v1/invalid/success-with-null-manifest.json"
        ),
    ),
    (
        "../../conformance/fixtures/game-information-content-manifest-v1/invalid/unknown-member.json",
        include_str!(
            "../../../../conformance/fixtures/game-information-content-manifest-v1/invalid/unknown-member.json"
        ),
    ),
    (
        "../../conformance/fixtures/game-information-content-manifest-v1/invalid/unsupported-schema-digest.json",
        include_str!(
            "../../../../conformance/fixtures/game-information-content-manifest-v1/invalid/unsupported-schema-digest.json"
        ),
    ),
];

fn vector(path: &str) -> &'static str {
    VECTORS
        .iter()
        .find(|(name, _)| *name == path)
        .map(|(_, bytes)| *bytes)
        .unwrap_or_else(|| panic!("unvendored content-manifest vector {path}"))
}

fn case_member<'a>(case: &'a JsonValue, key: &str) -> &'a JsonValue {
    case.as_object()
        .and_then(|object| object.get(key))
        .unwrap_or_else(|| panic!("content-manifest case member {key}"))
}

fn case_paths(case: &JsonValue, key: &str) -> Vec<String> {
    case_member(case, key)
        .as_array()
        .unwrap_or_else(|| panic!("content-manifest case {key} is not an array"))
        .iter()
        .map(|path| {
            String::from(
                path.as_string()
                    .unwrap_or_else(|| panic!("content-manifest case {key} holds a non-path")),
            )
        })
        .collect()
}

fn token<'a>(value: &'a JsonValue, key: &str) -> Option<&'a str> {
    value.as_object()?.get(key)?.as_string()
}

/// The refusal a vector relays, or `None` when the vector is a complete catalog.
fn refusal_code(body: &JsonValue) -> Option<&str> {
    if token(body, "kind") != Some("error_response") {
        return None;
    }
    body.as_object()?
        .get("error")
        .and_then(|error| token(error, "code"))
}

fn relay(body: JsonValue, id: &str, status: u16) -> Value {
    let mut server = manifest_server([Ok(GatewayResponse { status, body })]);
    wire_value(&server.handle_frame(&frame(
        id,
        GAME_INFORMATION_CONTENT_MANIFEST_TOOL,
        context(),
    )))
}

#[test]
fn vendored_content_manifest_case_declares_exactly_the_pinned_vectors() {
    let case = parse_json(CASE).expect("content-manifest case JSON");
    assert_eq!(
        token(&case, "contract"),
        Some("sts2.protocol/game-information-content-manifest-v1")
    );
    assert_eq!(
        token(&case, "profile"),
        Some("game-information-content-manifest-v1")
    );
    assert_eq!(
        token(&case, "schema_digest"),
        Some(CONTENT_MANIFEST_SCHEMA_DIGEST)
    );
    assert_eq!(
        case_member(&case, "max_message_bytes"),
        &JsonValue::Number(CONTENT_MANIFEST_MAX_MESSAGE_BYTES)
    );
    assert!(
        token(&case, "producer_integration")
            .expect("producer integration remark")
            .starts_with("pending in sts2-game-mod")
    );
    let mut declared = case_paths(&case, "valid_vectors");
    declared.extend(case_paths(&case, "invalid_vectors"));
    let mut pinned: Vec<String> = content_manifest_conformance_paths()
        .map(String::from)
        .collect();
    assert_eq!(pinned.remove(0), CASE_PATH);
    assert_eq!(pinned.len(), VECTORS.len());
    let mut vendored: Vec<String> = VECTORS
        .iter()
        .map(|(path, _)| String::from(*path))
        .collect();
    for paths in [&mut declared, &mut pinned, &mut vendored] {
        paths.sort();
    }
    assert_eq!(
        declared, pinned,
        "the case must declare exactly the pinned vectors"
    );
    assert_eq!(declared, vendored, "every declared vector must be vendored");
}

#[test]
fn vendored_content_manifest_vectors_relay_or_fail_closed() {
    let case = parse_json(CASE).expect("content-manifest case JSON");
    let valid = case_paths(&case, "valid_vectors");
    let invalid = case_paths(&case, "invalid_vectors");
    let bodies: Vec<JsonValue> = valid
        .iter()
        .chain(&invalid)
        .map(|path| parse_json(vector(path)).expect("accepted content-manifest vector JSON"))
        .collect();
    let ids: Vec<String> = (0..bodies.len())
        .map(|index| format!("vector-{index}"))
        .collect();
    let responses = bodies.iter().zip(&ids).map(|(body, id)| {
        let mut body = body.clone();
        set_correlation(&mut body, id);
        Ok(GatewayResponse { status: 200, body })
    });
    let mut server = manifest_server(responses);
    for (index, id) in ids.iter().enumerate() {
        let output = wire_value(&server.handle_frame(&frame(
            id,
            GAME_INFORMATION_CONTENT_MANIFEST_TOOL,
            context(),
        )));
        let expected = if index < valid.len() {
            refusal_code(&bodies[index])
        } else {
            Some("game_information_malformed_response")
        };
        match expected {
            None => assert_eq!(output["result"]["isError"], false, "{id}: {output}"),
            Some(code) => {
                assert_eq!(output["result"]["isError"], true, "{id}: {output}");
                assert_eq!(
                    output["result"]["structuredContent"]["error"]["code"], code,
                    "{id}: {output}"
                );
            }
        }
    }
    assert_eq!(server.gateway().requests.len(), bodies.len());
}

#[test]
fn vendored_content_manifest_error_mapping_is_relayable() {
    let case = parse_json(CASE).expect("content-manifest case JSON");
    let mapping = case_member(&case, "error_mapping")
        .as_object()
        .expect("content-manifest case error mapping")
        .clone();
    assert!(!mapping.is_empty());
    for (owner, pair) in mapping {
        let code = token(&pair, "code").unwrap_or_else(|| panic!("{owner} holds no code"));
        let reason = token(&pair, "reason").unwrap_or_else(|| panic!("{owner} holds no reason"));
        let mut body = manifest_golden("refusal");
        *member(&mut body, "error") = JsonValue::object([
            (String::from("code"), JsonValue::string(code)),
            (String::from("reason"), JsonValue::string(reason)),
        ]);
        set_correlation(&mut body, "mapping");
        let output = relay(body, "mapping", 403);
        assert_eq!(output["result"]["isError"], true, "{owner}: {output}");
        assert_eq!(
            output["result"]["structuredContent"]["error"]["code"], code,
            "{owner}: {output}"
        );
    }
}
