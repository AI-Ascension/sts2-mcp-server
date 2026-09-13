// SPDX-License-Identifier: MIT

use std::collections::{BTreeMap, BTreeSet};

use super::offers::{CapabilityLayer, CapabilityOffer, NegotiationRequest};
use super::types::{
    CAPABILITY_DISCOVERY_TOOL, CapabilityScope, NEGOTIATED_COMPOSITION_REVISION,
    NegotiatedCapabilitySet, NegotiatedOperation, NegotiationError, UnavailableCapability,
    UnavailableReason,
};
use crate::catalog::{CapabilityCatalog, ToolCatalog, ToolDescriptor};

impl NegotiatedCapabilitySet {
    pub fn negotiate(request: &NegotiationRequest) -> Result<Self, NegotiationError> {
        let set = negotiate(request)?;
        if set.operations.is_empty() {
            Err(NegotiationError::NoCompatibleOperations)
        } else {
            Ok(set)
        }
    }
}

pub(crate) fn compose_profiles(
    profiles: &[ToolCatalog],
    gateway: CapabilityLayer,
    producer: CapabilityLayer,
    caller_scope: CapabilityScope,
) -> Result<ToolCatalog, NegotiationError> {
    let mut merged = merge_profiles(profiles)?;
    merged
        .tools
        .push(super::profile::capability_discovery_descriptor());
    let mcp = CapabilityLayer::from_catalog(&merged)?;
    let set = negotiate(&NegotiationRequest::new(
        mcp,
        gateway,
        producer,
        caller_scope,
    ))?;
    let allowed: BTreeSet<&str> = set
        .operations()
        .map(|operation| operation.operation.as_str())
        .collect();
    let mut tools: Vec<ToolDescriptor> = merged
        .tools
        .into_iter()
        .filter(|tool| allowed.contains(tool.name.as_str()))
        .collect();
    tools.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(ToolCatalog {
        revision: NEGOTIATED_COMPOSITION_REVISION.to_owned(),
        capabilities: CapabilityCatalog::default(),
        tools,
        composition: Some(set),
    })
}

fn merge_profiles(profiles: &[ToolCatalog]) -> Result<ToolCatalog, NegotiationError> {
    if profiles.is_empty() {
        return Err(NegotiationError::NoCompatibleOperations);
    }
    let mut tools: BTreeMap<String, (ToolDescriptor, String)> = BTreeMap::new();
    for profile in profiles {
        let mut profile_names = BTreeSet::new();
        for tool in &profile.tools {
            if !profile_names.insert(tool.name.clone()) {
                return Err(NegotiationError::DuplicateOperation(tool.name.clone()));
            }
            // Deduplicate on both the descriptor and the source operation
            // revision. The merged catalog later rebuilds offers from the
            // composition revision, so a conflicting per-operation source
            // revision would otherwise be silently discarded.
            let revision = super::profile::operation_revision(&tool.name, &profile.revision);
            if let Some((previous, previous_revision)) = tools.get(&tool.name) {
                if previous != tool || *previous_revision != revision {
                    return Err(NegotiationError::RevisionConflict {
                        operation: tool.name.clone(),
                        left: previous_revision.clone(),
                        right: revision,
                    });
                }
            } else {
                tools.insert(tool.name.clone(), (tool.clone(), revision));
            }
        }
    }
    Ok(ToolCatalog {
        revision: NEGOTIATED_COMPOSITION_REVISION.to_owned(),
        capabilities: CapabilityCatalog::default(),
        tools: tools.into_values().map(|(tool, _)| tool).collect(),
        composition: None,
    })
}

