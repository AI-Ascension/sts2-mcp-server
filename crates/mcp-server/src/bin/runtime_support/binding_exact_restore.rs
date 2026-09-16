// SPDX-License-Identifier: MIT

use super::super::RuntimeConfig;
use sts2_mcp_server::{
    EXACT_RESTORE_BEGIN_TOOL, EXACT_RESTORE_COMMIT_TOOL, EXACT_RESTORE_FINISH_BLOB_TOOL,
    EXACT_RESTORE_LOOKUP_TOOL, EXACT_RESTORE_PUT_CHUNK_TOOL, ExactRestoreRequestBinding,
    ExactRestoreTransportOwner, GatewayError, GatewayMethod, GatewayRequest,
    validate_exact_restore_request,
};

pub(crate) fn is_route(request: &GatewayRequest) -> bool {
    request.method == GatewayMethod::Post
        && request.body.is_some()
        && matches!(
            request.path.as_str(),
            "/v1/exact-restore/begin"
                | "/v1/exact-restore/chunk"
                | "/v1/exact-restore/finish"
                | "/v1/exact-restore/commit"
                | "/v1/exact-restore/lookup"
        )
}

pub(crate) fn validate(
    config: &RuntimeConfig,
    request: &GatewayRequest,
) -> Result<Option<ExactRestoreRequestBinding>, GatewayError> {
    if !is_route(request) {
        return Ok(None);
    }
    if !config.exact_restore_profile {
        return Err(GatewayError::Rejected);
    }
    let expected_tool = match request.path.as_str() {
        "/v1/exact-restore/begin" => EXACT_RESTORE_BEGIN_TOOL,
        "/v1/exact-restore/chunk" => EXACT_RESTORE_PUT_CHUNK_TOOL,
        "/v1/exact-restore/finish" => EXACT_RESTORE_FINISH_BLOB_TOOL,
        "/v1/exact-restore/commit" => EXACT_RESTORE_COMMIT_TOOL,
        "/v1/exact-restore/lookup" => EXACT_RESTORE_LOOKUP_TOOL,
        _ => return Err(GatewayError::Rejected),
    };
    let owner = ExactRestoreTransportOwner {
        instance_id: config.instance_id.clone(),
        session_id: config.session_id.clone(),
        lease_id: config.lease_id.clone(),
        lease_epoch: config.lease_epoch,
    };
    let body = request.body.as_ref().ok_or(GatewayError::Rejected)?;
    let binding = validate_exact_restore_request(body, expected_tool, &config.caller_id, &owner)
        .map_err(|_| GatewayError::Rejected)?;
    validate_headers(config, request)?;
    Ok(Some(binding))
}

fn validate_headers(config: &RuntimeConfig, request: &GatewayRequest) -> Result<(), GatewayError> {
    let lease_epoch = config.lease_epoch.to_string();
    let request_id = request.correlation.mcp_request_id.stable_text();
    for (header, expected) in [
        ("x-mcp-session-id", config.mcp_session_id.as_str()),
        ("x-mcp-correlation-id", request_id.as_str()),
        ("x-sts2-instance-id", config.instance_id.as_str()),
        ("x-sts2-session-id", config.session_id.as_str()),
        ("x-sts2-lease-id", config.lease_id.as_str()),
        ("x-sts2-lease-epoch", lease_epoch.as_str()),
    ] {
        if request.headers.get(header).map(String::as_str) != Some(expected) {
            return Err(GatewayError::Rejected);
        }
    }
    const ALLOWED_HEADERS: [&str; 6] = [
        "x-mcp-session-id",
        "x-mcp-correlation-id",
        "x-sts2-instance-id",
        "x-sts2-session-id",
        "x-sts2-lease-id",
        "x-sts2-lease-epoch",
    ];
    if request
        .headers
        .keys()
        .any(|header| !ALLOWED_HEADERS.contains(&header.as_str()))
    {
        return Err(GatewayError::Rejected);
    }
    if request.correlation.mcp_session_id != config.mcp_session_id {
        return Err(GatewayError::Rejected);
    }
    Ok(())
}
