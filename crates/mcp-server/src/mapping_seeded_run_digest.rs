// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::json::JsonValue;

pub(super) fn canonical_context(
    object: &BTreeMap<String, JsonValue>,
) -> Result<String, &'static str> {
    let baseline = object
        .get("profile_baseline")
        .and_then(JsonValue::as_object)
        .ok_or("selected_context profile_baseline is missing")?;
    let compatibility = object
        .get("compatibility")
        .and_then(JsonValue::as_object)
        .ok_or("selected_context compatibility is missing")?;
    let game = compatibility
        .get("game")
        .and_then(JsonValue::as_object)
        .ok_or("selected_context game compatibility is missing")?;
    let modification = compatibility
        .get("mod")
        .and_then(JsonValue::as_object)
        .ok_or("selected_context mod compatibility is missing")?;
    let value = |source: &BTreeMap<String, JsonValue>, key: &str| {
        source
            .get(key)
            .map(JsonValue::to_json)
            .ok_or("selected_context canonical field is missing")
    };
    Ok(format!(
        "{{\"context_id\":{},\"game_mode\":{},\"character\":{},\"ascension\":{},\"modifiers\":{},\"acts\":{},\"selection_policy\":{},\"profile_baseline\":{{\"kind\":{},\"identity\":{},\"digest\":{}}},\"save_policy\":{},\"compatibility\":{{\"game\":{{\"identity\":{},\"digest\":{}}},\"mod\":{{\"identity\":{},\"digest\":{}}}}}}}",
        value(object, "context_id")?,
        value(object, "game_mode")?,
        value(object, "character")?,
        value(object, "ascension")?,
        value(object, "modifiers")?,
        value(object, "acts")?,
        value(object, "selection_policy")?,
        value(baseline, "kind")?,
        value(baseline, "identity")?,
        value(baseline, "digest")?,
        value(object, "save_policy")?,
        value(game, "identity")?,
        value(game, "digest")?,
        value(modification, "identity")?,
        value(modification, "digest")?,
    ))
}
