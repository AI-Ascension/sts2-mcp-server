// SPDX-License-Identifier: MIT

use super::{envelope, response};
use crate::gateway::GatewayError;
use crate::projection::RuntimeV2Context;
use crate::protocol::{RequestId, RpcResponse};

pub(super) fn uncertain_result(
    id: RequestId,
    context: &RuntimeV2Context,
    expected_kind: &str,
    error: GatewayError,
) -> RpcResponse {
    if expected_kind == "state_response" {
        return super::super::tool_result(
            id,
            "gateway state read unavailable; no observation was obtained",
            true,
        );
    }
    let error_code = match error {
        GatewayError::MalformedResponse => "sts2.runtime/unknown_after_invalid_response",
        GatewayError::Timeout | GatewayError::Unavailable => {
            "sts2.runtime/unknown_after_disconnect"
        }
        _ => "sts2.runtime/unknown",
    };
    let body = envelope::result_envelope(context, expected_kind, "unknown", error_code, None, None);
    response::gateway_success_v2(
        id,
        crate::gateway::GatewayResponse { status: 504, body },
        context,
        expected_kind,
    )
}

pub(super) fn gateway_error_result(id: RequestId, error: GatewayError) -> RpcResponse {
    let (code, message) = match error {
        GatewayError::Unauthorized => (-32001, "gateway authorization failed"),
        GatewayError::Forbidden => (-32007, "gateway scope authorization failed"),
        GatewayError::NotFound => (-32004, "gateway target was not found"),
        GatewayError::Unavailable => (-32003, "gateway is unavailable"),
        GatewayError::Timeout => (-32008, "gateway request timed out"),
        GatewayError::MalformedResponse => (-32002, "gateway returned an invalid response"),
        GatewayError::Rejected => (-32005, "gateway rejected the request"),
    };
    super::super::tool_result(id, format!("gateway error {code}: {message}"), true)
}
