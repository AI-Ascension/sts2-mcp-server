// SPDX-License-Identifier: MIT

use crate::gateway::GatewayAdapter;
use crate::json::JsonValue;

use super::{MAX_REST_ACTION_SELECTIONS, McpServer};

pub(crate) const REST_ACTION_SELECTOR_CAPACITY_ERROR: &str =
    "Runtime-v4 REST selector admission capacity is exhausted";

impl<G: GatewayAdapter> McpServer<G> {
    pub(crate) fn reserve_rest_action_selector_capacity(
        &mut self,
        operation_id: &str,
        action: &JsonValue,
    ) -> bool {
        if !rest_action_can_open_selector(action)
            || self.rest_action_operations.contains_key(operation_id)
            || self
                .rest_action_selector_reservations
                .contains(operation_id)
        {
            return true;
        }
        if self
            .rest_action_selections
            .values()
            .filter(|context| !context.terminal)
            .count()
            + self.rest_action_selector_reservations.len()
            >= MAX_REST_ACTION_SELECTIONS
        {
            return false;
        }
        self.rest_action_selector_reservations
            .insert(operation_id.to_owned());
        true
    }

    pub(crate) fn release_rest_action_selector_reservation(&mut self, operation_id: &str) {
        self.rest_action_selector_reservations.remove(operation_id);
    }
}

fn rest_action_can_open_selector(action: &JsonValue) -> bool {
    let Some(payload) = action
        .as_object()
        .and_then(|action| action.get("action"))
        .and_then(JsonValue::as_object)
    else {
        return false;
    };
    payload.get("kind").and_then(JsonValue::as_string) == Some("rest_option")
        && matches!(
            payload.get("rest_option_id").and_then(JsonValue::as_string),
            Some("smith" | "mend")
        )
}
