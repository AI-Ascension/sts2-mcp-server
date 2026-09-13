// SPDX-License-Identifier: MIT

use std::collections::VecDeque;

use sts2_mcp_server::{
    GatewayAdapter, GatewayError, GatewayRequest, GatewayResponse, JsonValue, McpServer,
    SAVE_PROFILE_CONTRACT, SAVE_PROFILE_CREATE_DISPOSABLE_TOOL,
    SAVE_PROFILE_LAUNCH_PROFILE_CONTRACT, SAVE_PROFILE_MAX_BODY_BYTES, SAVE_PROFILE_SELECT_TOOL,
    ToolCatalog,
};

const DIGEST: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

pub struct FakeGateway {
    pub requests: Vec<GatewayRequest>,
    responses: VecDeque<Result<GatewayResponse, GatewayError>>,
}

impl FakeGateway {
    pub fn new(responses: impl IntoIterator<Item = Result<GatewayResponse, GatewayError>>) -> Self {
        Self {
            requests: Vec::new(),
            responses: responses.into_iter().collect(),
        }
    }
}

impl GatewayAdapter for FakeGateway {
    fn forward(&mut self, request: GatewayRequest) -> Result<GatewayResponse, GatewayError> {
        self.requests.push(request);
        self.responses
            .pop_front()
            .unwrap_or(Err(GatewayError::Unavailable))
    }
}

pub fn frame(id: &str, name: &str, arguments: JsonValue) -> String {
    JsonValue::object([
        (String::from("jsonrpc"), JsonValue::string("2.0")),
        (String::from("id"), JsonValue::string(id)),
        (String::from("method"), JsonValue::string("tools/call")),
        (
            String::from("params"),
            JsonValue::object([
                (String::from("name"), JsonValue::string(name)),
                (String::from("arguments"), arguments),
            ]),
        ),
    ])
    .to_json()
}

pub fn context() -> JsonValue {
    JsonValue::object([
        (String::from("instance_id"), JsonValue::string("instance-1")),
        (
            String::from("mcp_session_id"),
            JsonValue::string("mcp-session-1"),
        ),
        (String::from("lease_id"), JsonValue::string("lease-1")),
        (String::from("lease_epoch"), JsonValue::Number(7)),
    ])
}

pub fn select_arguments(profile_id: &str) -> JsonValue {
    let mut value = context();
    let JsonValue::Object(object) = &mut value else {
        panic!("context is an object");
    };
    object.insert(
        String::from("profile_id"),
        JsonValue::string(profile_id.to_owned()),
    );
    object.insert(String::from("baseline"), baseline("before"));
    value
}

pub fn status_arguments(operation_id: &str) -> JsonValue {
    let mut value = context();
    let JsonValue::Object(object) = &mut value else {
        panic!("context is an object");
    };
    object.insert(
        String::from("operation_id"),
        JsonValue::string(operation_id.to_owned()),
    );
    value
}

pub fn baseline(identity: &str) -> JsonValue {
    JsonValue::object([
        (String::from("identity"), JsonValue::string(identity)),
        (String::from("digest"), JsonValue::string(DIGEST)),
    ])
}

pub fn user_data(operation_id: &str) -> JsonValue {
    JsonValue::object([
        (String::from("identity"), JsonValue::Number(9)),
        (
            String::from("provenance"),
            JsonValue::object([
                (String::from("owner"), JsonValue::string("gateway")),
                (String::from("instance_id"), JsonValue::string("instance-1")),
                (
                    String::from("operation_id"),
                    JsonValue::string(operation_id),
                ),
                (
                    String::from("contract"),
                    JsonValue::string(SAVE_PROFILE_LAUNCH_PROFILE_CONTRACT),
                ),
            ]),
        ),
        (String::from("baseline"), JsonValue::Null),
    ])
}

