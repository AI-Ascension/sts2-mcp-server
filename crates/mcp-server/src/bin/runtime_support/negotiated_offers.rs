// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use sts2_mcp_server::{
    CapabilityLayer, CapabilityOffer, CapabilityOwner, CapabilityScope, DISPATCH_ACTION_TOOL,
    GAME_INFORMATION_AVAILABILITY_TOOL, GAME_INFORMATION_BINDING_TOOL,
    GAME_INFORMATION_CAPABILITIES_TOOL, GAME_INFORMATION_DETAIL_TOOL, GAME_INFORMATION_GET_TOOL,
    GAME_INFORMATION_LIST_TOOL, GAME_INFORMATION_LIVE_OBSERVATION_BOOTSTRAP_TOOL,
    GAME_INFORMATION_SEARCH_TOOL, LEGAL_ACTIONS_TOOL, NEGOTIATED_COMPOSITION_REVISION,
    NegotiationError, OBSERVE_TOOL, RECOVER_TOOL, REOBSERVE_TOOL, ToolCatalog, ToolLimits,
    WAIT_FOR_TRANSITION_TOOL,
};

use super::profiles::GatewayWireLimits;

#[path = "negotiated_offer_parse.rs"]
mod offer_parse;
pub(super) use offer_parse::{RemoteOffer, remote_offers};

struct Mapping {
    local: &'static str,
    remote: &'static [&'static str],
}

const MAPPINGS: &[Mapping] = &[
    Mapping {
        local: GAME_INFORMATION_CAPABILITIES_TOOL,
        remote: &["game_information.capabilities"],
    },
    Mapping {
        local: GAME_INFORMATION_LIST_TOOL,
        remote: &["game_information.list"],
    },
    Mapping {
        local: GAME_INFORMATION_SEARCH_TOOL,
        remote: &["game_information.search"],
    },
    Mapping {
        local: GAME_INFORMATION_GET_TOOL,
        remote: &["game_information.get"],
    },
    Mapping {
        local: GAME_INFORMATION_DETAIL_TOOL,
        remote: &["game_information.detail"],
    },
    Mapping {
        local: GAME_INFORMATION_AVAILABILITY_TOOL,
        remote: &["game_information.availability"],
    },
    Mapping {
        local: GAME_INFORMATION_BINDING_TOOL,
        remote: &[
            "game_information.lookup_binding.discovery",
            "game_information.lookup_binding.observe",
        ],
    },
    Mapping {
        local: GAME_INFORMATION_LIVE_OBSERVATION_BOOTSTRAP_TOOL,
        remote: &["game_information.live_observation_bootstrap"],
    },
    Mapping {
        local: OBSERVE_TOOL,
        remote: &["runtime_v3.state"],
    },
    Mapping {
        local: LEGAL_ACTIONS_TOOL,
        remote: &["runtime_v3.legal_actions"],
    },
    Mapping {
        local: DISPATCH_ACTION_TOOL,
        remote: &["runtime_v3.dispatch_action"],
    },
    Mapping {
        local: WAIT_FOR_TRANSITION_TOOL,
        remote: &["runtime_v3.wait"],
    },
    Mapping {
        local: REOBSERVE_TOOL,
        remote: &["runtime_v3.reobserve"],
    },
    Mapping {
        local: RECOVER_TOOL,
        remote: &["runtime_v3.recover"],
    },
];

pub(super) fn require_runtime_baseline(
    offers: &BTreeMap<String, RemoteOffer>,
    caller_scope: CapabilityScope,
) -> Result<(), String> {
    for operation in [
        "runtime_v3.state",
        "runtime_v3.legal_actions",
        "runtime_v3.wait",
        "runtime_v3.reobserve",
    ] {
        if !offers.contains_key(operation) {
            return Err(String::from(
                "Gateway Runtime-v3 baseline offers are incomplete",
            ));
        }
    }
    if caller_scope.contains(CapabilityScope::MUTATE)
        && !offers.contains_key("runtime_v3.dispatch_action")
    {
        return Err(String::from(
            "Gateway Runtime-v3 mutation offer is missing for an authorized caller",
        ));
    }
    if caller_scope.contains(CapabilityScope::CONTROL) && !offers.contains_key("runtime_v3.recover")
    {
        return Err(String::from(
            "Gateway Runtime-v3 recovery offer is missing for an authorized caller",
        ));
    }
    Ok(())
}

