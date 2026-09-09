// SPDX-License-Identifier: MIT

use crate::gateway::GatewayAdapter;
use crate::json::JsonValue;
use crate::projection::{RestActionSelectionAdmission, RestActionSelectionKey};

use super::McpServer;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RestActionOperationContext {
    pub(crate) mcp_session_id: String,
    pub(crate) instance_id: String,
    pub(crate) session_id: String,
    pub(crate) lease_id: String,
    pub(crate) lease_epoch: i64,
    pub(crate) generation: i64,
    pub(crate) state_id: String,
    pub(crate) action: JsonValue,
    pub(crate) selection: Option<RestActionOperationSelection>,
}

/// Bounded response-derived state retained alongside an operation binding.
///
/// The operation ledger already has to remain available for same-operation
/// reconciliation. Keeping the selector admission and terminal marker on the
/// operation lets a late receipt remain valid after the bounded selector
/// catalog evicts its key, without allowing that receipt to reactivate the
/// completed selector.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RestActionOperationSelection {
    pub(crate) key: RestActionSelectionKey,
    pub(crate) admission: RestActionSelectionAdmission,
    pub(crate) generation: i64,
    pub(crate) terminal: bool,
}

impl<G: GatewayAdapter> McpServer<G> {
    pub(crate) fn remember_rest_action_operation(
        &mut self,
        operation_id: &str,
        context: RestActionOperationContext,
    ) -> bool {
        match self.rest_action_operations.get(operation_id) {
            Some(existing) => operation_context_matches(existing, &context),
            None => {
                self.rest_action_operations
                    .insert(operation_id.to_owned(), context);
                true
            }
        }
    }

    pub(crate) fn rest_action_operation_is_compatible(
        &self,
        operation_id: &str,
        context: &RestActionOperationContext,
    ) -> bool {
        match self.rest_action_operations.get(operation_id) {
            Some(existing) => operation_context_matches(existing, context),
            None => true,
        }
    }

    pub(crate) fn remember_rest_action_operation_selection(
        &mut self,
        operation_id: &str,
        selection: RestActionOperationSelection,
    ) {
        let Some(context) = self.rest_action_operations.get_mut(operation_id) else {
            return;
        };
        let replace = match context.selection.as_ref() {
            None => true,
            Some(existing) if existing.key != selection.key => false,
            Some(existing) if existing.terminal => false,
            Some(existing) => selection.generation >= existing.generation,
        };
        if replace {
            context.selection = Some(selection);
        } else if let Some(existing) = context.selection.as_mut()
            && existing.key == selection.key
            && selection.generation >= existing.generation
        {
            existing.terminal |= selection.terminal;
        }
    }

    pub(crate) fn rest_action_operation_selection(
        &self,
        operation_id: &str,
    ) -> Option<&RestActionOperationSelection> {
        self.rest_action_operations
            .get(operation_id)
            .and_then(|context| context.selection.as_ref())
    }

    pub(crate) fn rest_action_selection_history(
        &self,
        key: &RestActionSelectionKey,
    ) -> Option<&RestActionOperationSelection> {
        self.rest_action_operations
            .values()
            .filter_map(|context| context.selection.as_ref())
            .filter(|selection| &selection.key == key)
            .max_by_key(|selection| selection.generation)
    }

    pub(crate) fn rest_action_operation_binding(
        &self,
        operation_id: &str,
        mcp_session_id: &str,
        instance_id: &str,
        session_id: &str,
        lease_id: &str,
        lease_epoch: i64,
    ) -> Result<Option<(i64, String, JsonValue)>, ()> {
        let Some(context) = self.rest_action_operations.get(operation_id) else {
            return Ok(None);
        };
        if context.mcp_session_id != mcp_session_id
            || context.instance_id != instance_id
            || context.session_id != session_id
            || context.lease_id != lease_id
            || context.lease_epoch != lease_epoch
        {
            return Err(());
        }
        Ok(Some((
            context.generation,
            context.state_id.clone(),
            context.action.clone(),
        )))
    }
}

fn operation_context_matches(
    existing: &RestActionOperationContext,
    requested: &RestActionOperationContext,
) -> bool {
    existing.mcp_session_id == requested.mcp_session_id
        && existing.instance_id == requested.instance_id
        && existing.session_id == requested.session_id
        && existing.lease_id == requested.lease_id
        && existing.lease_epoch == requested.lease_epoch
        && existing.generation == requested.generation
        && existing.state_id == requested.state_id
        && existing.action == requested.action
}
