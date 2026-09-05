// SPDX-License-Identifier: MIT

use crate::gateway::GatewayResponse;
use crate::json::JsonValue;
use crate::projection::{
    RuntimeV2Context, RuntimeV3GameplayProjectionContext, project_gateway_body,
    project_runtime_gateway_body, project_runtime_v2_gateway_body, project_runtime_v3_gateway_body,
    projection_is_error, runtime_v2_result_is_error, runtime_v3_result_is_error,
};
use crate::protocol::{RequestId, RpcResponse};

pub(super) const MAX_RESPONSE_BYTES: usize = 128 * 1024;

pub(super) fn gateway_success(
    id: RequestId,
    response: GatewayResponse,
    runtime_v1: bool,
) -> RpcResponse {
    let projection = if runtime_v1 {
        project_runtime_gateway_body(&response.body)
    } else {
        project_gateway_body(&response.body)
    };
    let Ok(projection) = projection else {
        return super::tool_result(
            id,
            "gateway response has no valid allowlisted state or error projection",
            true,
        );
    };
    let body = projection.to_json();
    if body.len() > MAX_RESPONSE_BYTES {
        return super::tool_result(id, "gateway returned an oversized response", true);
    }
    super::tool_result(
        id,
        body,
        !(200..300).contains(&response.status) || projection_is_error(&projection),
    )
}

pub(super) fn gateway_success_v2(
    id: RequestId,
    response: GatewayResponse,
    context: &RuntimeV2Context,
    expected_kind: &str,
) -> RpcResponse {
    if response.status == 429 {
        return gateway_overload(id, response);
    }
    let projection = project_runtime_v2_gateway_body(&response.body, context, expected_kind);
    let Ok(projection) = projection else {
        return super::tool_result(
            id,
            "gateway response is not a valid Runtime-v2 envelope",
            true,
        );
    };
    let body = projection.to_json();
    if body.len() > MAX_RESPONSE_BYTES {
        return super::tool_result(id, "gateway returned an oversized response", true);
    }
    super::tool_result(
        id,
        body,
        !(200..300).contains(&response.status) || runtime_v2_result_is_error(&projection),
    )
}

fn gateway_overload(id: RequestId, response: GatewayResponse) -> RpcResponse {
    let Some(object) = response.body.as_object() else {
        return super::tool_result(id, "gateway returned an invalid overload response", true);
    };
    let Some(error_code) = object.get("error_code").and_then(JsonValue::as_string) else {
        return super::tool_result(id, "gateway returned an invalid overload response", true);
    };
    if error_code.is_empty()
        || error_code.len() > 128
        || !error_code.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':' | b'/')
        })
        || object.get("retryable") != Some(&JsonValue::Bool(true))
    {
        return super::tool_result(id, "gateway returned an invalid overload response", true);
    }
    let Some(JsonValue::Number(retry_after_ms)) = object.get("retry_after_ms") else {
        return super::tool_result(id, "gateway returned an invalid overload response", true);
    };
    if !(0..=60_000).contains(retry_after_ms) {
        return super::tool_result(id, "gateway returned an invalid overload response", true);
    }
    let body = JsonValue::object([
        ("error_code".to_owned(), JsonValue::string(error_code)),
        ("retryable".to_owned(), JsonValue::Bool(true)),
        (
            "retry_after_ms".to_owned(),
            JsonValue::Number(*retry_after_ms),
        ),
    ])
    .to_json();
    super::tool_result(id, body, true)
}

pub(super) fn gateway_success_v3(
    id: RequestId,
    response: GatewayResponse,
    context: &RuntimeV3GameplayProjectionContext,
    expected_kind: &str,
) -> RpcResponse {
    let projection = project_runtime_v3_gateway_body(&response.body, context, expected_kind);
    let Ok(projection) = projection else {
        return super::tool_result(
            id,
            "gateway response is not a valid Runtime-v3 gameplay envelope",
            true,
        );
    };
    let body = projection.to_json();
    if body.len() > MAX_RESPONSE_BYTES {
        return super::tool_result(id, "gateway returned an oversized response", true);
    }
    super::tool_result(
        id,
        body,
        !(200..300).contains(&response.status) || runtime_v3_result_is_error(&projection),
    )
}
