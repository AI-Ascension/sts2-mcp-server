// SPDX-License-Identifier: MIT

use std::collections::{BTreeMap, BTreeSet};

use crate::json::JsonValue;
use crate::protocol_artifact_runtime_v4_expert_rest_action::RUNTIME_V4_EXPERT_REST_ACTION_EFFECT_WITNESS_VERSION;

use super::super::bounded_number;
use super::validate_identity;

pub(super) fn validate_effect_witness(
    root: &BTreeMap<String, JsonValue>,
    value: &JsonValue,
    option_id: &str,
    operation_id: &str,
    generation: i64,
    selection: Option<(&[String], &str)>,
) -> Result<(), &'static str> {
    let witness = value
        .as_object()
        .ok_or("Runtime-v4 REST effect witness is not an object")?;
    let is_mend = option_id == "mend";
    let fields = if is_mend {
        &[
            "version",
            "kind",
            "operation_id",
            "rest_option_id",
            "generation",
            "target_player_id",
            "evidence",
        ][..]
    } else {
        &[
            "version",
            "kind",
            "operation_id",
            "rest_option_id",
            "generation",
            "evidence",
        ][..]
    };
    if witness.len() != fields.len() || fields.iter().any(|field| !witness.contains_key(*field)) {
        return Err("Runtime-v4 REST effect witness contains unknown or missing fields");
    }
    if witness.get("version").and_then(JsonValue::as_string)
        != Some(RUNTIME_V4_EXPERT_REST_ACTION_EFFECT_WITNESS_VERSION)
        || witness.get("operation_id").and_then(JsonValue::as_string) != Some(operation_id)
        || witness.get("rest_option_id").and_then(JsonValue::as_string) != Some(option_id)
        || bounded_number(witness.get("generation"))? != generation
    {
        return Err("Runtime-v4 REST effect witness identity is inconsistent");
    }
    let expected_kind = expected_kind(option_id)?;
    if witness.get("kind").and_then(JsonValue::as_string) != Some(expected_kind) {
        return Err("Runtime-v4 REST effect witness kind does not match option");
    }
    if is_mend {
        validate_identity(witness.get("target_player_id"))?;
        if let Some((selected, selection_kind)) = selection
            && (selection_kind != "player"
                || selected.len() != 1
                || witness.get("target_player_id") != Some(&JsonValue::String(selected[0].clone())))
        {
            return Err("Runtime-v4 Mend witness target does not match selection");
        }
    }
    validate_evidence(witness.get("evidence"), expected_kind)?;
    if expected_kind == "smith_applied"
        && let Some((selected, selection_kind)) = selection
        && (selection_kind != "card"
            || !same_ids(
                witness
                    .get("evidence")
                    .and_then(JsonValue::as_object)
                    .and_then(|evidence| evidence.get("upgraded_card_ids")),
                selected,
            ))
    {
        return Err("Runtime-v4 Smith witness does not match selected cards");
    }
    let observation_state_id = root
        .get("observation")
        .and_then(JsonValue::as_object)
        .and_then(|observation| observation.get("state_id"))
        .and_then(JsonValue::as_string);
    if matches!(
        expected_kind,
        "kindle_applied" | "lift_applied" | "mend_applied"
    ) && witness
        .get("evidence")
        .and_then(JsonValue::as_object)
        .and_then(|evidence| evidence.get("kind"))
        .and_then(JsonValue::as_string)
        == Some("native_completion")
        && witness
            .get("evidence")
            .and_then(JsonValue::as_object)
            .and_then(|evidence| evidence.get("native_state_id"))
            .and_then(JsonValue::as_string)
            != observation_state_id
    {
        return Err("Runtime-v4 REST native evidence state does not match observation");
    }
    Ok(())
}

fn expected_kind(option_id: &str) -> Result<&'static str, &'static str> {
    match option_id {
        "clone" => Ok("clone_applied"),
        "cook" => Ok("cook_applied"),
        "dig" => Ok("dig_applied"),
        "hatch" => Ok("hatch_applied"),
        "heal" => Ok("heal_applied"),
        "kindle" => Ok("kindle_applied"),
        "lift" => Ok("lift_applied"),
        "smith" => Ok("smith_applied"),
        "mend" => Ok("mend_applied"),
        _ => Err("Runtime-v4 REST option has no supported witness kind"),
    }
}

fn validate_evidence(value: Option<&JsonValue>, witness_kind: &str) -> Result<(), &'static str> {
    let evidence = value
        .and_then(JsonValue::as_object)
        .ok_or("Runtime-v4 REST effect evidence is not an object")?;
    match witness_kind {
        "clone_applied" => validate_card_change(evidence, "added_card_ids")?,
        "cook_applied" => validate_card_change(evidence, "removed_card_ids")?,
        "dig_applied" | "hatch_applied" => {
            let fields = ["kind", "added_relic_ids", "removed_relic_ids"];
            exact_fields(evidence, &fields)?;
            if evidence.get("kind").and_then(JsonValue::as_string) != Some("relic_change") {
                return Err("Runtime-v4 REST relic evidence kind is invalid");
            }
            validate_ids(evidence.get("added_relic_ids"))?;
            validate_ids(evidence.get("removed_relic_ids"))?;
            let required = "added_relic_ids";
            if evidence
                .get(required)
                .and_then(JsonValue::as_array)
                .is_none_or(Vec::is_empty)
            {
                return Err("Runtime-v4 REST relic evidence does not show the required change");
            }
        }
        "heal_applied" => validate_hp_change(evidence)?,
        "mend_applied" => match evidence.get("kind").and_then(JsonValue::as_string) {
            Some("hp_change") => validate_hp_change(evidence)?,
            Some("native_completion") => validate_native_completion(evidence)?,
            _ => return Err("Runtime-v4 REST Mend evidence is invalid"),
        },
        "kindle_applied" => validate_native_completion(evidence)?,
        "lift_applied" => {
            if evidence.get("kind").and_then(JsonValue::as_string) == Some("stat_change") {
                validate_stat_change(evidence)?;
            } else {
                validate_native_completion(evidence)?;
            }
        }
        "smith_applied" => validate_card_change(evidence, "upgraded_card_ids")?,
        _ => return Err("Runtime-v4 REST effect witness kind is unsupported"),
    }
    Ok(())
}

