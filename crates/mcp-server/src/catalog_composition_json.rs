// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::json::JsonValue;

use super::types::{CapabilityGroup, NegotiatedCapabilitySet, ToolLimits};

impl NegotiatedCapabilitySet {
    pub fn to_json(&self) -> JsonValue {
        let mut groups = BTreeMap::new();
        for group in CapabilityGroup::ALL {
            let available: Vec<JsonValue> = self
                .operations
                .values()
                .filter(|operation| operation.group == group)
                .map(|operation| JsonValue::string(operation.operation.clone()))
                .collect();
            let unavailable: Vec<JsonValue> = self
                .unavailable
                .values()
                .filter(|operation| operation.group == group)
                .map(|operation| {
                    JsonValue::object([
                        (
                            "operation".into(),
                            JsonValue::string(operation.operation.clone()),
                        ),
                        (
                            "reason".into(),
                            JsonValue::string(operation.reason.as_str()),
                        ),
                    ])
                })
                .collect();
            groups.insert(
                group.as_str().to_owned(),
                JsonValue::object([
                    ("available".into(), JsonValue::Array(available)),
                    ("unavailable".into(), JsonValue::Array(unavailable)),
                ]),
            );
        }
        JsonValue::object([
            ("revision".into(), JsonValue::string(self.revision.clone())),
            ("features".into(), JsonValue::Object(groups)),
            (
                "capability_discovery".into(),
                JsonValue::object([
                    ("read_only".into(), JsonValue::Bool(true)),
                    ("forwards".into(), JsonValue::Bool(false)),
                    ("scope".into(), JsonValue::string("caller_intersection")),
                ]),
            ),
        ])
    }
}

impl ToolLimits {
    pub fn to_json(self) -> JsonValue {
        JsonValue::object([
            ("max_request_bytes".into(), number(self.max_request_bytes)),
            ("max_response_bytes".into(), number(self.max_response_bytes)),
            ("max_content_bytes".into(), number(self.max_content_bytes)),
            ("max_page_items".into(), number(self.max_page_items)),
        ])
    }
}

pub(crate) fn composition_metadata(set: &NegotiatedCapabilitySet) -> JsonValue {
    set.to_json()
}

fn number(value: usize) -> JsonValue {
    JsonValue::Number(i64::try_from(value).unwrap_or(i64::MAX))
}
