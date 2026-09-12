// SPDX-License-Identifier: MIT

use super::{RuntimeConfig, safe_header_value};
use sts2_mcp_server::{
    COOP_NATIVE_PROTOCOL_VERSION, COOP_RECEIPT_QUERY_PROTOCOL_VERSION, GatewayError, GatewayMethod,
    GatewayRequest, JsonValue, RUNTIME_V4_EXPERT_REST_ACTION_PROTOCOL_VERSION,
};

#[path = "binding_native_peer.rs"]
mod native_peer;

pub(super) fn is_runtime_result(body: &JsonValue) -> bool {
    matches!(
        body,
        JsonValue::Object(object)
            if matches!(
                object.get("kind"),
                Some(JsonValue::String(kind))
                    if matches!(
                        kind.as_str(),
                        "state_response"
                            | "action_response"
                            | "reconcile_response"
                            | "legal_actions_response"
                            | "dispatch_action_response"
                            | "wait_response"
                            | "reobserve_response"
                            | "recover_response"
                            | "snapshot_response"
                            | "receipt_query_response"
                            | "start_response"
                            | "observation"
                            | "legal_catalog_response"
                            | "effect_response"
                            | "recovery_response"
                    )
            )
    )
}

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
    if !matches!(version, "v1" | "v2" | "v3" | "v4")
        || !request
            .path
            .starts_with(&format!("/{version}/instances/{}/", config.instance_id))
        || !safe_header_value(&request.correlation.mcp_request_id.stable_text())
    {
        return Err(GatewayError::Rejected);
    }
    let native_route = version == "v1" && native_peer::is_route(config, request);
    if native_route {
        native_peer::admit(config, request)?;
    }
    if version != "v3"
        && version != "v4"
        && response_kind(config, request).is_none()
        && !native_route
    {
        return Err(GatewayError::Rejected);
    }
    if version == "v4" {
        let prefix = format!("/v4/instances/{}/", config.instance_id);
        let route = request
            .path
            .strip_prefix(&prefix)
            .ok_or(GatewayError::Rejected)?;
        match (request.method, route) {
            (GatewayMethod::Get, "expert-state") if request.body.is_none() => {}
            (GatewayMethod::Post, "expert-action") if request.body.is_some() => {}
            (GatewayMethod::Post, "expert-rest-action") if request.body.is_some() => {}
            (GatewayMethod::Get, route)
                if request.body.is_none()
                    && (route.starts_with("expert-actions/")
                        || route.starts_with("expert-rest-actions/")) =>
            {
                let operation_id = route
                    .strip_prefix("expert-actions/")
                    .or_else(|| route.strip_prefix("expert-rest-actions/"))
                    .ok_or(GatewayError::Rejected)?;
                if !safe_operation_id(operation_id) {
                    return Err(GatewayError::Rejected);
                }
            }
            _ => return Err(GatewayError::Rejected),
        }
    }
    checkpoint_reference::admit(config, request)?;
    let adapter_injected_v4_authority = version == "v4" && request.body.is_none();
    // Runtime-v1 retains its documented configured identity injection. Newer profiles
    // must not silently substitute authority, including for bodyless observation calls.
    let is_legacy_v1_injection = version == "v1"
        && !request.path.ends_with("/coop/synchronization")
        && !request.path.ends_with("/coop/receipt-query")
        && !request.path.ends_with("/map-snapshot")
        && !request.path.ends_with("/checkpoint-reference");
    if !is_legacy_v1_injection {
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
                || (request.body.is_none() && supplied.is_none() && !adapter_injected_v4_authority)
            {
                return Err(GatewayError::Rejected);
            }
        }
    }
    Ok(())
}

/// Inject the private route credential only after all caller-visible request
/// fields have been admitted. The token cannot enter an MCP tool argument or
/// native envelope, and no arbitrary caller-supplied token header is accepted.
pub(super) fn attach_native_peer_token(
    config: &RuntimeConfig,
    request: GatewayRequest,
) -> Result<GatewayRequest, GatewayError> {
    native_peer::attach(config, request)
}

#[cfg(test)]
#[path = "binding_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "binding_native_peer_tests.rs"]
mod native_peer_tests;

