// SPDX-License-Identifier: MIT
//! Independent pinned-schema validation for the issue #51 acceptance producer
//! and its controls.
//!
//! The schema is loaded from its checked-in source of truth, so mapped requests
//! and outbound envelopes are checked against the published
//! `game-information-query-v1` contract instead of the producer's own constants,
//! and a missing required member or an extra root member is reported rather than
//! silently projected.

use jsonschema::Validator;
use serde_json::Value;

const SCHEMA_SOURCE: &str =
    include_str!("../../../../schemas/game-information-query-v1.schema.json");

/// Compiles the pinned `game-information-query-v1` schema.
pub(crate) fn validator() -> Validator {
    jsonschema::draft202012::options()
        .build(
            &serde_json::from_str::<Value>(SCHEMA_SOURCE)
                .expect("pinned game-information schema is valid JSON"),
        )
        .expect("pinned game-information schema compiles")
}

/// Reports every pinned-schema violation for one envelope.
pub(crate) fn errors(validator: &Validator, value: &Value) -> Vec<String> {
    if validator.is_valid(value) {
        return Vec::new();
    }
    validator
        .iter_errors(value)
        .map(|error| error.to_string())
        .collect()
}

/// Appends one readable pinned-schema violation for the named envelope.
pub(crate) fn record(violations: &mut Vec<String>, scope: &str, errors: &[String]) {
    violations.push(format!(
        "{scope} violates the pinned schema: {}",
        errors.join("; ")
    ));
}
