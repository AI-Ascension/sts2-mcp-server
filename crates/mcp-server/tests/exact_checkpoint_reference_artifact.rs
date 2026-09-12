// SPDX-License-Identifier: MIT

//! Consumer conformance for the shared public checkpoint-reference envelope.
//!
//! The adapter accepts a bounded, digest-free reference shape and never carries an exact-state,
//! checkpoint, blob, or compatibility digest. This binds the copied protocol artifact and rejects a
//! privileged member or an unsupported version rather than passing it through.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use jsonschema::draft202012::options;
use serde_json::Value;

const SCHEMA: &str =
    include_str!("../../../protocol-artifact/exact-checkpoint-reference-v1/schema.json");
const MANIFEST: &str =
    include_str!("../../../protocol-artifact/exact-checkpoint-reference-v1/manifest.json");
const REFERENCE: &str =
    include_str!("../../../protocol-artifact/exact-checkpoint-reference-v1/golden/reference.json");
const PRIVILEGED: &str = include_str!(
    "../../../protocol-artifact/exact-checkpoint-reference-v1/golden/invalid-privileged.json"
);
const FUTURE: &str = include_str!(
    "../../../protocol-artifact/exact-checkpoint-reference-v1/golden/unsupported-version.json"
);

fn parse(text: &str) -> Value {
    serde_json::from_str(text).expect("fixture JSON is valid")
}

fn validator() -> jsonschema::Validator {
    options()
        .build(&parse(SCHEMA))
        .expect("reference schema compiles")
}

#[test]
fn copied_reference_artifact_is_bound_by_its_manifest() {
    let manifest = parse(MANIFEST);
    assert_eq!(
        manifest["artifact"],
        "sts2-protocol/exact-checkpoint-reference-v1"
    );
    assert_eq!(
        manifest["protocol_version"],
        "exact-checkpoint-reference-v1"
    );
    assert_eq!(
        manifest["schema_digest"],
        "028e00d06f9f2b16cb9097f47aedd057e74046a7cb2ba97362978e18029f48ab"
    );
    assert_eq!(
        manifest["consumers"].as_array().expect("consumers").len(),
        3
    );
}

#[test]
fn valid_reference_is_accepted_and_privileged_or_future_ones_are_not() {
    let validator = validator();
    validator
        .validate(&parse(REFERENCE))
        .expect("valid reference is accepted");
    assert!(validator.validate(&parse(PRIVILEGED)).is_err());
    assert!(validator.validate(&parse(FUTURE)).is_err());
}

#[test]
fn the_public_envelope_defines_no_digest_property() {
    let schema = parse(SCHEMA);
    assert_eq!(schema["additionalProperties"], false);
    let properties = schema["properties"].as_object().expect("properties");
    assert_eq!(properties.len(), 8);
    for name in properties.keys() {
        assert!(
            !name.contains("digest"),
            "public reference schema must not define {name}"
        );
    }
    let handle_pattern = properties["handle"]["pattern"].as_str().expect("pattern");
    assert!(handle_pattern.contains("ckpt-h1"));
    let assurance = properties["assurance"]["enum"].as_array().expect("enum");
    assert_eq!(assurance.len(), 5);
}
