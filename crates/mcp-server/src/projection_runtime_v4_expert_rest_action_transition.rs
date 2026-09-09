// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::json::JsonValue;

use super::super::{bounded_number, exact_fields, require_null};
use super::RestActionSelectionAdmission;
use super::selector::{
    positive_count, selected_ids, selection_kind, validate_completed_choices, validate_option,
    validate_selector,
};
use super::{validate_identity, witness};

pub(super) fn validate_transition(
    root: &BTreeMap<String, JsonValue>,
    transition_value: &JsonValue,
    observation: &BTreeMap<String, JsonValue>,
    generation: i64,
    operation_id: &str,
    admission: Option<&RestActionSelectionAdmission>,
    require_prior_selection_admission: bool,
) -> Result<(), &'static str> {
    let transition = transition_value
        .as_object()
        .ok_or("Runtime-v4 REST transition is not an object")?;
    let kind = transition
        .get("kind")
        .and_then(JsonValue::as_string)
        .ok_or("Runtime-v4 REST transition kind is missing")?;
    let before = transition_number(transition, "before_generation")?;
    let after = transition_number(transition, "after_generation")?;
    if before >= after || after != generation {
        return Err("Runtime-v4 REST transition generation is invalid");
    }
    let action_payload = root_action_payload(root)?;
    let option_id = transition
        .get("rest_option_id")
        .and_then(JsonValue::as_string)
        .ok_or("Runtime-v4 REST transition option identity is missing")?;
    validate_identity(transition.get("rest_option_id"))?;
    validate_option(
        transition.get("rest_option_id"),
        !matches!(kind, "rest_option_completed"),
    )?;
    if action_payload
        .get("rest_option_id")
        .and_then(JsonValue::as_string)
        != Some(option_id)
    {
        return Err("Runtime-v4 REST transition option does not match action");
    }
    match kind {
        "rest_option_completed" => {
            exact_fields(
                transition,
                &[
                    "kind",
                    "before_generation",
                    "after_generation",
                    "rest_option_id",
                    "completed",
                    "effect_witness",
                ],
            )?;
            if action_payload.get("kind").and_then(JsonValue::as_string) != Some("rest_option")
                || matches!(option_id, "smith" | "mend")
                || transition.get("completed") != Some(&JsonValue::Bool(true))
            {
                return Err("Runtime-v4 REST completed option transition is invalid");
            }
            let witness = transition
                .get("effect_witness")
                .filter(|value| !matches!(value, JsonValue::Null))
                .ok_or("Runtime-v4 REST completed option witness is missing")?;
            witness::validate_effect_witness(
                root,
                witness,
                option_id,
                operation_id,
                generation,
                None,
            )?;
            if root.get("effect_witness") != Some(witness) {
                return Err("Runtime-v4 REST root and transition witnesses differ");
            }
        }
        "rest_option_selection_requested" => {
            exact_fields(
                transition,
                &[
                    "kind",
                    "before_generation",
                    "after_generation",
                    "rest_option_id",
                    "selector",
                    "effect_witness",
                ],
            )?;
            if action_payload.get("kind").and_then(JsonValue::as_string) != Some("rest_option") {
                return Err("Runtime-v4 REST selection request action is invalid");
            }
            require_null(transition.get("effect_witness"))?;
            require_null(root.get("effect_witness"))?;
            validate_selector(
                transition.get("selector"),
                option_id,
                observation,
                admission,
            )?;
        }
        "rest_option_selection_progressed" => {
            exact_fields(
                transition,
                &[
                    "kind",
                    "before_generation",
                    "after_generation",
                    "rest_option_id",
                    "selection_id",
                    "selection_kind",
                    "required_count",
                    "selected_choice_ids",
                    "remaining_count",
                    "selector",
                    "effect_witness",
                ],
            )?;
            require_null(transition.get("effect_witness"))?;
            require_null(root.get("effect_witness"))?;
            let selection_kind = selection_kind(transition.get("selection_kind"))?;
            if (selection_kind == "card" && option_id != "smith")
                || (selection_kind == "player" && option_id != "mend")
            {
                return Err("Runtime-v4 REST selection option does not match selection kind");
            }
            let selection_id = transition
                .get("selection_id")
                .and_then(JsonValue::as_string)
                .ok_or("Runtime-v4 REST selection identity is missing")?;
            validate_identity(Some(&JsonValue::String(selection_id.to_owned())))?;
            if action_payload.get("kind").and_then(JsonValue::as_string)
                != Some(match selection_kind {
                    "card" => "select_card",
                    "player" => "select_player",
                    _ => unreachable!(),
                })
                || action_payload
                    .get("selection_id")
                    .and_then(JsonValue::as_string)
                    != Some(selection_id)
            {
                return Err("Runtime-v4 REST selection progress action is invalid");
            }
            let choice_field = if selection_kind == "card" {
                "card_id"
            } else {
                "player_id"
            };
            let choice = action_payload
                .get(choice_field)
                .and_then(JsonValue::as_string)
                .ok_or("Runtime-v4 REST selected choice is missing")?;
            let selected = transition
                .get("selected_choice_ids")
                .and_then(JsonValue::as_array)
                .ok_or("Runtime-v4 REST selected choices are missing")?;
            if !selected
                .iter()
                .any(|value| value.as_string() == Some(choice))
            {
                return Err("Runtime-v4 REST action choice is absent from transition");
            }
            if require_prior_selection_admission && admission.is_none() {
                return Err("Runtime-v4 REST selection progress lacks prior admission");
            }
            validate_selector(
                transition.get("selector"),
                option_id,
                observation,
                admission,
            )?;
            let selector = transition
                .get("selector")
                .and_then(JsonValue::as_object)
                .ok_or("Runtime-v4 REST selector is missing")?;
            for field in [
                "selection_id",
                "selection_kind",
                "required_count",
                "selected_choice_ids",
                "remaining_count",
            ] {
                if transition.get(field) != selector.get(field) {
                    return Err("Runtime-v4 REST transition and selector counts differ");
                }
            }
        }
        "rest_option_selection_completed" => {
            exact_fields(
                transition,
                &[
                    "kind",
                    "before_generation",
                    "after_generation",
                    "rest_option_id",
                    "selection_id",
                    "selection_kind",
                    "required_count",
                    "selected_choice_ids",
                    "remaining_count",
                    "completed",
                    "effect_witness",
                ],
            )?;
            if (action_payload.get("kind").and_then(JsonValue::as_string)
                != Some("confirm_selection")
                && !(option_id == "mend"
                    && action_payload.get("kind").and_then(JsonValue::as_string)
                        == Some("select_player")))
                || transition.get("completed") != Some(&JsonValue::Bool(true))
                || transition.get("remaining_count") != Some(&JsonValue::Number(0))
            {
                return Err("Runtime-v4 REST selection completion action is invalid");
            }
            let selection_id = transition
                .get("selection_id")
                .and_then(JsonValue::as_string)
                .ok_or("Runtime-v4 REST selection identity is missing")?;
            if action_payload
                .get("selection_id")
                .and_then(JsonValue::as_string)
                != Some(selection_id)
            {
                return Err("Runtime-v4 REST confirmation selection identity differs");
            }
            let selection_kind = selection_kind(transition.get("selection_kind"))?;
            if (option_id == "smith" && selection_kind != "card")
                || (option_id == "mend" && selection_kind != "player")
            {
                return Err("Runtime-v4 REST completion option does not match selection kind");
            }
            let required = positive_count(transition.get("required_count"))?;
            let selected = selected_ids(transition.get("selected_choice_ids"))?;
            if selected.len() != required {
                return Err("Runtime-v4 REST selection count is incomplete");
            }
            if require_prior_selection_admission && admission.is_none() {
                return Err("Runtime-v4 REST selection completion lacks prior admission");
            }
            validate_completed_choices(
                observation,
                option_id,
                selection_kind,
                &selected,
                admission,
                require_prior_selection_admission,
            )?;
            if option_id == "mend"
                && action_payload.get("kind").and_then(JsonValue::as_string)
                    == Some("select_player")
                && action_payload
                    .get("player_id")
                    .and_then(JsonValue::as_string)
                    != selected.first().map(String::as_str)
            {
                return Err("Runtime-v4 Mend selection action does not match selection");
            }
            let witness = transition
                .get("effect_witness")
                .filter(|value| !matches!(value, JsonValue::Null))
                .ok_or("Runtime-v4 REST selection completion witness is missing")?;
            witness::validate_effect_witness(
                root,
                witness,
                option_id,
                operation_id,
                generation,
                Some((&selected, selection_kind)),
            )?;
            if root.get("effect_witness") != Some(witness) {
                return Err("Runtime-v4 REST root and transition witnesses differ");
            }
        }
        _ => return Err("Runtime-v4 REST transition kind is unsupported"),
    }
    Ok(())
}

fn root_action_payload(
    root: &BTreeMap<String, JsonValue>,
) -> Result<&BTreeMap<String, JsonValue>, &'static str> {
    root.get("action")
        .and_then(JsonValue::as_object)
        .and_then(|action| action.get("action"))
        .and_then(JsonValue::as_object)
        .ok_or("Runtime-v4 REST action payload is missing")
}

fn transition_number(
    transition: &BTreeMap<String, JsonValue>,
    field: &str,
) -> Result<i64, &'static str> {
    bounded_number(transition.get(field))
}
