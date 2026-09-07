// SPDX-License-Identifier: MIT

#[derive(Clone, Debug)]
struct ExpertResponseBinding {
    correlation_id: String,
    instance_id: String,
    session_id: String,
    lease_id: String,
    lease_epoch: i64,
    generation: Option<i64>,
    operation_id: String,
    action: Option<JsonValue>,
}

impl ExpertResponseBinding {
    fn matches(&self, body: &JsonValue) -> bool {
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
        if let Some(expected_action) = self.action.as_ref() {
            let Some(response_action) = root.get("action") else {
                return false;
            };
            if !matches!(response_action, JsonValue::Null) && response_action != expected_action {
                return false;
            }
        }
        if let Some(expected_generation) = self.generation
            && root.get("status").and_then(JsonValue::as_string) == Some("settled")
        {
            let before = root
                .get("transition")
                .and_then(JsonValue::as_object)
                .and_then(|transition| transition.get("before_generation"));
            if before != Some(&JsonValue::Number(expected_generation)) {
                return false;
            }
        }
        true
    }
}

fn expert_action_call<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    id: RequestId,
    arguments: &BTreeMap<String, JsonValue>,
    correlation_id: &str,
) -> RpcResponse {
    if !has_only_arguments(arguments, &ACTION_ARGUMENTS) {
        return invalid_params(
            id,
            "sts2.expert_action arguments contain an unsupported field",
        );
    }
    let Some(instance_id) = arguments.get("instance_id").and_then(JsonValue::as_string) else {
        return invalid_params(id, "instance_id must be a non-empty string");
    };
    let Some(mcp_session_id) = arguments
        .get("mcp_session_id")
        .and_then(JsonValue::as_string)
    else {
        return invalid_params(id, "mcp_session_id must be a non-empty string");
    };
    let Some(lease_id) = arguments.get("lease_id").and_then(JsonValue::as_string) else {
        return invalid_params(id, "lease_id must be a non-empty string");
    };
    if !super::safe_segment(instance_id)
        || !super::safe_header_value(mcp_session_id)
        || !super::safe_header_value(lease_id)
        || server
            .mcp_session_id()
            .is_some_and(|expected| expected != mcp_session_id)
    {
        return invalid_params(
            id,
            "expert-action identity is unsafe or not bound to this MCP session",
        );
    }
    let Some(lease_epoch) = super::nonnegative_integer(arguments, "lease_epoch")
        .filter(|value| *value <= 9_007_199_254_740_991)
    else {
        return invalid_params(id, "lease_epoch exceeds the protocol bound");
    };
    let Some(generation) = super::nonnegative_integer(arguments, "generation")
        .filter(|value| *value <= 9_007_199_254_740_991)
    else {
        return invalid_params(id, "generation exceeds the protocol bound");
    };
    let Some(state_id) = arguments
        .get("state_id")
        .and_then(JsonValue::as_string)
        .filter(|value| super::safe_header_value(value))
    else {
        return invalid_params(id, "state_id must be a safe non-empty identity");
    };
    let Some(operation_id) = arguments
        .get("operation_id")
        .and_then(JsonValue::as_string)
        .filter(|value| super::safe_header_value(value) && !value.contains('/'))
    else {
        return invalid_params(id, "operation_id must be a safe non-empty identity");
    };
    let Some(action) = arguments.get("action") else {
        return invalid_params(id, "action must be a host-generated potion legal action");
    };
    if !valid_potion_action(action) {
        return invalid_params(
            id,
            "action must contain exactly one use_potion legal action",
        );
    }
    let body = JsonValue::object([
        (
            "protocol_version".into(),
            JsonValue::string(RUNTIME_V4_EXPERT_ACTION_PROTOCOL_VERSION),
        ),
        (
            "schema_digest".into(),
            JsonValue::string(RUNTIME_V4_EXPERT_ACTION_SCHEMA_DIGEST),
        ),
        (
            "provenance".into(),
            JsonValue::object([
                (
                    "artifact".into(),
                    JsonValue::string(RUNTIME_V4_EXPERT_ACTION_ARTIFACT),
                ),
                (
                    "source".into(),
                    JsonValue::string(RUNTIME_V4_EXPERT_ACTION_SCHEMA_SOURCE),
                ),
                (
                    "generator".into(),
                    JsonValue::string(RUNTIME_V4_EXPERT_ACTION_GENERATOR),
                ),
            ]),
        ),
        ("profile".into(), JsonValue::string("expert-action")),
        ("correlation_id".into(), JsonValue::string(correlation_id)),
        ("instance_id".into(), JsonValue::string(instance_id)),
        (
            "session_id".into(),
            JsonValue::string(server.gateway_session_id().unwrap_or(mcp_session_id)),
        ),
        ("lease_id".into(), JsonValue::string(lease_id)),
        ("lease_epoch".into(), JsonValue::Number(lease_epoch)),
        ("generation".into(), JsonValue::Number(generation)),
        ("state_id".into(), JsonValue::string(state_id)),
        ("operation_id".into(), JsonValue::string(operation_id)),
        ("kind".into(), JsonValue::string("action_request")),
        ("action".into(), action.clone()),
        ("status".into(), JsonValue::Null),
        ("observation".into(), JsonValue::Null),
        ("transition".into(), JsonValue::Null),
        ("error_code".into(), JsonValue::Null),
    ]);
    let request = GatewayRequest {
        method: GatewayMethod::Post,
        path: format!("/v4/instances/{instance_id}/expert-action"),
        headers: headers(mcp_session_id, correlation_id),
        body: Some(body),
        correlation: Correlation {
            mcp_session_id: String::from(mcp_session_id),
            mcp_request_id: id.clone(),
        },
    };
    let binding = ExpertResponseBinding {
        correlation_id: correlation_id.to_owned(),
        instance_id: instance_id.to_owned(),
        session_id: server.gateway_session_id().unwrap_or(mcp_session_id).to_owned(),
        lease_id: lease_id.to_owned(),
        lease_epoch,
        generation: Some(generation),
        operation_id: operation_id.to_owned(),
        action: Some(action.clone()),
    };
    match server.gateway.forward(request) {
        Ok(response) => expert_action_response(id, response, binding),
        Err(error) => gateway_error_result(id, error),
    }
}

fn expert_action_response(
    id: RequestId,
    response: crate::gateway::GatewayResponse,
    binding: ExpertResponseBinding,
) -> RpcResponse {
    if !(response.status == 200 || response.status == 409 || response.status == 503)
        || crate::projection::project_runtime_v4_expert_action_gateway_body(&response.body).is_err()
        || !binding.matches(&response.body)
    {
        return expert_error_result(id, response.status, &response.body);
    }
    tool_result(id, response.body.to_json(), response.status != 200)
}