fn validate_card_change(
    evidence: &BTreeMap<String, JsonValue>,
    required: &str,
) -> Result<(), &'static str> {
    exact_fields(
        evidence,
        &[
            "kind",
            "added_card_ids",
            "removed_card_ids",
            "upgraded_card_ids",
        ],
    )?;
    if evidence.get("kind").and_then(JsonValue::as_string) != Some("card_change") {
        return Err("Runtime-v4 REST card evidence kind is invalid");
    }
    validate_ids(evidence.get("added_card_ids"))?;
    validate_ids(evidence.get("removed_card_ids"))?;
    validate_ids(evidence.get("upgraded_card_ids"))?;
    if evidence
        .get(required)
        .and_then(JsonValue::as_array)
        .is_none_or(Vec::is_empty)
    {
        return Err("Runtime-v4 REST card evidence does not show the required change");
    }
    Ok(())
}

fn validate_hp_change(evidence: &BTreeMap<String, JsonValue>) -> Result<(), &'static str> {
    exact_fields(
        evidence,
        &[
            "kind",
            "hp_before",
            "hp_after",
            "max_hp_before",
            "max_hp_after",
        ],
    )?;
    if evidence.get("kind").and_then(JsonValue::as_string) != Some("hp_change") {
        return Err("Runtime-v4 REST HP evidence kind is invalid");
    }
    for field in ["hp_before", "hp_after", "max_hp_before", "max_hp_after"] {
        bounded_small_number(evidence.get(field))?;
    }
    let Some(JsonValue::Number(before)) = evidence.get("hp_before") else {
        return Err("Runtime-v4 REST HP evidence is invalid");
    };
    let Some(JsonValue::Number(after)) = evidence.get("hp_after") else {
        return Err("Runtime-v4 REST HP evidence is invalid");
    };
    let Some(JsonValue::Number(max_before)) = evidence.get("max_hp_before") else {
        return Err("Runtime-v4 REST HP evidence is invalid");
    };
    let Some(JsonValue::Number(max_after)) = evidence.get("max_hp_after") else {
        return Err("Runtime-v4 REST HP evidence is invalid");
    };
    if before > max_before || after > max_after || after <= before {
        return Err("Runtime-v4 REST HP evidence is incomplete or unchanged");
    }
    Ok(())
}

fn validate_stat_change(evidence: &BTreeMap<String, JsonValue>) -> Result<(), &'static str> {
    exact_fields(evidence, &["kind", "stat_id", "before", "after"])?;
    if evidence.get("kind").and_then(JsonValue::as_string) != Some("stat_change") {
        return Err("Runtime-v4 REST stat evidence kind is invalid");
    }
    validate_identity(evidence.get("stat_id"))?;
    for field in ["before", "after"] {
        match evidence.get(field) {
            Some(JsonValue::Number(value)) if (-65_535..=65_535).contains(value) => {}
            _ => return Err("Runtime-v4 REST stat evidence value is invalid"),
        }
    }
    if evidence.get("before") == evidence.get("after") {
        return Err("Runtime-v4 REST stat evidence is unchanged");
    }
    Ok(())
}

fn validate_native_completion(evidence: &BTreeMap<String, JsonValue>) -> Result<(), &'static str> {
    exact_fields(evidence, &["kind", "completion_id", "native_state_id"])?;
    if evidence.get("kind").and_then(JsonValue::as_string) != Some("native_completion") {
        return Err("Runtime-v4 REST native evidence kind is invalid");
    }
    validate_identity(evidence.get("completion_id"))?;
    validate_identity(evidence.get("native_state_id"))
}

fn validate_ids(value: Option<&JsonValue>) -> Result<(), &'static str> {
    let values = value
        .and_then(JsonValue::as_array)
        .ok_or("Runtime-v4 REST evidence IDs are not an array")?;
    if values.len() > 256 {
        return Err("Runtime-v4 REST evidence IDs exceed the bound");
    }
    let mut seen = BTreeSet::new();
    for value in values {
        validate_identity(Some(value))?;
        let value = value
            .as_string()
            .ok_or("Runtime-v4 REST evidence ID is not a string")?;
        if !seen.insert(value) {
            return Err("Runtime-v4 REST evidence IDs are duplicated");
        }
    }
    Ok(())
}

fn same_ids(value: Option<&JsonValue>, expected: &[String]) -> bool {
    value.and_then(JsonValue::as_array).is_some_and(|values| {
        values.len() == expected.len()
            && values
                .iter()
                .zip(expected)
                .all(|(value, expected)| value.as_string() == Some(expected.as_str()))
    })
}

fn bounded_small_number(value: Option<&JsonValue>) -> Result<(), &'static str> {
    match value {
        Some(JsonValue::Number(value)) if (0..=65_535).contains(value) => Ok(()),
        _ => Err("Runtime-v4 REST bounded evidence number is invalid"),
    }
}

fn exact_fields(object: &BTreeMap<String, JsonValue>, fields: &[&str]) -> Result<(), &'static str> {
    if object.len() == fields.len() && fields.iter().all(|field| object.contains_key(*field)) {
        Ok(())
    } else {
        Err("Runtime-v4 REST effect evidence contains unknown or missing fields")
    }
}
