// SPDX-License-Identifier: MIT

use super::{RuntimeConfig, safe_header_value};
use sts2_mcp_server::{GatewayError, GatewayMethod, GatewayRequest, JsonValue};

#[path = "binding_exact_restore.rs"]
mod exact_restore;
#[path = "binding_native_peer.rs"]
mod native_peer;
#[path = "binding_recovery.rs"]
mod recovery;
#[path = "binding_response.rs"]
mod response;
#[path = "binding_response_kind.rs"]
mod response_kind;
#[path = "binding_save_profile.rs"]
mod save_profile;
pub(super) use response::validate as response;
pub(super) use response_kind::response_kind;
#[path = "binding_result.rs"]
mod result;
pub(super) use result::{is_runtime_result, is_save_profile_result};

pub(super) fn is_save_profile_route(request: &GatewayRequest) -> bool {
    save_profile::is_route(request)
}

pub(super) fn is_game_information_binding_route(
    config: &RuntimeConfig,
    request: &GatewayRequest,
) -> bool {
    request.method == GatewayMethod::Post
        && request.path
            == format!(
                "/v1/instances/{}/game-information/lookup-binding",
                config.instance_id
            )
        && request.body.is_some()
}

pub(super) fn is_game_information_live_bootstrap_route(
    config: &RuntimeConfig,
    request: &GatewayRequest,
) -> bool {
    request.method == GatewayMethod::Post
        && request.path
            == format!(
                "/v1/instances/{}/game-information/live-observation-bootstrap",
                config.instance_id
            )
        && request.body.is_some()
}
pub(super) use exact_restore::{
    is_route as is_exact_restore_route, validate as exact_restore_binding,
};
pub(super) use recovery::{is_route as is_recovery_route, validate as recovery_binding};
pub(super) use save_profile::classify as classify_save_profile;

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
    let exact_restore_route = is_exact_restore_route(request);
    let recovery_route = is_recovery_route(request);
    let game_information_binding_route = is_game_information_binding_route(config, request);
    let game_information_live_bootstrap_route =
        is_game_information_live_bootstrap_route(config, request);
    if !matches!(version, "v1" | "v2" | "v3" | "v4")
        || (!exact_restore_route
            && !recovery_route
            && !request
                .path
                .starts_with(&format!("/{version}/instances/{}/", config.instance_id)))
        || !safe_header_value(&request.correlation.mcp_request_id.stable_text())
    {
        return Err(GatewayError::Rejected);
    }
    if recovery_route {
        recovery_binding(config, request)?;
        return Ok(());
    }
    if exact_restore_route {
        exact_restore_binding(config, request)?;
        return Ok(());
    }
    let native_route = version == "v1" && native_peer::is_route(config, request);
    if native_route {
        native_peer::admit(config, request)?;
    }
    if is_save_profile_route(request)
        && (!save_profile::headers_are_fixed(request) || !save_profile::body_is_fixed(request))
    {
        return Err(GatewayError::Rejected);
    }
    if version != "v3"
        && version != "v4"
        && response_kind(config, request).is_none()
        && !native_route
        && !game_information_binding_route
        && !game_information_live_bootstrap_route
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
        && !request.path.ends_with("/checkpoint-reference")
        && !request.path.ends_with("/game-information/capabilities")
        && !request.path.ends_with("/game-information/query")
        && !game_information_binding_route
        && !game_information_live_bootstrap_route
        && !is_save_profile_route(request);
    if !is_legacy_v1_injection {
        // MCP correlation sessions are a separate namespace; only explicit gateway
        // authority headers/body fields are compared with configured gateway identity.
        let game_information_route = request.path.ends_with("/game-information/capabilities")
            || request.path.ends_with("/game-information/query")
            || game_information_binding_route
            || game_information_live_bootstrap_route;
        for (name, expected) in [
            ("x-sts2-instance-id", config.instance_id.as_str()),
            ("x-sts2-session-id", config.session_id.as_str()),
            ("x-sts2-lease-id", config.lease_id.as_str()),
            ("x-sts2-lease-epoch", &config.lease_epoch.to_string()),
        ] {
            let supplied = request.headers.get(name);
            if supplied.is_some_and(|value| value != expected)
                || (supplied.is_none()
                    && (request.body.is_none()
                        || game_information_route
                        || is_save_profile_route(request))
                    && !adapter_injected_v4_authority)
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

#[cfg(test)]
#[path = "binding_save_profile_tests.rs"]
mod save_profile_tests;

fn safe_operation_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && !value.contains("..")
        && !value.contains('/')
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}

#[path = "binding_checkpoint_reference.rs"]
mod checkpoint_reference;
