// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::catalog::{
    EXACT_RESTORE_BEGIN_TOOL, EXACT_RESTORE_COMMIT_TOOL, EXACT_RESTORE_FINISH_BLOB_TOOL,
    EXACT_RESTORE_LOOKUP_TOOL, EXACT_RESTORE_PUT_CHUNK_TOOL,
};
use crate::exact_restore::{
    ExactRestoreTransportOwner, validate_exact_restore_request, validate_exact_restore_response,
};
use crate::gateway::{Correlation, GatewayAdapter, GatewayMethod, GatewayRequest};
use crate::json::JsonValue;
use crate::protocol::{METHOD_NOT_FOUND, RpcError, RpcRequest, RpcResponse};
use crate::server::McpServer;

use super::{
    gateway_error_result, has_only_arguments, invalid_params, safe_header_value, tool_result,
};

const TOOLS: [&str; 5] = [
    EXACT_RESTORE_BEGIN_TOOL,
    EXACT_RESTORE_PUT_CHUNK_TOOL,
    EXACT_RESTORE_FINISH_BLOB_TOOL,
    EXACT_RESTORE_COMMIT_TOOL,
    EXACT_RESTORE_LOOKUP_TOOL,
];

pub(super) fn tools_call<G: GatewayAdapter>(
    server: &mut McpServer<G>,
    request: RpcRequest,
) -> RpcResponse {
    let Some(params) = request.params.as_object() else {
        return invalid_params(
            request.id,
            "exact-restore tools/call params must be an object",
        );
    };
    if !has_only_arguments(params, &["name", "arguments"]) {
        return invalid_params(
            request.id,
            "exact-restore tools/call has unsupported fields",
        );
    }
    let Some(tool) = params.get("name").and_then(JsonValue::as_string) else {
        return invalid_params(request.id, "exact-restore tool name is missing");
    };
    if !TOOLS.contains(&tool) || server.catalog.descriptor(tool).is_none() {
        return RpcResponse::failure(
            Some(request.id),
            RpcError::new(METHOD_NOT_FOUND, "exact-restore tool is not active"),
        );
    }
    let Some(envelope) = params.get("arguments") else {
        return invalid_params(request.id, "exact-restore arguments are missing");
    };
    let principal = match member_string(envelope, &["actor", "principal_id"]) {
        Some(value) => value,
        None => return invalid_params(request.id, "exact-restore wrapper actor is missing"),
    };
    let owner = match owner_from_envelope(envelope) {
        Some(owner) => owner,
        None => return invalid_params(request.id, "exact-restore owner identity is missing"),
    };
    let binding = match validate_exact_restore_request(envelope, tool, principal, &owner) {
        Ok(binding) => binding,
        Err(message) => return invalid_params(request.id, message),
    };
    let id = request.id;
    let correlation_id = id.stable_text();
    if !safe_header_value(&correlation_id) {
        return invalid_params(id, "MCP correlation ID is unsafe or oversized");
    }
    let headers = transport_headers(&binding, server.mcp_session_id(), &correlation_id);
    let gateway_request = GatewayRequest {
        method: GatewayMethod::Post,
        path: format!("/v1/exact-restore/{}", binding.phase.route()),
        headers,
        body: Some(envelope.clone()),
        correlation: Correlation {
            mcp_session_id: server
                .mcp_session_id()
                .unwrap_or("mcp-session-1")
                .to_owned(),
            mcp_request_id: id.clone(),
        },
    };
    match server.forward_gateway(gateway_request) {
        Ok(response) => {
            match validate_exact_restore_response(&response.body, &binding, principal) {
                Ok(validated) => {
                    let is_error = !(200..300).contains(&response.status)
                        || crate::exact_restore::string_member(&validated.frame, "kind")
                            == Some("exact_restore_error_response");
                    tool_result(id, validated.frame.to_json(), is_error)
                }
                Err(message) => tool_result(id, message, true),
            }
        }
        Err(error) => gateway_error_result(id, error),
    }
}

fn owner_from_envelope(envelope: &JsonValue) -> Option<ExactRestoreTransportOwner> {
    let frame = envelope
        .as_object()?
        .get("payload")?
        .as_object()?
        .get("frame")?;
    let owner = frame
        .as_object()?
        .get("payload")?
        .as_object()?
        .get("expected_owner")?
        .as_object()?;
    Some(ExactRestoreTransportOwner {
        instance_id: owner.get("instance_id")?.as_string()?.to_owned(),
        session_id: owner.get("session_id")?.as_string()?.to_owned(),
        lease_id: owner.get("lease_id")?.as_string()?.to_owned(),
        lease_epoch: match owner.get("lease_epoch")? {
            JsonValue::Number(value) => *value,
            _ => return None,
        },
    })
}

fn member_string<'a>(value: &'a JsonValue, path: &[&str]) -> Option<&'a str> {
    let mut current = value;
    for member in path {
        current = current.as_object()?.get(*member)?;
    }
    current.as_string()
}

fn transport_headers(
    binding: &crate::exact_restore::ExactRestoreRequestBinding,
    mcp_session: Option<&str>,
    mcp_correlation: &str,
) -> BTreeMap<String, String> {
    let mut headers = BTreeMap::from([
        (
            String::from("x-mcp-session-id"),
            mcp_session.unwrap_or("mcp-session-1").to_owned(),
        ),
        (
            String::from("x-mcp-correlation-id"),
            mcp_correlation.to_owned(),
        ),
    ]);
    if let Some(owner) = binding.expected_owner.as_object() {
        for (field, header) in [
            ("instance_id", "x-sts2-instance-id"),
            ("session_id", "x-sts2-session-id"),
            ("lease_id", "x-sts2-lease-id"),
        ] {
            if let Some(value) = owner.get(field).and_then(JsonValue::as_string) {
                headers.insert(String::from(header), value.to_owned());
            }
        }
        if let Some(JsonValue::Number(epoch)) = owner.get("lease_epoch") {
            headers.insert(String::from("x-sts2-lease-epoch"), epoch.to_string());
        }
    }
    headers
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_phase_tools_have_a_fixed_phase_route() {
        let phases = [
            (
                EXACT_RESTORE_BEGIN_TOOL,
                crate::exact_restore::ExactRestorePhase::Begin,
            ),
            (
                EXACT_RESTORE_PUT_CHUNK_TOOL,
                crate::exact_restore::ExactRestorePhase::PutChunk,
            ),
            (
                EXACT_RESTORE_FINISH_BLOB_TOOL,
                crate::exact_restore::ExactRestorePhase::FinishBlob,
            ),
            (
                EXACT_RESTORE_COMMIT_TOOL,
                crate::exact_restore::ExactRestorePhase::Commit,
            ),
            (
                EXACT_RESTORE_LOOKUP_TOOL,
                crate::exact_restore::ExactRestorePhase::Lookup,
            ),
        ];
        for (tool, phase) in phases {
            assert!(TOOLS.contains(&tool));
            assert_eq!(phase.tool(), tool);
        }
    }
}
