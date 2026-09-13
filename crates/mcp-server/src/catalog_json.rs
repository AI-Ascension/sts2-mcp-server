// SPDX-License-Identifier: MIT

use crate::json::JsonValue;

use super::{ToolCatalog, game_information, save_profile};

impl ToolCatalog {
    pub(crate) fn to_json(&self) -> JsonValue {
        let tools = self
            .tools
            .iter()
            .map(|tool| {
                let mut descriptor = JsonValue::object([
                    ("name".to_owned(), JsonValue::string(tool.name.as_str())),
                    (
                        "description".to_owned(),
                        JsonValue::string(tool.description.as_str()),
                    ),
                    ("inputSchema".to_owned(), tool.input_schema.clone()),
                ]);
                if (game_information::is_tool(&tool.name)
                    || save_profile::is_tool(&tool.name)
                    || tool.name == super::composition::CAPABILITY_DISCOVERY_TOOL)
                    && let JsonValue::Object(object) = &mut descriptor
                {
                    let read_only = game_information::is_tool(&tool.name)
                        || save_profile::is_read_tool(&tool.name)
                        || tool.name == super::composition::CAPABILITY_DISCOVERY_TOOL;
                    object.insert(
                        String::from("annotations"),
                        JsonValue::object([
                            ("readOnlyHint".to_owned(), JsonValue::Bool(read_only)),
                            ("destructiveHint".to_owned(), JsonValue::Bool(false)),
                            ("idempotentHint".to_owned(), JsonValue::Bool(read_only)),
                            ("openWorldHint".to_owned(), JsonValue::Bool(false)),
                        ]),
                    );
                }
                if let Some(composition) = &self.composition
                    && let Some(operation) = composition.available(&tool.name)
                    && let JsonValue::Object(object) = &mut descriptor
                {
                    object.insert(
                        String::from("_meta"),
                        JsonValue::object([(
                            String::from("sts2"),
                            JsonValue::object([
                                (
                                    String::from("feature"),
                                    JsonValue::string(operation.group.as_str()),
                                ),
                                (
                                    String::from("revision"),
                                    JsonValue::string(operation.revision.clone()),
                                ),
                                (
                                    String::from("scope"),
                                    JsonValue::Array(operation.effective_scope.names()),
                                ),
                                (String::from("limits"), operation.limits.to_json()),
                                (String::from("availability"), JsonValue::string("available")),
                            ]),
                        )]),
                    );
                }
                descriptor
            })
            .collect();
        let mut result = JsonValue::object([
            ("tools".to_owned(), JsonValue::Array(tools)),
            (
                "revision".to_owned(),
                JsonValue::string(self.revision.as_str()),
            ),
        ]);
        if let Some(composition) = &self.composition
            && let JsonValue::Object(object) = &mut result
        {
            object.insert(
                String::from("composition"),
                super::composition::composition_metadata(composition),
            );
        }
        result
    }
}
