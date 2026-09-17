// SPDX-License-Identifier: MIT

use sts2_mcp_server::{
    GAME_INFORMATION_MAX_MESSAGE_BYTES, GatewayMethod, JsonValue,
    NEGOTIATED_CAPABILITIES_MAX_BYTES, validate_game_information_binding_discovery,
    validate_negotiated_capabilities_snapshot, validate_negotiated_capabilities_v2_snapshot,
    verify_game_information_artifact, verify_negotiated_capabilities_artifact,
    verify_negotiated_capabilities_v2_artifact, verify_runtime_map_artifact,
};

use super::{RuntimeConfig, exchange, negotiated_discovery, profiles::RuntimeProfile};

#[path = "negotiated_snapshot.rs"]
mod snapshot;

const CAPABILITY_CORRELATION: &str = "negotiated-capabilities-startup";
const LOOKUP_BINDING_PATH: &str = "/v1/instances/";
const LOOKUP_BINDING_SUFFIX: &str = "/game-information/lookup-binding";
const CAPABILITIES_SUFFIX: &str = "/negotiated-capabilities";

pub(super) fn bootstrap(config: &RuntimeConfig) -> Result<RuntimeProfile, String> {
    verify_negotiated_capabilities_artifact()
        .map_err(|_| String::from("negotiated capabilities artifact is invalid"))?;
    verify_runtime_map_artifact().map_err(|_| String::from("runtime-map artifact is invalid"))?;
    verify_game_information_artifact()
        .map_err(|_| String::from("game-information artifact is invalid"))?;
    let discovery_request = negotiated_discovery::from_environment()?;
    let discovery_response = discovery_request
        .as_ref()
        .map(|request| discover_lookup_binding(config, request))
        .transpose()?;
    let snapshot = fetch_snapshot(config)?;
    snapshot::build_profile(
        config,
        discovery_request.as_ref(),
        discovery_response.as_ref(),
        snapshot,
    )
}

fn discover_lookup_binding(
    config: &RuntimeConfig,
    request: &negotiated_discovery::DiscoveryRequest,
) -> Result<JsonValue, String> {
    let body = request.gateway_body().to_json();
    if body.len() > 16 * 1024 {
        return Err(String::from(
            "lookup-binding startup request exceeds its wire bound",
        ));
    }
    let path = format!(
        "{LOOKUP_BINDING_PATH}{}{LOOKUP_BINDING_SUFFIX}",
        config.instance_id
    );
    let response = exchange::exchange_startup(
        config,
        GatewayMethod::Post,
        &path,
        body.as_bytes(),
        negotiated_discovery::CORRELATION_ID,
        GAME_INFORMATION_MAX_MESSAGE_BYTES,
    )
    .map_err(|_| String::from("Gateway lookup-binding discovery failed"))?;
    if !(200..300).contains(&response.status) {
        return Err(String::from(
            "Gateway lookup-binding discovery was not accepted",
        ));
    }
    validate_game_information_binding_discovery(
        &response.body,
        &config.instance_id,
        negotiated_discovery::CORRELATION_ID,
        &request.gateway_body(),
    )
    .map_err(|_| String::from("Gateway lookup-binding discovery did not match its owner fence"))?;
    Ok(response.body)
}

fn fetch_snapshot(config: &RuntimeConfig) -> Result<JsonValue, String> {
    let path = format!("/v1/instances/{}{CAPABILITIES_SUFFIX}", config.instance_id);
    let response = exchange::exchange_startup_capabilities(
        config,
        &path,
        CAPABILITY_CORRELATION,
        NEGOTIATED_CAPABILITIES_MAX_BYTES,
    )
    .map_err(|_| String::from("Gateway negotiated capability discovery failed"))?;
    if !(200..300).contains(&response.status) {
        return Err(String::from(
            "Gateway negotiated capability discovery was not accepted",
        ));
    }
    match response
        .body
        .as_object()
        .and_then(|object| object.get("schema_version"))
        .and_then(JsonValue::as_string)
    {
        Some("sts2-gateway-negotiated-capabilities-v2") => {
            verify_negotiated_capabilities_v2_artifact()
                .map_err(|_| String::from("Gateway negotiated v2 artifact is invalid"))?;
            validate_negotiated_capabilities_v2_snapshot(&response.body.to_json())
                .map_err(|_| String::from("Gateway negotiated v2 capability snapshot is invalid"))
        }
        Some("sts2-gateway-negotiated-capabilities-v1") => {
            verify_negotiated_capabilities_artifact()
                .map_err(|_| String::from("Gateway negotiated v1 artifact is invalid"))?;
            validate_negotiated_capabilities_snapshot(&response.body.to_json())
                .map_err(|_| String::from("Gateway negotiated v1 capability snapshot is invalid"))
        }
        _ => Err(String::from(
            "Gateway negotiated capability snapshot has an unsupported version",
        )),
    }
}
