// SPDX-License-Identifier: MIT

use crate::json::JsonValue;

pub const GET_STATE_TOOL: &str = "sts2_get_state";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolDescriptor {
    pub name: String,
    pub description: String,
    pub input_schema: JsonValue,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilityCatalog {
    pub tools: bool,
}

impl Default for CapabilityCatalog {
    fn default() -> Self {
        Self { tools: true }
    }
}

impl CapabilityCatalog {
    pub(crate) fn to_json(&self) -> JsonValue {
        let tools = if self.tools {
            JsonValue::Object(Default::default())
        } else {
            JsonValue::Null
        };
        JsonValue::object([(String::from("tools"), tools)])
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolCatalog {
    pub revision: String,
    pub capabilities: CapabilityCatalog,
    tools: Vec<ToolDescriptor>,
}

impl Default for ToolCatalog {
    fn default() -> Self {
        let schema = JsonValue::object([
            ("type".to_owned(), JsonValue::string("object")),
            (
                "required".to_owned(),
                JsonValue::Array(vec![
                    JsonValue::string("instance_id"),
                    JsonValue::string("mcp_session_id"),
                ]),
            ),
            (
                "properties".to_owned(),
                JsonValue::object([
                    (
                        "instance_id".to_owned(),
                        JsonValue::object([
                            ("type".to_owned(), JsonValue::string("string")),
                            ("minLength".to_owned(), JsonValue::Number(1)),
                        ]),
                    ),
                    (
                        "mcp_session_id".to_owned(),
                        JsonValue::object([
                            ("type".to_owned(), JsonValue::string("string")),
                            ("minLength".to_owned(), JsonValue::Number(1)),
                        ]),
                    ),
                ]),
            ),
        ]);
        Self {
            revision: String::from("wave2-local-v0"),
            capabilities: CapabilityCatalog::default(),
            tools: vec![ToolDescriptor {
                name: String::from(GET_STATE_TOOL),
                description: String::from(
                    "Read one bounded state snapshot through the authenticated gateway.",
                ),
                input_schema: schema,
            }],
        }
    }
}

impl ToolCatalog {
    pub(crate) fn descriptor(&self, name: &str) -> Option<&ToolDescriptor> {
        self.tools.iter().find(|tool| tool.name == name)
    }

    pub(crate) fn to_json(&self) -> JsonValue {
        let tools = self
            .tools
            .iter()
            .map(|tool| {
                JsonValue::object([
                    ("name".to_owned(), JsonValue::string(tool.name.as_str())),
                    (
                        "description".to_owned(),
                        JsonValue::string(tool.description.as_str()),
                    ),
                    ("inputSchema".to_owned(), tool.input_schema.clone()),
                ])
            })
            .collect();
        JsonValue::object([
            ("tools".to_owned(), JsonValue::Array(tools)),
            (
                "revision".to_owned(),
                JsonValue::string(self.revision.as_str()),
            ),
        ])
    }
}
