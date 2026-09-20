// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use sts2_mcp_server::{GatewayError, GatewayMethod, GatewayRequest, JsonValue};

/// The adapter's name for the gateway's fixed whole-manifest read.
pub(super) const CONTENT_MANIFEST_OPERATION: &str = "sts2.game_information_content_manifest";

pub(super) fn wire_operation(instance_id: &str, request: &GatewayRequest) -> Option<&'static str> {
    let v1_prefix = format!("/v1/instances/{instance_id}/");
    let v3_prefix = format!("/v3/instances/{instance_id}/");
    if request.method == GatewayMethod::Get
        && request.path == format!("{v1_prefix}game-information/capabilities")
    {
        return Some("sts2.game_information_capabilities");
    }
    if request.method == GatewayMethod::Get
        && request.path == format!("{v1_prefix}game-information/content-manifest")
    {
        return Some(CONTENT_MANIFEST_OPERATION);
    }
    if request.method == GatewayMethod::Post
        && request.path == format!("{v1_prefix}game-information/lookup-binding")
    {
        return Some("sts2.game_information_binding");
    }
    if request.method == GatewayMethod::Post
        && request.path == format!("{v1_prefix}game-information/live-observation-bootstrap")
    {
        return Some("sts2.game_information.live_observation_bootstrap");
    }
    if request.method == GatewayMethod::Post
        && request.path == format!("{v1_prefix}game-information/query")
    {
        let query_kind = request
            .body
            .as_ref()?
            .as_object()?
            .get("query")?
            .as_object()?
            .get("query_kind")?
            .as_string()?;
        return match query_kind {
            "list" => Some("sts2.game_information_list"),
            "search" => Some("sts2.game_information_search"),
            "get" => Some("sts2.game_information_get"),
            "detail" => Some("sts2.game_information_detail"),
            "availability" => Some("sts2.game_information_availability"),
            _ => None,
        };
    }
    if request.method == GatewayMethod::Get && request.path == format!("{v3_prefix}state") {
        return Some("sts2.observe");
    }
    if request.method == GatewayMethod::Get && request.path == format!("{v3_prefix}legal-actions") {
        return Some("sts2.legal_actions");
    }
    if request.method == GatewayMethod::Post && request.path == format!("{v3_prefix}action") {
        return Some("sts2.dispatch_action");
    }
    if request.method == GatewayMethod::Post && request.path == format!("{v3_prefix}wait") {
        return Some("sts2.wait_for_transition");
    }
    if request.method == GatewayMethod::Get && request.path == format!("{v3_prefix}reobserve") {
        return Some("sts2.reobserve");
    }
    if request.method == GatewayMethod::Post && request.path == format!("{v3_prefix}recover") {
        return Some("sts2.recover");
    }
    None
}

pub(super) fn validate_binding_body(
    object: &BTreeMap<String, JsonValue>,
) -> Result<(), GatewayError> {
    const FIELDS: [&str; 6] = [
        "operation",
        "project_id",
        "run_id",
        "episode_id",
        "agent_id",
        "authority_epoch",
    ];
    if object.len() != FIELDS.len() || FIELDS.iter().any(|field| !object.contains_key(*field)) {
        return Err(GatewayError::Rejected);
    }
    if !matches!(
        object.get("operation"),
        Some(JsonValue::String(operation)) if matches!(operation.as_str(), "discovery" | "observe")
    ) {
        return Err(GatewayError::Rejected);
    }
    for field in ["project_id", "run_id", "episode_id", "agent_id"] {
        let Some(JsonValue::String(value)) = object.get(field) else {
            return Err(GatewayError::Rejected);
        };
        if !super::super::safe_header_value(value) {
            return Err(GatewayError::Rejected);
        }
    }
    if !matches!(
        object.get("authority_epoch"),
        Some(JsonValue::Number(epoch)) if (1..=9_007_199_254_740_991).contains(epoch)
    ) {
        return Err(GatewayError::Rejected);
    }
    Ok(())
}

/// The legacy transport identity `inject_profile_identity` adds to runtime-v1 bodies.
///
/// The bootstrap profile is a self-describing envelope that the gateway re-validates against the
/// pinned schema, and that schema sets `additionalProperties: false`. These four members are
/// therefore *foreign members* on a bootstrap body, not an authority the gateway needs: adding
/// them made the gateway reject the request as schema-invalid before any producer call, which the
/// harness observed as `ProviderUnavailable` with no bootstrap request in the downstream trace.
pub(super) const LEGACY_IDENTITY_FIELDS: [&str; 4] =
    ["instance_id", "session_id", "lease_id", "lease_epoch"];

/// Admit the live-observation bootstrap envelope unchanged, refusing any legacy identity member.
///
/// The envelope is built and bounded by the bootstrap tool adapter, and the gateway is its
/// authority: it re-parses the bytes against the pinned schema and re-derives scope from its own
/// configured authority. The adapter must therefore forward the body verbatim. This guard states
/// that invariant directly rather than duplicating the schema's field list, so an additive schema
/// change cannot turn a valid request into a rejection here.
pub(super) fn validate_live_bootstrap_body(
    object: &BTreeMap<String, JsonValue>,
) -> Result<(), GatewayError> {
    if !matches!(
        object.get("protocol_version"),
        Some(JsonValue::String(version))
            if version.as_str() == sts2_mcp_server::LIVE_BOOTSTRAP_PROTOCOL_VERSION
    ) || !matches!(
        object.get("schema_digest"),
        Some(JsonValue::String(digest))
            if digest.as_str() == sts2_mcp_server::LIVE_BOOTSTRAP_SCHEMA_DIGEST
    ) || !matches!(
        object.get("kind"),
        Some(JsonValue::String(kind)) if kind.as_str() == "bootstrap_request"
    ) {
        return Err(GatewayError::Rejected);
    }
    if LEGACY_IDENTITY_FIELDS
        .iter()
        .any(|field| object.contains_key(*field))
    {
        return Err(GatewayError::Rejected);
    }
    Ok(())
}
