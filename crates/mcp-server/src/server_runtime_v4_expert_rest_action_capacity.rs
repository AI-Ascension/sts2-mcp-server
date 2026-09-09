// SPDX-License-Identifier: MIT

use crate::gateway::GatewayAdapter;
use crate::json::JsonValue;
use crate::projection::{RestActionSelectionAdmission, RestActionSelectionKey};

use super::{
    MAX_REST_ACTION_SELECTIONS, McpServer, RestActionOperationSelection, RestActionSelectionContext,
};

pub(crate) const REST_ACTION_SELECTOR_CAPACITY_ERROR: &str =
    "Runtime-v4 REST selector admission capacity is exhausted";

impl<G: GatewayAdapter> McpServer<G> {
    pub(crate) fn rest_action_selection_admission(
        &self,
        instance_id: &str,
        session_id: &str,
        lease_id: &str,
        lease_epoch: i64,
        body: &JsonValue,
    ) -> Option<&RestActionSelectionAdmission> {
        let selection_id = crate::projection::rest_action_selection_id(body)?;
        let key = RestActionSelectionKey {
            instance_id: instance_id.to_owned(),
            session_id: session_id.to_owned(),
            lease_id: lease_id.to_owned(),
            lease_epoch,
            selection_id: selection_id.to_owned(),
        };
        let operation_id = body
            .as_object()
            .and_then(|root| root.get("operation_id"))
            .and_then(JsonValue::as_string);
        if let Some(operation_id) = operation_id
            && let Some(selection) = self.rest_action_operation_selection(operation_id)
            && selection.key == key
        {
            return Some(&selection.admission);
        }
        self.rest_action_selection_admission_for_key(&key)
    }

    pub(crate) fn remember_rest_action_selection(
        &mut self,
        instance_id: &str,
        session_id: &str,
        lease_id: &str,
        lease_epoch: i64,
        body: &JsonValue,
    ) -> bool {
        let Some(selection_id) = crate::projection::rest_action_selection_id(body) else {
            return true;
        };
        let Some(root) = body.as_object() else {
            return false;
        };
        let Some(operation_id) = root.get("operation_id").and_then(JsonValue::as_string) else {
            return false;
        };
        let Some(JsonValue::Number(generation)) = root.get("generation") else {
            return false;
        };
        let generation = *generation;
        let key = RestActionSelectionKey {
            instance_id: instance_id.to_owned(),
            session_id: session_id.to_owned(),
            lease_id: lease_id.to_owned(),
            lease_epoch,
            selection_id: selection_id.to_owned(),
        };
        let terminal = body
            .as_object()
            .and_then(|root| root.get("transition"))
            .and_then(JsonValue::as_object)
            .and_then(|transition| transition.get("kind"))
            .and_then(JsonValue::as_string)
            == Some("rest_option_selection_completed");

        let admission = crate::projection::rest_action_selection_admission(body)
            .map(|(_, admission)| admission)
            .or_else(|| self.rest_action_selection_admission_for_key(&key).cloned());
        let Some(admission) = admission else {
            return false;
        };

        let existing_generation = self.rest_action_selection_generation(&key);
        if terminal {
            let terminal_is_newer =
                existing_generation.is_none_or(|existing| generation >= existing);
            if let Some(context) = self.rest_action_selections.get_mut(&key)
                && terminal_is_newer
            {
                context.generation = generation;
                context.terminal = true;
            }
            self.remember_rest_action_operation_selection(
                operation_id,
                RestActionOperationSelection {
                    key,
                    admission,
                    generation,
                    terminal: terminal_is_newer,
                },
            );
            return true;
        }

        let selector_is_terminal = self.rest_action_selection_is_terminal(&key);
        self.remember_rest_action_operation_selection(
            operation_id,
            RestActionOperationSelection {
                key: key.clone(),
                admission: admission.clone(),
                generation,
                terminal: selector_is_terminal,
            },
        );
        if selector_is_terminal {
            return true;
        }
        let history_generation = self
            .rest_action_selection_history(&key)
            .map(|selection| selection.generation);
        if let Some(context) = self.rest_action_selections.get_mut(&key) {
            if generation > context.generation
                && history_generation.is_none_or(|known| generation >= known)
            {
                context.generation = generation;
                context.admission = admission;
            }
            return true;
        }
        if self
            .rest_action_selection_history(&key)
            .is_some_and(|selection| generation < selection.generation)
        {
            // A newer response is retained in the operation ledger even when
            // the bounded active catalog could not admit it. Do not reinsert
            // an older receipt when a slot later becomes available.
            return true;
        }
        if self.rest_action_selections.len() >= MAX_REST_ACTION_SELECTIONS
            && !self.rest_action_selections.contains_key(&key)
        {
            let Some(eviction_key) = self
                .rest_action_selections
                .iter()
                .find_map(|(key, context)| context.terminal.then(|| key.clone()))
            else {
                // Never discard an active catalog. The caller must fail closed
                // rather than surface a selector it cannot later reconcile.
                return false;
            };
            self.rest_action_selections.remove(&eviction_key);
        }
        self.rest_action_selections.insert(
            key,
            RestActionSelectionContext {
                admission,
                generation,
                terminal: false,
            },
        );
        true
    }

    fn rest_action_selection_is_terminal(&self, key: &RestActionSelectionKey) -> bool {
        self.rest_action_selections
            .get(key)
            .is_some_and(|context| context.terminal)
            || self
                .rest_action_operations
                .values()
                .filter_map(|context| context.selection.as_ref())
                .any(|selection| selection.key == *key && selection.terminal)
    }

    fn rest_action_selection_admission_for_key(
        &self,
        key: &RestActionSelectionKey,
    ) -> Option<&RestActionSelectionAdmission> {
        let active = self.rest_action_selections.get(key);
        let history = self.rest_action_selection_history(key);
        match (active, history) {
            (Some(active), Some(history)) if history.generation > active.generation => {
                Some(&history.admission)
            }
            (Some(active), _) => Some(&active.admission),
            (_, Some(history)) => Some(&history.admission),
            (None, None) => None,
        }
    }

    fn rest_action_selection_generation(&self, key: &RestActionSelectionKey) -> Option<i64> {
        let active = self
            .rest_action_selections
            .get(key)
            .map(|context| context.generation);
        let history = self
            .rest_action_selection_history(key)
            .map(|selection| selection.generation);
        active.into_iter().chain(history).max()
    }

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
