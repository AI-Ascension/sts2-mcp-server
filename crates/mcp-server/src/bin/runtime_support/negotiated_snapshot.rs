// SPDX-License-Identifier: MIT

use std::collections::BTreeSet;

use sts2_mcp_server::{
    CapabilityLayer, CapabilityOwner, CapabilityScope, GAME_INFORMATION_SCHEMA_DIGEST, JsonValue,
    ToolCatalog,
};

use super::super::{
    RuntimeConfig,
    http::MAP_MAX_RESPONSE_BYTES,
    negotiated_discovery::DiscoveryRequest,
    negotiated_offers::{
        mapped_layer, remote_offers, require_composed_baseline, require_runtime_baseline,
    },
    profiles::RuntimeProfile,
};

const RUNTIME_WITNESS_PROFILE: &str = "runtime-v3-gameplay";
const RUNTIME_WITNESS_DIGEST: &str =
    "8e99cea36b7ede97532348fd8efe302ca79260895265a7bf14ddf7e006d8ff63";
const LOOKUP_PROFILE: &str = "game-information-lookup-binding-v1";
const LOOKUP_DIGEST: &str = "f10f9af01d6be1de104069ba842e7971971e88f27553e782e81174ee7aa1cd58";

pub(super) fn build_profile(
    config: &RuntimeConfig,
    discovery_request: Option<&DiscoveryRequest>,
    discovery_response: Option<&JsonValue>,
    snapshot: JsonValue,
) -> Result<RuntimeProfile, String> {
    validate_identity(&snapshot, config)?;
    validate_producer(&snapshot)?;
    validate_runtime_witness(&snapshot)?;
    let caller_scope = caller_scope(&snapshot)?;
    let remote = remote_offers(&snapshot)?;
    require_runtime_baseline(&remote, caller_scope)?;
    if discovery_request.is_some()
        && (!remote.contains_key("game_information.lookup_binding.discovery")
            || !remote.contains_key("game_information.lookup_binding.observe"))
    {
        return Err(String::from(
            "Gateway lookup-binding discovery offers are incomplete",
        ));
    }
    let lookup_is_current =
        validate_lookup_binding(&snapshot, discovery_request, discovery_response)?;
    if discovery_request.is_some() && !lookup_is_current {
        return Err(String::from(
            "Gateway did not retain the requested current lookup-binding witness",
        ));
    }
    let local_profiles = [
        ToolCatalog::runtime_map_v1_negotiated(),
        ToolCatalog::game_information(),
        ToolCatalog::game_information_live_observation_bootstrap(),
    ];
    let mcp = CapabilityLayer::from_catalogs(CapabilityOwner::Mcp, &local_profiles)
        .map_err(negotiation_error)?;
    let (gateway, wire_limits) =
        mapped_layer(CapabilityOwner::Gateway, &mcp, &remote, lookup_is_current)?;
    let (producer, _) = mapped_layer(CapabilityOwner::Producer, &mcp, &remote, lookup_is_current)?;
    let catalog = ToolCatalog::compose_profiles(&local_profiles, gateway, producer, caller_scope)
        .map_err(negotiation_error)?;
    require_composed_baseline(&catalog, caller_scope)?;
    let max_response_bytes = wire_limits
        .values()
        .map(|limit| limit.max_response_bytes)
        .max()
        .unwrap_or(MAP_MAX_RESPONSE_BYTES);
    Ok(RuntimeProfile {
        catalog,
        max_response_bytes,
        requires_coop_native_peer_binding: false,
        wire_limits,
    })
}

fn validate_identity(value: &JsonValue, config: &RuntimeConfig) -> Result<(), String> {
    for (field, expected) in [
        ("instance_id", config.instance_id.as_str()),
        ("caller_id", config.caller_id.as_str()),
        ("session_id", config.session_id.as_str()),
        ("mcp_session_id", config.mcp_session_id.as_str()),
        ("lease_id", config.lease_id.as_str()),
    ] {
        if string(value, field)? != expected {
            return Err(String::from(
                "Gateway snapshot identity does not match configured owner",
            ));
        }
    }
    if integer(value, "lease_epoch")? != config.lease_epoch
        || string(value, "correlation_id")? != super::CAPABILITY_CORRELATION
    {
        return Err(String::from(
            "Gateway snapshot lease or correlation does not match startup",
        ));
    }
    Ok(())
}

fn validate_producer(value: &JsonValue) -> Result<(), String> {
    let producer = member(value, "producer")?;
    if string(producer, "profile")? != "game-information-query-v1"
        || string(producer, "schema_digest")? != GAME_INFORMATION_SCHEMA_DIGEST
    {
        return Err(String::from(
            "Gateway producer profile or schema digest is not pinned",
        ));
    }
    identity(producer, "content_manifest_id")?;
    identity(producer, "run_id")?;
    Ok(())
}

