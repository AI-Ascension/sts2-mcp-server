// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use sts2_mcp_server::{GatewayError, GatewayMethod, GatewayRequest, JsonValue};

pub(super) fn wire_operation(instance_id: &str, request: &GatewayRequest) -> Option<&'static str> {
    let v1_prefix = format!("/v1/instances/{instance_id}/");
    let v3_prefix = format!("/v3/instances/{instance_id}/");
    if request.method == GatewayMethod::Get
        && request.path == format!("{v1_prefix}game-information/capabilities")
    {
        return Some("sts2.game_information_capabilities");
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
