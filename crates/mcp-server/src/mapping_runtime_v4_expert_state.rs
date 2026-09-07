// SPDX-License-Identifier: MIT

fn expert_state_call<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    id: RequestId,
    arguments: &BTreeMap<String, JsonValue>,
    correlation_id: &str,
) -> RpcResponse {
    if !has_only_arguments(arguments, &ARGUMENTS) {
        return invalid_params(
            id,
            "sts2.expert_state arguments contain an unsupported field",
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
    if !super::safe_segment(instance_id) || !super::safe_header_value(mcp_session_id) {
        return invalid_params(id, "expert-state identity is unsafe or oversized");
    }
    if server
        .mcp_session_id()
        .is_some_and(|expected| expected != mcp_session_id)
    {
        return invalid_params(
            id,
            "MCP session identity does not match the configured session",
        );
    }
    let request = GatewayRequest {
        method: GatewayMethod::Get,
        path: format!("/v4/instances/{instance_id}/expert-state"),
        headers: headers(mcp_session_id, correlation_id),
        body: None,
        correlation: Correlation {
            mcp_session_id: String::from(mcp_session_id),
            mcp_request_id: id.clone(),
        },
    };
    match server.gateway.forward(request) {
        Ok(response) if (200..300).contains(&response.status) => {
            let Ok(body) =
                crate::projection::project_runtime_v4_expert_gateway_body(&response.body)
            else {
                return tool_result(
                    id,
                    "gateway response is not a valid Runtime-v4 expert state",
                    true,
                );
            };
            let text = body.to_json();
            if text.len() > super::response::RUNTIME_V4_EXPERT_MAX_RESPONSE_BYTES {
                return tool_result(
                    id,
                    "gateway returned an oversized expert-state response",
                    true,
                );
            }
            tool_result(id, text, false)
        }
        Ok(response) => expert_error_result(id, response.status, &response.body),
        Err(error) => gateway_error_result(id, error),
    }
}

