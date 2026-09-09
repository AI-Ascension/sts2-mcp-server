// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::json::JsonValue;

use super::RestActionSelectionAdmission;

pub(crate) fn validate_completed_choices(
    observation: &BTreeMap<String, JsonValue>,
    option_id: &str,
    selection_kind: &str,
    selected: &[String],
    admission: Option<&RestActionSelectionAdmission>,
    require_prior_selection_admission: bool,
) -> Result<(), &'static str> {
    if let Some(admission) = admission {
        if admission.option_id != option_id
            || admission.selection_kind != selection_kind
            || admission.required_count != selected.len()
            || selected
                .iter()
                .any(|choice| !admission.choice_ids.contains(choice))
        {
            return Err("Runtime-v4 REST selection is outside prior admission");
        }
        return Ok(());
    }
    if require_prior_selection_admission {
        return Err("Runtime-v4 REST selection lacks prior admission");
    }
    if selection_kind == "player" {
        if selected.iter().any(|value| !value.starts_with("player:")) {
            return Err("Runtime-v4 REST player selection identity is invalid");
        }
        return Ok(());
    }
    let deck = observation
        .get("player")
        .and_then(JsonValue::as_object)
        .and_then(|player| player.get("deck"))
        .and_then(JsonValue::as_array)
        .ok_or("Runtime-v4 REST card selection has no visible deck")?;
    if selected.iter().any(|selected| {
        !deck.iter().any(|card| {
            card.as_object()
                .and_then(|card| card.get("card_id"))
                .and_then(JsonValue::as_string)
                == Some(selected.as_str())
        })
    }) {
        Err("Runtime-v4 REST card selection is not visible in the observation")
    } else {
        Ok(())
    }
}
