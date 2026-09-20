// SPDX-License-Identifier: MIT

use super::{RuntimeConfig, safe_operation_id, save_profile};
use sts2_mcp_server::{
    COOP_NATIVE_PROTOCOL_VERSION, COOP_NATIVE_SCHEMA_DIGEST, COOP_RECEIPT_QUERY_PROTOCOL_VERSION,
    GAME_INFORMATION_PROTOCOL_VERSION, GAME_INFORMATION_SCHEMA_DIGEST, GatewayMethod,
    GatewayRequest, JsonValue, RUNTIME_V4_EXPERT_REST_ACTION_PROTOCOL_VERSION,
};

pub(crate) fn response_kind(
    config: &RuntimeConfig,
    request: &GatewayRequest,
) -> Option<&'static str> {
    for version in ["v1", "v2"] {
        let prefix = format!("/{version}/instances/{}/", config.instance_id);
        if let Some(route) = request.path.strip_prefix(&prefix) {
            if version == "v1" && save_profile::response_kind(request) {
                return Some("save_profile_response");
            }
            return match (request.method, route) {
                (GatewayMethod::Get, "coop/native/observation")
                    if version == "v1" && request.body.is_none() =>
                {
                    Some("observation")
                }
                (GatewayMethod::Post, "coop/native/legal-catalog")
                    if version == "v1"
                        && has_protocol_digest(
                            request,
                            COOP_NATIVE_PROTOCOL_VERSION,
                            COOP_NATIVE_SCHEMA_DIGEST,
                        ) =>
                {
                    Some("legal_catalog_response")
                }
                (GatewayMethod::Post, "coop/native/action" | "coop/native/vote")
                    if version == "v1"
                        && has_protocol_digest(
                            request,
                            COOP_NATIVE_PROTOCOL_VERSION,
                            COOP_NATIVE_SCHEMA_DIGEST,
                        ) =>
                {
                    Some("effect_response")
                }
                (GatewayMethod::Post, "coop/native/rejoin" | "coop/native/recover")
                    if version == "v1"
                        && has_protocol_digest(
                            request,
                            COOP_NATIVE_PROTOCOL_VERSION,
                            COOP_NATIVE_SCHEMA_DIGEST,
                        ) =>
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
                (GatewayMethod::Get, "game-information/capabilities")
                    if version == "v1" && request.body.is_none() =>
                {
                    Some("capabilities_response")
                }
                (GatewayMethod::Get, "game-information/content-manifest")
                    if version == "v1" && request.body.is_none() =>
                {
                    Some("content_manifest_response")
                }
                (GatewayMethod::Post, "game-information/query")
                    if version == "v1"
                        && has_protocol_digest(
                            request,
                            GAME_INFORMATION_PROTOCOL_VERSION,
                            GAME_INFORMATION_SCHEMA_DIGEST,
                        )
                        && has_kind(request, "query_request") =>
                {
                    Some("game_information_response")
                }
                (GatewayMethod::Post, "action") => Some("action_response"),
                (GatewayMethod::Post, "coop/receipt-query")
                    if version == "v1"
                        && has_protocol(request, COOP_RECEIPT_QUERY_PROTOCOL_VERSION) =>
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
    let route = request.path.strip_prefix(&prefix)?;
    match (request.method, route) {
        (GatewayMethod::Get, "expert-state") => Some("state_response"),
        (GatewayMethod::Post, "expert-action") => Some("action_response"),
        (GatewayMethod::Post, "expert-rest-action")
            if has_protocol(request, RUNTIME_V4_EXPERT_REST_ACTION_PROTOCOL_VERSION) =>
        {
            Some("action_response")
        }
        (GatewayMethod::Get, route)
            if route.starts_with("expert-rest-actions/")
                && safe_operation_id(route.strip_prefix("expert-rest-actions/").unwrap_or("")) =>
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
    }
}

fn has_protocol(request: &GatewayRequest, protocol: &str) -> bool {
    request.body.as_ref().is_some_and(|body| {
        matches!(
            body,
            JsonValue::Object(object)
                if object.get("protocol_version") == Some(&JsonValue::string(protocol))
        )
    })
}

fn has_protocol_digest(request: &GatewayRequest, protocol: &str, digest: &str) -> bool {
    request.body.as_ref().is_some_and(|body| {
        matches!(
            body,
            JsonValue::Object(object)
                if object.get("protocol_version") == Some(&JsonValue::string(protocol))
                    && object.get("schema_digest") == Some(&JsonValue::string(digest))
        )
    })
}

fn has_kind(request: &GatewayRequest, kind: &str) -> bool {
    request.body.as_ref().is_some_and(|body| {
        matches!(
            body,
            JsonValue::Object(object)
                if object.get("kind") == Some(&JsonValue::string(kind))
        )
    })
}