pub fn result(operation_id: &str, route: &str, status: &str) -> JsonValue {
    let mut fields = vec![
        (
            String::from("contract"),
            JsonValue::string(SAVE_PROFILE_CONTRACT),
        ),
        (
            String::from("operation_id"),
            JsonValue::string(operation_id),
        ),
        (String::from("route"), JsonValue::string(route)),
        (String::from("status"), JsonValue::string(status)),
        (String::from("profile_id"), JsonValue::Null),
        (String::from("baseline"), JsonValue::Null),
        (String::from("user_data"), JsonValue::Null),
        (String::from("guidance"), JsonValue::Null),
        (String::from("downstream"), JsonValue::Array(Vec::new())),
    ];
    if route == "select" {
        fields[4] = (String::from("profile_id"), JsonValue::string("slot-2"));
        fields[5] = (String::from("baseline"), baseline("after"));
    }
    if route == "createdisposable" && matches!(status, "settled" | "created") {
        fields[5] = (String::from("baseline"), baseline("created"));
        fields[6] = (String::from("user_data"), user_data(operation_id));
    }
    JsonValue::object(fields)
}

pub fn wire(output: &str) -> serde_json::Value {
    serde_json::from_str(output).expect("MCP response JSON")
}

#[test]
fn malformed_mutation_response_becomes_unknown_with_reconciliation_metadata() {
    let mut malformed = result("select-malformed", "select", "settled");
    if let JsonValue::Object(object) = &mut malformed {
        object.remove("baseline");
    }
    let mut server = McpServer::with_catalog(
        FakeGateway::new([Ok(GatewayResponse {
            status: 200,
            body: malformed,
        })]),
        ToolCatalog::save_profile_v1(),
    );

    let output = wire(&server.handle_frame(&frame(
        "select-malformed",
        SAVE_PROFILE_SELECT_TOOL,
        select_arguments("slot-2"),
    )));

    assert_eq!(output["result"]["isError"], true);
    assert_eq!(
        output["result"]["structuredContent"]["error"]["code"],
        "save_profile_unknown_after_malformed_response"
    );
    assert!(output.to_string().contains("select-malformed"));
    assert!(
        output
            .to_string()
            .contains("save_profile_reconcile_required")
    );
    assert_eq!(server.gateway().requests.len(), 1);
}

#[test]
fn oversized_mutation_projection_becomes_unknown_with_reconciliation_metadata() {
    let mut oversized = result("select-oversized", "select", "rejected");
    if let JsonValue::Object(object) = &mut oversized {
        object.insert(
            String::from("downstream"),
            JsonValue::string("x".repeat(SAVE_PROFILE_MAX_BODY_BYTES - 64)),
        );
    }
    let mut server = McpServer::with_catalog(
        FakeGateway::new([Ok(GatewayResponse {
            status: 200,
            body: oversized,
        })]),
        ToolCatalog::save_profile_v1(),
    );

    let output = wire(&server.handle_frame(&frame(
        "select-oversized",
        SAVE_PROFILE_SELECT_TOOL,
        select_arguments("slot-2"),
    )));

    assert_eq!(output["result"]["isError"], true);
    assert_eq!(
        output["result"]["structuredContent"]["error"]["code"],
        "save_profile_unknown_after_oversized_response"
    );
    assert!(output.to_string().contains("select-oversized"));
    assert!(
        output
            .to_string()
            .contains("save_profile_reconcile_required")
    );
    assert_eq!(server.gateway().requests.len(), 1);
}

#[test]
fn mutation_tools_are_not_advertised_as_idempotent() {
    let mut server = McpServer::with_catalog(FakeGateway::new([]), ToolCatalog::save_profile_v1());
    let listed =
        wire(&server.handle_frame(r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#));

    for tool in listed["result"]["tools"].as_array().expect("tool array") {
        let name = tool["name"].as_str().expect("tool name");
        let expected =
            name != SAVE_PROFILE_SELECT_TOOL && name != SAVE_PROFILE_CREATE_DISPOSABLE_TOOL;
        assert_eq!(
            tool["annotations"]["idempotentHint"], expected,
            "unexpected idempotency annotation for {name}"
        );
    }
}
