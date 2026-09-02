// SPDX-License-Identifier: MIT

use jsonschema::draft202012::options;
use serde_json::Value;

const SCHEMA: &str = include_str!("../../../protocol-artifact/poc-v1/schema.json");
const FIXTURES: [(&str, &str); 6] = [
    (
        "state-request",
        include_str!("../../../protocol-artifact/poc-v1/golden/state-request.json"),
    ),
    (
        "state-response",
        include_str!("../../../protocol-artifact/poc-v1/golden/state-response.json"),
    ),
    (
        "action-request",
        include_str!("../../../protocol-artifact/poc-v1/golden/action-request.json"),
    ),
    (
        "action-accepted",
        include_str!("../../../protocol-artifact/poc-v1/golden/action-accepted.json"),
    ),
    (
        "action-rejected",
        include_str!("../../../protocol-artifact/poc-v1/golden/action-rejected.json"),
    ),
    (
        "invalid-action",
        include_str!("../../../protocol-artifact/poc-v1/fixtures/invalid-action.json"),
    ),
];

#[test]
fn copied_schema_validates_all_packaged_fixtures_and_rejects_unknown_shape() -> Result<(), String> {
    let schema: Value = serde_json::from_str(SCHEMA).map_err(|error| error.to_string())?;
    if schema.get("$id") != Some(&Value::String(String::from("sts2-poc-v1"))) {
        return Err(String::from("copied schema has an unexpected identity"));
    }
    let validator = options()
        .build(&schema)
        .map_err(|error| error.to_string())?;
    let mut state = None;
    for (name, fixture) in FIXTURES {
        let value: Value = serde_json::from_str(fixture).map_err(|error| error.to_string())?;
        if !validator.is_valid(&value) {
            return Err(format!("fixture is rejected by the copied schema: {name}"));
        }
        if name == "state-response" {
            state = Some(value);
        }
    }
    let mut unknown = state.clone().ok_or("state fixture was not loaded")?;
    let Some(object) = unknown.as_object_mut() else {
        return Err(String::from("state fixture is not an object"));
    };
    object.insert(String::from("unexpected"), Value::Bool(true));
    if validator.is_valid(&unknown) {
        return Err(String::from("schema accepted an unknown top-level field"));
    }
    let mut over_generation = state.clone().ok_or("state fixture was not loaded")?;
    let Some(object) = over_generation.as_object_mut() else {
        return Err(String::from("state fixture is not an object"));
    };
    object.insert(
        String::from("generation"),
        Value::from(9_007_199_254_740_992_i64),
    );
    if validator.is_valid(&over_generation) {
        return Err(String::from("schema accepted an oversized generation"));
    }
    let mut missing = state.ok_or("state fixture was not retained")?;
    let Some(object) = missing.as_object_mut() else {
        return Err(String::from("state fixture is not an object"));
    };
    object.remove("error_code");
    if validator.is_valid(&missing) {
        return Err(String::from("schema accepted a missing required field"));
    }
    Ok(())
}
