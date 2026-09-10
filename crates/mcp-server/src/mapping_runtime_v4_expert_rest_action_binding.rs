// SPDX-License-Identifier: MIT

use crate::json::JsonValue;

#[derive(Clone, Debug)]
pub(crate) struct RestResponseBinding {
    pub(crate) correlation_id: String,
    pub(crate) instance_id: String,
    pub(crate) session_id: String,
    pub(crate) lease_id: String,
    pub(crate) lease_epoch: i64,
    pub(crate) generation: Option<i64>,
    pub(crate) state_id: Option<String>,
    pub(crate) operation_id: String,
    pub(crate) action: Option<JsonValue>,
}

impl RestResponseBinding {
    pub(super) fn matches(&self, body: &JsonValue) -> bool {
        let Some(root) = body.as_object() else {
            return false;
        };
        for (field, expected) in [
            ("correlation_id", self.correlation_id.as_str()),
            ("instance_id", self.instance_id.as_str()),
            ("session_id", self.session_id.as_str()),
            ("lease_id", self.lease_id.as_str()),
            ("operation_id", self.operation_id.as_str()),
        ] {
            if root.get(field).and_then(JsonValue::as_string) != Some(expected) {
                return false;
            }
        }
        if root.get("lease_epoch") != Some(&JsonValue::Number(self.lease_epoch)) {
            return false;
        }
        if self.state_id.as_ref().is_some_and(|expected_state_id| {
            root.get("status").and_then(JsonValue::as_string) != Some("settled")
                && root.get("state_id") != Some(&JsonValue::String(expected_state_id.clone()))
        }) {
            return false;
        }
        if let Some(expected_action) = self.action.as_ref()
            && root.get("action") != Some(expected_action)
        {
            return false;
        }
        if let Some(expected_generation) = self.generation {
            if root.get("generation") != Some(&JsonValue::Number(expected_generation))
                && root.get("status").and_then(JsonValue::as_string) != Some("settled")
            {
                return false;
            }
            if root.get("status").and_then(JsonValue::as_string) == Some("settled") {
                let before = root
                    .get("transition")
                    .and_then(JsonValue::as_object)
                    .and_then(|transition| transition.get("before_generation"));
                if before != Some(&JsonValue::Number(expected_generation)) {
                    return false;
                }
            }
        }
        true
    }
}
