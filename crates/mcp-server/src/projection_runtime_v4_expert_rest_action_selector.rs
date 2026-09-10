// SPDX-License-Identifier: MIT

use std::collections::{BTreeMap, BTreeSet};

use crate::json::JsonValue;

use super::{RestActionSelectionAdmission, exact_object, validate_identity};

#[path = "projection_runtime_v4_expert_rest_action_completion.rs"]
mod completion;
pub(super) use completion::validate_completed_choices;

pub(super) fn validate_option(
    value: Option<&JsonValue>,
    selector_only: bool,
) -> Result<(), &'static str> {
    let option = value
        .and_then(JsonValue::as_string)
        .ok_or("Runtime-v4 REST option identity is missing")?;
    if !matches!(
        option,
        "clone" | "cook" | "dig" | "hatch" | "heal" | "kindle" | "lift" | "smith" | "mend"
    ) || selector_only && !matches!(option, "smith" | "mend")
    {
        Err("Runtime-v4 REST option is outside the audited native set")
    } else {
        Ok(())
    }
}

pub(super) fn validate_selection_payload(
    payload: &BTreeMap<String, JsonValue>,
    expected_kind: &str,
    choice_field: &str,
) -> Result<(), &'static str> {
    if payload.len() != 4
        || payload.get("kind").and_then(JsonValue::as_string) != Some(expected_kind)
        || !payload.contains_key("selection_id")
        || !payload.contains_key("rest_option_id")
        || !payload.contains_key(choice_field)
    {
        return Err("Runtime-v4 REST selection action is malformed");
    }
    validate_identity(payload.get("selection_id"))?;
    validate_identity(payload.get("rest_option_id"))?;
    validate_identity(payload.get(choice_field))
}

pub(super) fn validate_selector(
    value: Option<&JsonValue>,
    option_id: &str,
    observation: &BTreeMap<String, JsonValue>,
    admission: Option<&RestActionSelectionAdmission>,
) -> Result<(), &'static str> {
    let selector = exact_object(
        value,
        &[
            "selection_id",
            "selection_kind",
            "required_count",
            "selected_choice_ids",
            "remaining_count",
            "legal_actions",
        ],
    )?;
    validate_identity(selector.get("selection_id"))?;
    let selection_kind = selection_kind(selector.get("selection_kind"))?;
    validate_option(Some(&JsonValue::String(option_id.to_owned())), true)?;
    if (selection_kind == "card" && option_id != "smith")
        || (selection_kind == "player" && option_id != "mend")
    {
        return Err("Runtime-v4 REST selector kind does not match its option");
    }
    let required = positive_count(selector.get("required_count"))?;
    let selected = selected_ids(selector.get("selected_choice_ids"))?;
    let remaining = match selector.get("remaining_count") {
        Some(JsonValue::Number(value)) if (0..=256).contains(value) => *value as usize,
        _ => return Err("Runtime-v4 REST remaining selection count is invalid"),
    };
    if selected.len() > required || remaining != required - selected.len() {
        return Err("Runtime-v4 REST selector counts are inconsistent");
    }
    if let Some(admission) = admission
        && (admission.option_id != option_id
            || admission.selection_kind != selection_kind
            || admission.required_count != required)
    {
        return Err("Runtime-v4 REST selector differs from prior admission");
    }
    let state = observation
        .get("state")
        .and_then(JsonValue::as_object)
        .ok_or("Runtime-v4 REST selector observation state is missing")?;
    if state.get("state").and_then(JsonValue::as_string) != Some("selection") {
        return Err("Runtime-v4 REST selector is not backed by a selection state");
    }
    let choices = state
        .get("choices")
        .and_then(JsonValue::as_array)
        .ok_or("Runtime-v4 REST selector has no visible choice surface")?;
    if choices.is_empty() {
        return Err("Runtime-v4 REST selector has no visible choice surface");
    }
    let visible_choice_ids: BTreeSet<String> = choices
        .iter()
        .filter_map(|choice| {
            choice
                .as_object()
                .and_then(|choice| choice.get("choice_id"))
                .and_then(JsonValue::as_string)
                .map(str::to_owned)
        })
        .collect();
    if visible_choice_ids.len() != choices.len() {
        return Err("Runtime-v4 REST selector choice identities are invalid");
    }
    if selected
        .iter()
        .any(|choice| !visible_choice_ids.contains(choice))
    {
        return Err("Runtime-v4 REST selector choice is not visible");
    }
    if let Some(admission) = admission
        && selected
            .iter()
            .any(|choice| !admission.choice_ids.contains(choice))
    {
        return Err("Runtime-v4 REST selector choice exceeds prior admission");
    }
    let legal_actions = selector
        .get("legal_actions")
        .and_then(JsonValue::as_array)
        .ok_or("Runtime-v4 REST selector legal actions are missing")?;
    if legal_actions.len() > 256 {
        return Err("Runtime-v4 REST selector legal actions exceed the bound");
    }
    if legal_actions.is_empty() {
        return Err("Runtime-v4 REST selector has no visible legal actions");
    }
    let mut action_ids = BTreeSet::new();
    let mut has_confirm = false;
    let mut has_cancel = false;
    let mut has_choice = false;
    for value in legal_actions {
        let action = exact_object(Some(value), &["action_id", "action"])?;
        validate_identity(action.get("action_id"))?;
        let action_id = action
            .get("action_id")
            .and_then(JsonValue::as_string)
            .ok_or("Runtime-v4 REST legal action identity is missing")?;
        if !action_ids.insert(action_id.to_owned()) {
            return Err("Runtime-v4 REST selector legal action IDs are duplicated");
        }
        let payload = action
            .get("action")
            .and_then(JsonValue::as_object)
            .ok_or("Runtime-v4 REST legal action payload is missing")?;
        if let Some(admission) = admission
            && let Some(choice) = selection_choice_id(payload)
            && !admission.choice_ids.contains(choice)
        {
            return Err("Runtime-v4 REST selector choice exceeds prior admission");
        }
        let kind = payload
            .get("kind")
            .and_then(JsonValue::as_string)
            .ok_or("Runtime-v4 REST legal action kind is missing")?;
        match kind {
            "select_card" => {
                validate_selection_payload(payload, kind, "card_id")?;
                if selection_kind != "card" {
                    return Err(
                        "Runtime-v4 REST selector has a card action for a player selection",
                    );
                }
                ensure_selector_binding(payload, selector, option_id)?;
                ensure_unselected(payload.get("card_id"), &selected)?;
                if !visible_choice_ids.contains(
                    payload
                        .get("card_id")
                        .and_then(JsonValue::as_string)
                        .ok_or("Runtime-v4 REST selector card choice is missing")?,
                ) {
                    return Err("Runtime-v4 REST card selector action is not visible");
                }
                has_choice = true;
            }
            "select_player" => {
                validate_selection_payload(payload, kind, "player_id")?;
                if selection_kind != "player" {
                    return Err(
                        "Runtime-v4 REST selector has a player action for a card selection",
                    );
                }
                ensure_selector_binding(payload, selector, option_id)?;
                ensure_unselected(payload.get("player_id"), &selected)?;
                if !visible_choice_ids.contains(
                    payload
                        .get("player_id")
                        .and_then(JsonValue::as_string)
                        .ok_or("Runtime-v4 REST selector player choice is missing")?,
                ) {
                    return Err("Runtime-v4 REST player selector action is not visible");
                }
                has_choice = true;
            }
            "confirm_selection" | "cancel_selection" => {
                if payload.len() != 3
                    || payload.get("kind").and_then(JsonValue::as_string) != Some(kind)
                {
                    return Err("Runtime-v4 REST selector control action is malformed");
                }
                validate_identity(payload.get("selection_id"))?;
                validate_identity(payload.get("rest_option_id"))?;
                ensure_selector_binding(payload, selector, option_id)?;
                has_confirm |= kind == "confirm_selection";
                has_cancel |= kind == "cancel_selection";
            }
            _ => return Err("Runtime-v4 REST selector has an unsupported legal action"),
        }
    }
    if has_confirm != (remaining == 0) || !has_cancel || remaining > 0 && !has_choice {
        return Err("Runtime-v4 REST selector control catalog is incomplete");
    }
    Ok(())
}

