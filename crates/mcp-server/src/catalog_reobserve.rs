// SPDX-License-Identifier: MIT

use crate::GatewayResponse;

/// Validates the compact host catalog refusal for transport and semantic mapping.
/// Callers must independently restrict admission to the legal-action read route.
pub fn catalog_reobserve_body(response: &GatewayResponse, correlation_id: &str) -> Option<String> {
    let object = response.body.as_object()?;
    if object.len() != 3
        || object.get("correlation_id")?.as_string()? != correlation_id
        || object.get("recovery")?.as_string()? != "reobserve"
        || !admits_reobserve_code(response.status, object.get("error_code")?.as_string()?)
    {
        return None;
    }
    let body = response.body.to_json();
    (body.len() <= 1024).then_some(body)
}

/// The recovery codes the legal-action read admits, paired with the status each arrives with.
///
/// A refused launch contract reaches this route as `503` carrying a code the game-mod composes from
/// its own refusal prefix plus an optional bounded reason token (`AI-Ascension/sts2-gateway#85`).
/// The admitted set is the producer's vocabulary rather than a second one: see
/// [`is_launch_contract_refusal`].
fn admits_reobserve_code(status: u16, code: &str) -> bool {
    match (status, code) {
        (409, "stale_generation") => true,
        (503, "host_not_configured" | "host_observation_unavailable") => true,
        (503, code) => is_launch_contract_refusal(code),
        _ => false,
    }
}

/// True for the recovery code a refused launch contract carries.
///
/// The mod answers the bare prefix (`launch_contract_refused`) when a reason cannot be named on the
/// wire, and otherwise the prefix, `_`, and one reason token. The token rule is mirrored from the
/// producer so a code it cannot emit is refused here too: widening this set must not admit a
/// neighbouring string that merely starts the same way.
fn is_launch_contract_refusal(code: &str) -> bool {
    const PREFIX: &str = "launch_contract_refused";
    const MAX_REASON_BYTES: usize = 64;
    let Some(reason) = code.strip_prefix(PREFIX) else {
        return false;
    };
    if reason.is_empty() {
        return true;
    }
    let Some(token) = reason.strip_prefix('_') else {
        return false;
    };
    !token.is_empty()
        && token.len() <= MAX_REASON_BYTES
        && token
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
}
