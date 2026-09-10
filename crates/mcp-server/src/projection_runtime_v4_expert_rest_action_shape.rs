// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::json::JsonValue;

use super::{RestActionSelectionAdmission, exact_object, validate_identity};

#[path = "projection_runtime_v4_expert_rest_action_selector.rs"]
mod selector;
#[path = "projection_runtime_v4_expert_rest_action_transition.rs"]
mod transition;
#[path = "projection_runtime_v4_expert_rest_action_witness.rs"]
mod witness;

use selector::{validate_option, validate_selection_payload};

pub(super) fn validate_transition(
    root: &BTreeMap<String, JsonValue>,
    transition_value: &JsonValue,
    observation: &BTreeMap<String, JsonValue>,
    generation: i64,
    operation_id: &str,
    admission: Option<&RestActionSelectionAdmission>,
    require_prior_selection_admission: bool,
) -> Result<(), &'static str> {
    transition::validate_transition(
        root,
        transition_value,
        observation,
        generation,
        operation_id,
        admission,
        require_prior_selection_admission,
    )
}

pub(super) fn validate_action_reference(value: Option<&JsonValue>) -> Result<(), &'static str> {
    let action = exact_object(value, &["action_id", "action"])?;
    validate_identity(action.get("action_id"))?;
    let payload = action
        .get("action")
        .and_then(JsonValue::as_object)
        .ok_or("Runtime-v4 REST action payload is not an object")?;
    match payload.get("kind").and_then(JsonValue::as_string) {
        Some("rest_option") => {
            if payload.len() != 2
                || payload.get("kind").and_then(JsonValue::as_string) != Some("rest_option")
                || !payload.contains_key("rest_option_id")
            {
                return Err("Runtime-v4 REST option action is malformed");
            }
            validate_identity(payload.get("rest_option_id"))?;
            validate_option(payload.get("rest_option_id"), false)?;
        }
        Some("select_card") => {
            validate_selection_payload(payload, "select_card", "card_id")?;
            if payload.get("rest_option_id").and_then(JsonValue::as_string) != Some("smith") {
                return Err("Runtime-v4 REST card selection is not bound to smith");
            }
        }
        Some("select_player") => {
            validate_selection_payload(payload, "select_player", "player_id")?;
            if payload.get("rest_option_id").and_then(JsonValue::as_string) != Some("mend") {
                return Err("Runtime-v4 REST player selection is not bound to mend");
            }
        }
        Some("confirm_selection" | "cancel_selection") => {
            if payload.len() != 3
                || !payload.contains_key("kind")
                || !payload.contains_key("selection_id")
                || !payload.contains_key("rest_option_id")
            {
                return Err("Runtime-v4 REST selection control is malformed");
            }
            validate_identity(payload.get("selection_id"))?;
            validate_identity(payload.get("rest_option_id"))?;
            validate_option(payload.get("rest_option_id"), true)?;
        }
        _ => return Err("Runtime-v4 REST action kind is unsupported"),
    }
    Ok(())
}