pub(super) fn response_kind(
    config: &RuntimeConfig,
    request: &GatewayRequest,
) -> Option<&'static str> {
    // V3 profile-specific validation is owned by its versioned projection.
    for version in ["v1", "v2"] {
        let prefix = format!("/{version}/instances/{}/", config.instance_id);
        if let Some(route) = request.path.strip_prefix(&prefix) {
            return match (request.method, route) {
                (GatewayMethod::Get, "coop/native/observation")
                    if version == "v1" && request.body.is_none() =>
                {
                    Some("observation")
                }
                (GatewayMethod::Post, "coop/native/legal-catalog")
                    if version == "v1" && is_native_body(request) =>
                {
                    Some("legal_catalog_response")
                }
                (GatewayMethod::Post, "coop/native/action" | "coop/native/vote")
                    if version == "v1" && is_native_body(request) =>
                {
                    Some("effect_response")
                }
                (GatewayMethod::Post, "coop/native/rejoin" | "coop/native/recover")
                    if version == "v1" && is_native_body(request) =>
                {
                    Some("recovery_response")
                }
                (GatewayMethod::Get, "coop/synchronization")
                    if version == "v1" && request.body.is_none() =>
                {
                    Some("synchronization_response")
                }
                (GatewayMethod::Get, "checkpoint-reference")
                    if version == "v1" && request.body.is_none() =>
                {
                    Some("checkpoint_reference_response")
                }
                (GatewayMethod::Get, "state") => Some("state_response"),
                (GatewayMethod::Get, "map-snapshot") if version == "v1" => {
                    Some("snapshot_response")
                }
                (GatewayMethod::Post, "action") => Some("action_response"),
                (GatewayMethod::Post, "coop/receipt-query")
                    if version == "v1"
                        && request.body.as_ref().and_then(|body| match body {
                            JsonValue::Object(object) => object.get("protocol_version"),
                            _ => None,
                        }) == Some(&JsonValue::string(COOP_RECEIPT_QUERY_PROTOCOL_VERSION)) =>
                {
                    Some("receipt_query_response")
                }
                (GatewayMethod::Get, route)
                    if version == "v2" && route.starts_with("operations/") =>
                {
                    Some("reconcile_response")
                }
                (GatewayMethod::Post, "seeded-run")
                    if version == "v2" && request.body.is_some() =>
                {
                    Some("start_response")
                }
                (GatewayMethod::Get, route)
                    if version == "v2"
                        && route.starts_with("seeded-operations/")
                        && safe_operation_id(
                            route.strip_prefix("seeded-operations/").unwrap_or(""),
                        ) =>
                {
                    Some("reconcile_response")
                }
                _ => None,
            };
        }
    }
    let prefix = format!("/v4/instances/{}/", config.instance_id);
    if let Some(route) = request.path.strip_prefix(&prefix) {
        return match (request.method, route) {
            (GatewayMethod::Get, "expert-state") => Some("state_response"),
            (GatewayMethod::Post, "expert-action") => Some("action_response"),
            (GatewayMethod::Post, "expert-rest-action")
                if request.body.as_ref().and_then(|body| match body {
                    JsonValue::Object(object) => object.get("protocol_version"),
                    _ => None,
                }) == Some(&JsonValue::string(
                    RUNTIME_V4_EXPERT_REST_ACTION_PROTOCOL_VERSION,
                )) =>
            {
                Some("action_response")
            }
            (GatewayMethod::Get, route)
                if route.starts_with("expert-rest-actions/")
                    && safe_operation_id(
                        route.strip_prefix("expert-rest-actions/").unwrap_or(""),
                    ) =>
            {
                Some("action_response")
            }
            (GatewayMethod::Get, route)
                if route.starts_with("expert-actions/")
                    && safe_operation_id(route.strip_prefix("expert-actions/").unwrap_or("")) =>
            {
                Some("action_response")
            }
            _ => None,
        };
    }
    None
}

fn is_native_body(request: &GatewayRequest) -> bool {
    request.body.as_ref().is_some_and(|body| {
        matches!(
            body,
            JsonValue::Object(object)
                if object.get("protocol_version")
                    == Some(&JsonValue::string(COOP_NATIVE_PROTOCOL_VERSION))
                    && object.get("schema_digest")
                        == Some(&JsonValue::string(sts2_mcp_server::COOP_NATIVE_SCHEMA_DIGEST))
        )
    })
}

fn safe_operation_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && !value.contains("..")
        && !value.contains('/')
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
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
        if name == "kind" && kind == "checkpoint_reference_response" {
            checkpoint_reference::response(config, body)?;
            continue;
        }
        if object.get(name) != Some(&JsonValue::string(expected)) {
            return Err(GatewayError::MalformedResponse);
        }
    }
    if object.get("lease_epoch") != Some(&JsonValue::Number(config.lease_epoch)) {
        return Err(GatewayError::MalformedResponse);
    }
    Ok(())
}

#[path = "binding_checkpoint_reference.rs"]
mod checkpoint_reference;
