// SPDX-License-Identifier: MIT

use std::collections::{BTreeMap, BTreeSet};

use sts2_mcp_server::{CapabilityScope, JsonValue};

use super::super::profiles::GatewayWireLimits;

#[derive(Clone)]
pub(crate) struct RemoteOffer {
    pub(crate) revision: String,
    pub(crate) required_scope: CapabilityScope,
    pub(crate) scope: CapabilityScope,
    pub(crate) wire_limits: GatewayWireLimits,
    pub(crate) content_limits: GatewayContentLimits,
}

#[derive(Clone, Copy)]
pub(crate) struct GatewayContentLimits {
    pub(crate) max_content_bytes: usize,
    pub(crate) max_page_items: usize,
}

pub(crate) fn remote_offers(value: &JsonValue) -> Result<BTreeMap<String, RemoteOffer>, String> {
    let mut offers = BTreeMap::new();
    for item in array(member(value, "offers")?)? {
        let operation = string(item, "operation")?.to_owned();
        let revision = string(item, "revision")?.to_owned();
        let required_scope = parse_single_scope(member(item, "required_scope")?)?;
        let scope = parse_scope(member(item, "scope")?)?;
        if !scope.contains(required_scope) {
            return Err(String::from(
                "Gateway offer omits its required caller scope",
            ));
        }
        let wire = member(item, "wire_limits")?;
        let content = member(item, "content_limits")?;
        let offer = RemoteOffer {
            revision,
            required_scope,
            scope,
            wire_limits: GatewayWireLimits {
                max_request_bytes: usize_value(wire, "max_request_bytes")?,
                max_response_bytes: usize_value(wire, "max_response_bytes")?,
            },
            content_limits: GatewayContentLimits {
                max_content_bytes: usize_value(content, "max_content_bytes")?,
                max_page_items: usize_value(content, "max_page_items")?,
            },
        };
        if offers.insert(operation, offer).is_some() {
            return Err(String::from("Gateway offers contain duplicate operations"));
        }
    }
    Ok(offers)
}

fn parse_scope(value: &JsonValue) -> Result<CapabilityScope, String> {
    let mut scope = CapabilityScope::NONE;
    let mut seen = BTreeSet::new();
    for item in array(value)? {
        let bit = parse_single_scope(item)?;
        let name = item
            .as_string()
            .ok_or_else(|| String::from("Gateway operation scope is malformed"))?;
        if !seen.insert(name) {
            return Err(String::from("Gateway operation scope contains duplicates"));
        }
        scope = scope | bit;
    }
    Ok(scope)
}

fn parse_single_scope(value: &JsonValue) -> Result<CapabilityScope, String> {
    match value.as_string() {
        Some("read") => Ok(CapabilityScope::READ),
        Some("mutate") => Ok(CapabilityScope::MUTATE),
        Some("control") => Ok(CapabilityScope::CONTROL),
        _ => Err(String::from("Gateway operation scope is unsupported")),
    }
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

fn integer(value: &JsonValue, key: &str) -> Result<i64, String> {
    match member(value, key)? {
        JsonValue::Number(value) if *value >= 0 => Ok(*value),
        _ => Err(format!("Gateway snapshot {key} is malformed")),
    }
}

fn usize_value(value: &JsonValue, key: &str) -> Result<usize, String> {
    usize::try_from(integer(value, key)?)
        .map_err(|_| format!("Gateway snapshot {key} exceeds the local integer bound"))
}

fn array(value: &JsonValue) -> Result<&[JsonValue], String> {
    value
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| String::from("Gateway snapshot array is malformed"))
}