fn validate_runtime_witness(value: &JsonValue) -> Result<(), String> {
    let witness = member(value, "runtime_v3_baseline_witness")?;
    if string(witness, "profile")? != RUNTIME_WITNESS_PROFILE
        || string(witness, "schema_digest")? != RUNTIME_WITNESS_DIGEST
        || member(witness, "configured_state_probe")? != &JsonValue::Bool(true)
        || member(witness, "recovery_kinds")?
            != &JsonValue::Array(vec![
                JsonValue::string("reobserve"),
                JsonValue::string("reconcile"),
            ])
    {
        return Err(String::from(
            "Gateway Runtime-v3 baseline witness is not the pinned configured adapter",
        ));
    }
    Ok(())
}

fn validate_lookup_binding(
    snapshot: &JsonValue,
    request: Option<&DiscoveryRequest>,
    response: Option<&JsonValue>,
) -> Result<bool, String> {
    let (Some(request), Some(response)) = (request, response) else {
        return Ok(false);
    };
    let binding = member(response, "binding")?;
    let producer = member(snapshot, "producer")?;
    let scope = member(binding, "scope")?;
    if string(binding, "content_manifest_id")? != string(producer, "content_manifest_id")?
        || string(scope, "run_id")? != string(producer, "run_id")?
        || integer(binding, "authority_epoch")? != request.authority_epoch
    {
        return Err(String::from(
            "lookup-binding response does not match current producer manifest and run",
        ));
    }
    let witness = member(snapshot, "lookup_binding_witness")?;
    if witness == &JsonValue::Null
        || string(witness, "profile")? != LOOKUP_PROFILE
        || string(witness, "schema_digest")? != LOOKUP_DIGEST
        || string(witness, "binding_id")? != string(binding, "binding_id")?
        || string(witness, "content_manifest_id")? != string(binding, "content_manifest_id")?
        || integer(witness, "authority_epoch")? != request.authority_epoch
    {
        return Err(String::from(
            "Gateway lookup-binding witness does not match validated discovery",
        ));
    }
    Ok(true)
}

fn caller_scope(snapshot: &JsonValue) -> Result<CapabilityScope, String> {
    let mut scope = CapabilityScope::NONE;
    let mut seen = BTreeSet::new();
    for item in array(member(snapshot, "caller_scopes")?)? {
        let name = item
            .as_string()
            .ok_or_else(|| String::from("Gateway caller scope is malformed"))?;
        if !seen.insert(name) {
            return Err(String::from("Gateway caller scopes contain duplicates"));
        }
        scope = scope
            | match name {
                "read" => CapabilityScope::READ,
                "mutate" => CapabilityScope::MUTATE,
                "control" => CapabilityScope::CONTROL,
                _ => return Err(String::from("Gateway caller scope is unsupported")),
            };
    }
    if !scope.contains(CapabilityScope::READ) {
        return Err(String::from(
            "Gateway caller lacks the read scope required by this profile",
        ));
    }
    Ok(scope)
}

fn negotiation_error(error: sts2_mcp_server::NegotiationError) -> String {
    format!("negotiated MCP capability composition failed: {error}")
}

fn member<'a>(value: &'a JsonValue, key: &str) -> Result<&'a JsonValue, String> {
    value
        .as_object()
        .and_then(|object| object.get(key))
        .ok_or_else(|| format!("Gateway snapshot is missing {key}"))
}

fn string<'a>(value: &'a JsonValue, key: &str) -> Result<&'a str, String> {
    member(value, key)?
        .as_string()
        .ok_or_else(|| format!("Gateway snapshot {key} is malformed"))
}

fn identity<'a>(value: &'a JsonValue, key: &str) -> Result<&'a str, String> {
    let value = string(value, key)?;
    if value.is_empty()
        || value.len() > 128
        || value.contains("..")
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_./:".contains(&byte))
    {
        return Err(format!("Gateway snapshot {key} is unsafe or oversized"));
    }
    Ok(value)
}

fn integer(value: &JsonValue, key: &str) -> Result<i64, String> {
    match member(value, key)? {
        JsonValue::Number(value) if *value >= 0 => Ok(*value),
        _ => Err(format!("Gateway snapshot {key} is malformed")),
    }
}

fn array(value: &JsonValue) -> Result<&[JsonValue], String> {
    value
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| String::from("Gateway snapshot array is malformed"))
}

#[cfg(test)]
#[path = "negotiated_snapshot_tests.rs"]
mod tests;
