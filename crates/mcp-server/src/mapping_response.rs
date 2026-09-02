// SPDX-License-Identifier: MIT

use crate::gateway::GatewayResponse;
use crate::projection::{project_gateway_body, project_runtime_gateway_body, projection_is_error};
use crate::protocol::{RequestId, RpcResponse};

const MAX_RESPONSE_BYTES: usize = 16 * 1024;

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
