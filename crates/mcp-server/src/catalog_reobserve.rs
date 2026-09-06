// SPDX-License-Identifier: MIT

use crate::GatewayResponse;

/// Validates the compact host catalog refusal for transport and semantic mapping.
/// Callers must independently restrict admission to the legal-action read route.
pub fn catalog_reobserve_body(response: &GatewayResponse, correlation_id: &str) -> Option<String> {
    let object = response.body.as_object()?;
    if object.len() != 3
        || object.get("correlation_id")?.as_string()? != correlation_id
        || object.get("recovery")?.as_string()? != "reobserve"
        || !matches!(
            (response.status, object.get("error_code")?.as_string()?),
            (409, "stale_generation")
                | (503, "host_not_configured" | "host_observation_unavailable")
        )
    {
        return None;
    }
    let body = response.body.to_json();
    (body.len() <= 1024).then_some(body)
}
