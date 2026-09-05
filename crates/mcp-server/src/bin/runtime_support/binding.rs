// SPDX-License-Identifier: MIT

use super::{RuntimeConfig, safe_header_value};
use sts2_mcp_server::{GatewayError, GatewayMethod, GatewayRequest, JsonValue};

pub(super) fn admit(config: &RuntimeConfig, request: &GatewayRequest) -> Result<(), GatewayError> {
    if request.correlation.mcp_session_id != config.mcp_session_id
        || request.headers.get("x-mcp-session-id").map(String::as_str)
            != Some(config.mcp_session_id.as_str())
    {
        return Err(GatewayError::Rejected);
    }
    let version = request
        .path
        .split('/')
        .nth(1)
        .ok_or(GatewayError::Rejected)?;
    if !matches!(version, "v1" | "v2" | "v3")
        || !request
            .path
            .starts_with(&format!("/{version}/instances/{}/", config.instance_id))
        || !safe_header_value(&request.correlation.mcp_request_id.stable_text())
    {
        return Err(GatewayError::Rejected);
    }
    if version != "v3" && response_kind(config, request).is_none() {
        return Err(GatewayError::Rejected);
    }
    // Runtime-v1 retains its documented configured identity injection. Newer profiles
    // must not silently substitute authority, including for bodyless observation calls.
    if version != "v1" {
        // MCP correlation sessions are a separate namespace; only explicit gateway
        // authority headers/body fields are compared with configured gateway identity.
        for (name, expected) in [
            ("x-sts2-instance-id", config.instance_id.as_str()),
            ("x-sts2-session-id", config.session_id.as_str()),
            ("x-sts2-lease-id", config.lease_id.as_str()),
            ("x-sts2-lease-epoch", &config.lease_epoch.to_string()),
        ] {
            let supplied = request.headers.get(name);
            if supplied.is_some_and(|value| value != expected)
                || (request.body.is_none() && supplied.is_none())
            {
                return Err(GatewayError::Rejected);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "binding_tests.rs"]
mod tests;

pub(super) fn response_kind(
    config: &RuntimeConfig,
    request: &GatewayRequest,
) -> Option<&'static str> {
    // V3 profile-specific validation is owned by its versioned projection.
    for version in ["v1", "v2"] {
        let prefix = format!("/{version}/instances/{}/", config.instance_id);
        if let Some(route) = request.path.strip_prefix(&prefix) {
            return match (request.method, route) {
                (GatewayMethod::Get, "state") => Some("state_response"),
                (GatewayMethod::Post, "action") => Some("action_response"),
                (GatewayMethod::Get, route)
                    if version == "v2" && route.starts_with("operations/") =>
                {
                    Some("reconcile_response")
                }
                _ => None,
            };
        }
    }
    None
}

pub(super) fn response(
    config: &RuntimeConfig,
    body: &JsonValue,
    correlation: &str,
    kind: &str,
) -> Result<(), GatewayError> {
    let JsonValue::Object(object) = body else {
        return Err(GatewayError::MalformedResponse);
    };
    for (name, expected) in [
        ("instance_id", config.instance_id.as_str()),
        ("session_id", config.session_id.as_str()),
        ("lease_id", config.lease_id.as_str()),
        ("correlation_id", correlation),
        ("kind", kind),
    ] {
        if object.get(name) != Some(&JsonValue::string(expected)) {
            return Err(GatewayError::MalformedResponse);
        }
    }
    if object.get("lease_epoch") != Some(&JsonValue::Number(config.lease_epoch)) {
        return Err(GatewayError::MalformedResponse);
    }
    Ok(())
}

pub(super) fn is_runtime_result(body: &JsonValue) -> bool {
    matches!(
        body,
        JsonValue::Object(object)
            if matches!(
                object.get("kind"),
                Some(JsonValue::String(kind))
                    if matches!(
                        kind.as_str(),
                        "state_response" | "action_response" | "reconcile_response"
                    )
            )
    )
}
