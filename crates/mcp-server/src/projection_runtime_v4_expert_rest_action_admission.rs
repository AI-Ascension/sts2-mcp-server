// SPDX-License-Identifier: MIT

use std::collections::BTreeSet;

use crate::json::JsonValue;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct RestActionSelectionKey {
    pub(crate) instance_id: String,
    pub(crate) session_id: String,
    pub(crate) lease_id: String,
    pub(crate) lease_epoch: i64,
    pub(crate) selection_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RestActionSelectionAdmission {
    pub(crate) option_id: String,
    pub(crate) selection_kind: String,
    pub(crate) required_count: usize,
    pub(crate) choice_ids: BTreeSet<String>,
}

pub(crate) fn rest_action_selection_id(body: &JsonValue) -> Option<&str> {
    let transition = body
        .as_object()?
        .get("transition")
        .and_then(JsonValue::as_object)?;
    if transition.get("kind").and_then(JsonValue::as_string)
        == Some("rest_option_selection_requested")
    {
        transition
            .get("selector")
            .and_then(JsonValue::as_object)?
            .get("selection_id")
            .and_then(JsonValue::as_string)
    } else {
        transition
            .get("selection_id")
            .and_then(JsonValue::as_string)
    }
}

pub(crate) fn rest_action_selection_admission(
    body: &JsonValue,
) -> Option<(String, RestActionSelectionAdmission)> {
    let root = body.as_object()?;
    let transition = root.get("transition").and_then(JsonValue::as_object)?;
    if !matches!(
        transition.get("kind").and_then(JsonValue::as_string),
        Some("rest_option_selection_requested" | "rest_option_selection_progressed")
    ) {
        return None;
    }
    let selector = transition.get("selector").and_then(JsonValue::as_object)?;
    let selection_id = selector
        .get("selection_id")
        .and_then(JsonValue::as_string)?;
    let option_id = transition
        .get("rest_option_id")
        .and_then(JsonValue::as_string)?;
    let selection_kind = selector
        .get("selection_kind")
        .and_then(JsonValue::as_string)?;
    let required_count = match selector.get("required_count") {
        Some(JsonValue::Number(value)) if (1..=256).contains(value) => *value as usize,
        _ => return None,
    };
    let mut choice_ids = BTreeSet::new();
    for choice in selector
        .get("selected_choice_ids")
        .and_then(JsonValue::as_array)?
    {
        choice_ids.insert(choice.as_string()?.to_owned());
    }
    for legal_action in selector
        .get("legal_actions")
        .and_then(JsonValue::as_array)?
    {
        let payload = legal_action
            .as_object()?
            .get("action")
            .and_then(JsonValue::as_object)?;
        let choice = match payload.get("kind").and_then(JsonValue::as_string) {
            Some("select_card") => payload.get("card_id").and_then(JsonValue::as_string),
            Some("select_player") => payload.get("player_id").and_then(JsonValue::as_string),
            _ => None,
        };
        if let Some(choice) = choice {
            choice_ids.insert(choice.to_owned());
        }
    }
    if choice_ids.len() > 256 {
        return None;
    }
    Some((
        selection_id.to_owned(),
        RestActionSelectionAdmission {
            option_id: option_id.to_owned(),
            selection_kind: selection_kind.to_owned(),
            required_count,
            choice_ids,
        },
    ))
}
