// SPDX-License-Identifier: MIT

use super::super::RuntimeConfig;
use sts2_mcp_server::{
    GatewayError, GatewayMethod, GatewayRequest, RecoveryOperation, validate_recovery_request,
};

/// One admitted recovery sideband exchange.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RecoveryRequestBinding {
    pub(crate) operation: RecoveryOperation,
    pub(crate) correlation_id: String,
}

/// The recovery routes carry no instance prefix, so they are matched exactly.
pub(crate) fn is_route(request: &GatewayRequest) -> bool {
    request.method == GatewayMethod::Post
        && request.body.is_some()
        && RecoveryOperation::from_path(&request.path).is_some()
}

pub(crate) fn validate(
    config: &RuntimeConfig,
    request: &GatewayRequest,
) -> Result<Option<RecoveryRequestBinding>, GatewayError> {
    if !is_route(request) {
        return Ok(None);
    }
    if !config.recovery_profile {
        return Err(GatewayError::Rejected);
    }
    let operation = RecoveryOperation::from_path(&request.path).ok_or(GatewayError::Rejected)?;
    let body = request.body.as_ref().ok_or(GatewayError::Rejected)?;
    // The frame must already be the closed request the gateway accepts, bound
    // to the configured caller. A frame that names any other principal is
    // refused here rather than being rewritten.
    let correlation_id = validate_recovery_request(body, operation, &config.caller_id)
        .map_err(|_| GatewayError::Rejected)?;
    validate_headers(config, request, &correlation_id)?;
    Ok(Some(RecoveryRequestBinding {
        operation,
        correlation_id,
    }))
}

fn validate_headers(
    config: &RuntimeConfig,
    request: &GatewayRequest,
    correlation_id: &str,
) -> Result<(), GatewayError> {
    if request.headers.get("x-mcp-session-id").map(String::as_str)
        != Some(config.mcp_session_id.as_str())
        || request
            .headers
            .get("x-sts2-correlation-id")
            .map(String::as_str)
            != Some(correlation_id)
        || request.correlation.mcp_session_id != config.mcp_session_id
    {
        return Err(GatewayError::Rejected);
    }
    // The caller, instance, session, lease, and capability headers are injected
    // from configuration after admission, so no caller-supplied header may
    // select or override them.
    const ALLOWED_HEADERS: [&str; 2] = ["x-mcp-session-id", "x-sts2-correlation-id"];
    if request
        .headers
        .keys()
        .any(|header| !ALLOWED_HEADERS.contains(&header.as_str()))
    {
        return Err(GatewayError::Rejected);
    }
    Ok(())
}

#[cfg(test)]
#[path = "binding_recovery_tests.rs"]
mod tests;
