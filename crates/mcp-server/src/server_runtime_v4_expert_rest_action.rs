// SPDX-License-Identifier: MIT

use crate::gateway::GatewayAdapter;
use crate::json::JsonValue;

use super::McpServer;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RestActionOperationContext {
    pub(crate) mcp_session_id: String,
    pub(crate) instance_id: String,
    pub(crate) session_id: String,
    pub(crate) lease_id: String,
    pub(crate) lease_epoch: i64,
    pub(crate) generation: i64,
    pub(crate) action: JsonValue,
}

impl<G: GatewayAdapter> McpServer<G> {
    pub(crate) fn remember_rest_action_operation(
        &mut self,
        operation_id: &str,
        context: RestActionOperationContext,
    ) -> bool {
        match self.rest_action_operations.get(operation_id) {
            Some(existing) => existing == &context,
            None => {
                self.rest_action_operations
                    .insert(operation_id.to_owned(), context);
                true
            }
        }
    }

    pub(crate) fn rest_action_operation_binding(
        &self,
        operation_id: &str,
        mcp_session_id: &str,
        instance_id: &str,
        session_id: &str,
        lease_id: &str,
        lease_epoch: i64,
    ) -> Result<Option<(i64, JsonValue)>, ()> {
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
        Ok(Some((context.generation, context.action.clone())))
    }
}
