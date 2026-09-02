// SPDX-License-Identifier: MIT

use crate::json::JsonValue;

pub const GET_STATE_TOOL: &str = "get_state";
pub const SUBMIT_ACTION_TOOL: &str = "submit_action";
pub(crate) const MAX_IDENTIFIER_BYTES: usize = 128;
const INSTANCE_ID_PATTERN: &str = "^[A-Za-z0-9_-]{1,128}$";
const SESSION_ID_PATTERN: &str = "^[A-Za-z0-9_.:/-]{1,128}$";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolDescriptor {
    pub name: String,
    pub description: String,
    pub input_schema: JsonValue,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilityCatalog {
    pub supports_tools: bool,
}

impl Default for CapabilityCatalog {
    fn default() -> Self {
        Self {
            supports_tools: true,
        }
    }
}

impl CapabilityCatalog {
    pub(crate) fn to_json(&self) -> JsonValue {
        let tools = if self.supports_tools {
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
        let state_schema = JsonValue::object([
            ("type".to_owned(), JsonValue::string("object")),
            ("additionalProperties".to_owned(), JsonValue::Bool(false)),
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
                            (
                                "maxLength".to_owned(),
                                JsonValue::Number(MAX_IDENTIFIER_BYTES as i64),
                            ),
                            ("pattern".to_owned(), JsonValue::string(INSTANCE_ID_PATTERN)),
                        ]),
                    ),
                    (
                        "mcp_session_id".to_owned(),
                        JsonValue::object([
                            ("type".to_owned(), JsonValue::string("string")),
                            ("minLength".to_owned(), JsonValue::Number(1)),
                            (
                                "maxLength".to_owned(),
                                JsonValue::Number(MAX_IDENTIFIER_BYTES as i64),
                            ),
                            ("pattern".to_owned(), JsonValue::string(SESSION_ID_PATTERN)),
                        ]),
                    ),
                ]),
            ),
        ]);
        let action_schema = JsonValue::object([
            ("type".to_owned(), JsonValue::string("object")),
            ("additionalProperties".to_owned(), JsonValue::Bool(false)),
            (
                "required".to_owned(),
                JsonValue::Array(vec![
                    JsonValue::string("instance_id"),
                    JsonValue::string("mcp_session_id"),
                    JsonValue::string("generation"),
                    JsonValue::string("action_id"),
                    JsonValue::string("units"),
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
                            (
                                "maxLength".to_owned(),
                                JsonValue::Number(MAX_IDENTIFIER_BYTES as i64),
                            ),
                            ("pattern".to_owned(), JsonValue::string(INSTANCE_ID_PATTERN)),
                        ]),
                    ),
                    (
                        "mcp_session_id".to_owned(),
                        JsonValue::object([
                            ("type".to_owned(), JsonValue::string("string")),
                            ("minLength".to_owned(), JsonValue::Number(1)),
                            (
                                "maxLength".to_owned(),
                                JsonValue::Number(MAX_IDENTIFIER_BYTES as i64),
                            ),
                            ("pattern".to_owned(), JsonValue::string(SESSION_ID_PATTERN)),
                        ]),
                    ),
                    (
                        "generation".to_owned(),
                        JsonValue::object([
                            ("type".to_owned(), JsonValue::string("integer")),
                            ("minimum".to_owned(), JsonValue::Number(0)),
                        ]),
                    ),
                    (
                        "action_id".to_owned(),
                        JsonValue::object([
                            ("type".to_owned(), JsonValue::string("string")),
                            ("const".to_owned(), JsonValue::string("use_budget")),
                        ]),
                    ),
                    (
                        "units".to_owned(),
                        JsonValue::object([
                            ("type".to_owned(), JsonValue::string("integer")),
                            ("minimum".to_owned(), JsonValue::Number(0)),
                            ("maximum".to_owned(), JsonValue::Number(8)),
                        ]),
                    ),
                ]),
            ),
        ]);
        Self {
            revision: String::from("poc-v1-mcp"),
            capabilities: CapabilityCatalog::default(),
            tools: vec![
                ToolDescriptor {
                    name: String::from(GET_STATE_TOOL),
                    description: String::from(
                        "Read one bounded state snapshot through the authenticated gateway.",
                    ),
                    input_schema: state_schema,
                },
                ToolDescriptor {
                    name: String::from(SUBMIT_ACTION_TOOL),
                    description: String::from(
                        "Submit one typed use_budget action through the authenticated gateway.",
                    ),
                    input_schema: action_schema,
                },
            ],
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