pub(super) fn mapped_layer(
    owner: CapabilityOwner,
    mcp: &CapabilityLayer,
    remote: &BTreeMap<String, RemoteOffer>,
    lookup_is_current: bool,
) -> Result<(CapabilityLayer, BTreeMap<String, GatewayWireLimits>), String> {
    let mut layer = CapabilityLayer::new(owner, NEGOTIATED_COMPOSITION_REVISION);
    let mut wire_limits = BTreeMap::new();
    for mapping in MAPPINGS {
        if matches!(
            mapping.local,
            GAME_INFORMATION_BINDING_TOOL | GAME_INFORMATION_LIVE_OBSERVATION_BOOTSTRAP_TOOL
        ) && !lookup_is_current
        {
            continue;
        }
        let Some(mapped) = combine_remote(mapping, remote)? else {
            continue;
        };
        let local = mcp
            .offer(mapping.local)
            .ok_or_else(|| String::from("MCP profile is missing a mapped tool"))?;
        if mapped.required_scope != local.required_scope {
            return Err(String::from(
                "Gateway operation scope does not match the local MCP operation",
            ));
        }
        layer
            .insert(
                CapabilityOffer::supported(
                    local.operation.clone(),
                    local.revision.clone(),
                    local.group,
                    local.required_scope,
                    local.limits.minimum(ToolLimits::bounded(
                        local.limits.max_request_bytes,
                        local.limits.max_response_bytes,
                        mapped
                            .content_limits
                            .max_content_bytes
                            .min(local.limits.max_content_bytes),
                        mapped
                            .content_limits
                            .max_page_items
                            .min(local.limits.max_page_items),
                    )),
                )
                .with_scope(mapped.scope),
            )
            .map_err(negotiation_error)?;
        wire_limits.insert(mapping.local.to_owned(), mapped.wire_limits);
    }
    Ok((layer, wire_limits))
}

pub(super) fn require_composed_baseline(
    catalog: &ToolCatalog,
    caller_scope: CapabilityScope,
) -> Result<(), String> {
    if !catalog.tools().iter().any(|tool| tool.name == OBSERVE_TOOL)
        || !catalog
            .tools()
            .iter()
            .any(|tool| tool.name == LEGAL_ACTIONS_TOOL)
        || !catalog
            .tools()
            .iter()
            .any(|tool| tool.name == WAIT_FOR_TRANSITION_TOOL)
        || !catalog
            .tools()
            .iter()
            .any(|tool| tool.name == REOBSERVE_TOOL)
        || (caller_scope.contains(CapabilityScope::MUTATE)
            && !catalog
                .tools()
                .iter()
                .any(|tool| tool.name == DISPATCH_ACTION_TOOL))
        || (caller_scope.contains(CapabilityScope::CONTROL)
            && !catalog.tools().iter().any(|tool| tool.name == RECOVER_TOOL))
    {
        return Err(String::from(
            "negotiated composition did not retain the verified Runtime-v3 baseline",
        ));
    }
    Ok(())
}

fn combine_remote(
    mapping: &Mapping,
    remote: &BTreeMap<String, RemoteOffer>,
) -> Result<Option<RemoteOffer>, String> {
    let mut found = Vec::new();
    for operation in mapping.remote {
        let Some(offer) = remote.get(*operation) else {
            return Ok(None);
        };
        validate_revision(operation, &offer.revision)?;
        found.push(offer);
    }
    let first = found
        .first()
        .ok_or_else(|| String::from("empty negotiated mapping"))?;
    let mut combined = (*first).clone();
    for offer in found.iter().skip(1) {
        if offer.required_scope != combined.required_scope {
            return Err(String::from(
                "aliased Gateway operations disagree on required scope",
            ));
        }
        combined.scope = combined.scope.intersect(offer.scope);
        combined.wire_limits.max_request_bytes = combined
            .wire_limits
            .max_request_bytes
            .min(offer.wire_limits.max_request_bytes);
        combined.wire_limits.max_response_bytes = combined
            .wire_limits
            .max_response_bytes
            .min(offer.wire_limits.max_response_bytes);
        combined.content_limits.max_content_bytes = combined
            .content_limits
            .max_content_bytes
            .min(offer.content_limits.max_content_bytes);
        combined.content_limits.max_page_items = combined
            .content_limits
            .max_page_items
            .min(offer.content_limits.max_page_items);
    }
    Ok(Some(combined))
}

fn validate_revision(operation: &str, revision: &str) -> Result<(), String> {
    let expected = if operation == "game_information.live_observation_bootstrap" {
        "game-information-live-observation-bootstrap-v1"
    } else if operation.starts_with("game_information.lookup_binding.") {
        "game-information-lookup-binding-v1"
    } else if operation.starts_with("game_information.") {
        "game-information-query-v1"
    } else {
        "runtime-v3-gameplay"
    };
    if revision != expected {
        return Err(String::from(
            "Gateway operation revision does not match its pinned profile",
        ));
    }
    Ok(())
}

fn negotiation_error(error: NegotiationError) -> String {
    format!("negotiated MCP capability composition failed: {error}")
}