fn negotiate(request: &NegotiationRequest) -> Result<NegotiatedCapabilitySet, NegotiationError> {
    let mut operations = BTreeMap::new();
    let mut unavailable = BTreeMap::new();
    for mcp_offer in request.mcp.operations() {
        let operation = mcp_offer.operation.as_str();
        if operation == CAPABILITY_DISCOVERY_TOOL {
            let effective_scope = request.caller_scope.intersect(mcp_offer.scope);
            if !mcp_offer.supported {
                insert_unavailable(
                    &mut unavailable,
                    mcp_offer,
                    mcp_offer
                        .unavailable_reason
                        .clone()
                        .unwrap_or(UnavailableReason::MissingMcp),
                );
            } else if !effective_scope.contains(mcp_offer.required_scope) {
                insert_unavailable(
                    &mut unavailable,
                    mcp_offer,
                    if request.caller_scope.contains(mcp_offer.required_scope) {
                        UnavailableReason::PermissionDenied
                    } else {
                        UnavailableReason::ScopeDenied
                    },
                );
            } else {
                operations.insert(
                    mcp_offer.operation.clone(),
                    NegotiatedOperation {
                        operation: mcp_offer.operation.clone(),
                        revision: mcp_offer.revision.clone(),
                        group: mcp_offer.group,
                        required_scope: mcp_offer.required_scope,
                        effective_scope,
                        limits: mcp_offer.limits,
                    },
                );
            }
            continue;
        }
        let Some(gateway_offer) = request.gateway.offer(operation) else {
            insert_unavailable(
                &mut unavailable,
                mcp_offer,
                UnavailableReason::MissingGateway,
            );
            continue;
        };
        let Some(producer_offer) = request.producer.offer(operation) else {
            insert_unavailable(
                &mut unavailable,
                mcp_offer,
                UnavailableReason::MissingProducer,
            );
            continue;
        };
        for other in [gateway_offer, producer_offer] {
            if !compatible_revision(&mcp_offer.revision, &other.revision) {
                return Err(NegotiationError::RevisionConflict {
                    operation: mcp_offer.operation.clone(),
                    left: mcp_offer.revision.clone(),
                    right: other.revision.clone(),
                });
            }
        }
        if !mcp_offer.supported {
            insert_unavailable(
                &mut unavailable,
                mcp_offer,
                mcp_offer
                    .unavailable_reason
                    .clone()
                    .unwrap_or(UnavailableReason::MissingMcp),
            );
            continue;
        }
        if !gateway_offer.supported {
            insert_unavailable(
                &mut unavailable,
                mcp_offer,
                gateway_offer
                    .unavailable_reason
                    .clone()
                    .unwrap_or(UnavailableReason::MissingGateway),
            );
            continue;
        }
        if !producer_offer.supported {
            insert_unavailable(
                &mut unavailable,
                mcp_offer,
                producer_offer
                    .unavailable_reason
                    .clone()
                    .unwrap_or(UnavailableReason::ProducerUnsupported),
            );
            continue;
        }
        let effective_scope = request
            .caller_scope
            .intersect(mcp_offer.scope)
            .intersect(gateway_offer.scope)
            .intersect(producer_offer.scope);
        if !effective_scope.contains(mcp_offer.required_scope) {
            let reason = if request.caller_scope.contains(mcp_offer.required_scope) {
                UnavailableReason::PermissionDenied
            } else {
                UnavailableReason::ScopeDenied
            };
            insert_unavailable(&mut unavailable, mcp_offer, reason);
            continue;
        }
        operations.insert(
            mcp_offer.operation.clone(),
            NegotiatedOperation {
                operation: mcp_offer.operation.clone(),
                revision: mcp_offer.revision.clone(),
                group: mcp_offer.group,
                required_scope: mcp_offer.required_scope,
                effective_scope,
                limits: mcp_offer
                    .limits
                    .minimum(gateway_offer.limits)
                    .minimum(producer_offer.limits),
            },
        );
    }
    Ok(NegotiatedCapabilitySet {
        revision: NEGOTIATED_COMPOSITION_REVISION.to_owned(),
        caller_scope: request.caller_scope,
        operations,
        unavailable,
    })
}

fn insert_unavailable(
    unavailable: &mut BTreeMap<String, UnavailableCapability>,
    offer: &CapabilityOffer,
    reason: UnavailableReason,
) {
    unavailable.insert(
        offer.operation.clone(),
        UnavailableCapability {
            operation: offer.operation.clone(),
            group: offer.group,
            reason,
            required_scope: offer.required_scope,
        },
    );
}

fn compatible_revision(left: &str, right: &str) -> bool {
    left == right
        || left.strip_suffix("-mcp") == Some(right)
        || right.strip_suffix("-mcp") == Some(left)
}