fn selection_choice_id(payload: &BTreeMap<String, JsonValue>) -> Option<&str> {
    match payload.get("kind").and_then(JsonValue::as_string) {
        Some("select_card") => payload.get("card_id").and_then(JsonValue::as_string),
        Some("select_player") => payload.get("player_id").and_then(JsonValue::as_string),
        _ => None,
    }
}

pub(super) fn selection_kind(value: Option<&JsonValue>) -> Result<&str, &'static str> {
    match value.and_then(JsonValue::as_string) {
        Some(value @ ("card" | "player")) => Ok(value),
        _ => Err("Runtime-v4 REST selection kind is invalid"),
    }
}

pub(super) fn positive_count(value: Option<&JsonValue>) -> Result<usize, &'static str> {
    match value {
        Some(JsonValue::Number(value)) if (1..=256).contains(value) => Ok(*value as usize),
        _ => Err("Runtime-v4 REST required selection count is invalid"),
    }
}

pub(super) fn selected_ids(value: Option<&JsonValue>) -> Result<Vec<String>, &'static str> {
    let values = value
        .and_then(JsonValue::as_array)
        .ok_or("Runtime-v4 REST selected choices are not an array")?;
    if values.len() > 256 {
        return Err("Runtime-v4 REST selected choices exceed the bound");
    }
    let mut seen = BTreeSet::new();
    let mut result = Vec::with_capacity(values.len());
    for value in values {
        validate_identity(Some(value))?;
        let value = value
            .as_string()
            .ok_or("Runtime-v4 REST selected choice is not an identity")?;
        if !seen.insert(value.to_owned()) {
            return Err("Runtime-v4 REST selected choices are duplicated");
        }
        result.push(value.to_owned());
    }
    Ok(result)
}

fn ensure_selector_binding(
    payload: &BTreeMap<String, JsonValue>,
    selector: &BTreeMap<String, JsonValue>,
    option_id: &str,
) -> Result<(), &'static str> {
    if payload.get("selection_id") != selector.get("selection_id")
        || payload.get("rest_option_id") != Some(&JsonValue::String(option_id.to_owned()))
    {
        Err("Runtime-v4 REST legal action is bound to another selector")
    } else {
        Ok(())
    }
}

fn ensure_unselected(value: Option<&JsonValue>, selected: &[String]) -> Result<(), &'static str> {
    let choice = value
        .and_then(JsonValue::as_string)
        .ok_or("Runtime-v4 REST selector choice is missing")?;
    if selected.iter().any(|item| item == choice) {
        Err("Runtime-v4 REST selector offers an already selected choice")
    } else {
        Ok(())
    }
}
