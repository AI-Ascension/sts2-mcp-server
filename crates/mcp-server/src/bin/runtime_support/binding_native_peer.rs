// SPDX-License-Identifier: MIT

use super::RuntimeConfig;
use sts2_mcp_server::{
    COOP_NATIVE_PROTOCOL_VERSION, GatewayError, GatewayMethod, GatewayRequest, JsonValue,
};

/// Inject the private route credential only after all caller-visible request
/// fields have been admitted. The token cannot enter an MCP tool argument or
/// native envelope, and no arbitrary caller-supplied token header is accepted.
pub(super) fn attach(
    config: &RuntimeConfig,
    mut request: GatewayRequest,
) -> Result<GatewayRequest, GatewayError> {
    if is_route(config, &request) {
        let binding = config
            .coop_native_peer_binding
            .as_ref()
            .ok_or(GatewayError::Rejected)?;
        request
            .headers
            .insert(String::from("x-sts2-peer-token"), binding.token.clone());
    }
    Ok(request)
}

pub(super) fn is_route(config: &RuntimeConfig, request: &GatewayRequest) -> bool {
    let prefix = format!("/v1/instances/{}/coop/native/", config.instance_id);
    let Some(route) = request.path.strip_prefix(&prefix) else {
        return false;
    };
    match (request.method, route, request.body.is_some()) {
        (GatewayMethod::Get, "observation", false) => true,
        (GatewayMethod::Post, "legal-catalog" | "action" | "vote" | "rejoin" | "recover", true) => {
            request
                .body
                .as_ref()
                .and_then(|body| match body {
                    JsonValue::Object(object)
                        if object.get("protocol_version")
                            == Some(&JsonValue::string(COOP_NATIVE_PROTOCOL_VERSION))
                            && object.get("schema_digest")
                                == Some(&JsonValue::string(
                                    sts2_mcp_server::COOP_NATIVE_SCHEMA_DIGEST,
                                )) =>
                    {
                        Some(())
                    }
                    _ => None,
                })
                .is_some()
        }
        _ => false,
    }
}

pub(super) fn admit(config: &RuntimeConfig, request: &GatewayRequest) -> Result<(), GatewayError> {
    let binding = config
        .coop_native_peer_binding
        .as_ref()
        .ok_or(GatewayError::Rejected)?;
    if request
        .headers
        .keys()
        .any(|name| name.eq_ignore_ascii_case("x-sts2-peer-token"))
    {
        return Err(GatewayError::Rejected);
    }
    let route = request
        .path
        .rsplit('/')
        .next()
        .ok_or(GatewayError::Rejected)?;
    if matches!(route, "legal-catalog" | "action" | "vote" | "rejoin")
        && request
            .body
            .as_ref()
            .and_then(|body| match body {
                JsonValue::Object(object) => object.get("actor_peer"),
                _ => None,
            })
            .and_then(|value| match value {
                JsonValue::String(value) => Some(value.as_str()),
                _ => None,
            })
            != Some(binding.peer_id.as_str())
    {
        return Err(GatewayError::Rejected);
    }
    // Reconcile recovery intentionally has no acting peer in its public
    // envelope. Reject an injected identity rather than treating the private
    // credential as an actor or accepting a foreign caller-supplied actor.
    if route == "recover"
        && request.body.as_ref().and_then(|body| match body {
            JsonValue::Object(object) => object.get("actor_peer"),
            _ => None,
        }) != Some(&JsonValue::Null)
    {
        return Err(GatewayError::Rejected);
    }
    Ok(())
}
