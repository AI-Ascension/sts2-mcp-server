// SPDX-License-Identifier: MIT

use super::*;
use crate::catalog::{
    DISPATCH_ACTION_TOOL, GAME_INFORMATION_DETAIL_TOOL, GAME_INFORMATION_SEARCH_TOOL,
    LEGAL_ACTIONS_TOOL, MAP_SNAPSHOT_TOOL, OBSERVE_TOOL, ToolCatalog,
};

fn composition_layers(
    profiles: &[ToolCatalog],
) -> Result<(CapabilityLayer, CapabilityLayer), NegotiationError> {
    Ok((
        CapabilityLayer::from_catalogs(CapabilityOwner::Gateway, profiles)?,
        CapabilityLayer::from_catalogs(CapabilityOwner::Producer, profiles)?,
    ))
}

#[test]
fn composes_gameplay_map_and_lookup_tools_with_local_discovery() -> Result<(), NegotiationError> {
    let profiles = [
        ToolCatalog::runtime_map_v1(),
        ToolCatalog::game_information(),
    ];
    let (gateway, producer) = composition_layers(&profiles)?;
    let catalog =
        ToolCatalog::compose_profiles(&profiles, gateway, producer, CapabilityScope::ALL)?;

    assert_eq!(catalog.revision, NEGOTIATED_COMPOSITION_REVISION);
    assert!(catalog.tools().iter().all(|tool| {
        catalog
            .tools()
            .iter()
            .filter(|other| other.name == tool.name)
            .count()
            == 1
    }));
    for operation in [
        OBSERVE_TOOL,
        LEGAL_ACTIONS_TOOL,
        DISPATCH_ACTION_TOOL,
        MAP_SNAPSHOT_TOOL,
        GAME_INFORMATION_SEARCH_TOOL,
        GAME_INFORMATION_DETAIL_TOOL,
        CAPABILITY_DISCOVERY_TOOL,
    ] {
        assert!(catalog.tools().iter().any(|tool| tool.name == operation));
        assert!(
            catalog
                .composition()
                .and_then(|set| set.available(operation))
                .is_some()
        );
    }
    Ok(())
}

#[test]
fn overlapping_profile_descriptors_keep_their_operation_revision() -> Result<(), NegotiationError> {
    let profiles = [
        ToolCatalog::runtime_v3_gameplay(),
        ToolCatalog::runtime_map_v1(),
    ];
    let (gateway, producer) = composition_layers(&profiles)?;
    let catalog =
        ToolCatalog::compose_profiles(&profiles, gateway, producer, CapabilityScope::ALL)?;

    assert!(catalog.tools().iter().any(|tool| tool.name == OBSERVE_TOOL));
    assert!(
        catalog
            .tools()
            .iter()
            .any(|tool| tool.name == MAP_SNAPSHOT_TOOL)
    );
    assert_eq!(
        catalog
            .composition()
            .and_then(|set| set.available(OBSERVE_TOOL))
            .map(|operation| operation.revision.as_str()),
        Some("runtime-v3-gameplay-mcp")
    );
    Ok(())
}

#[test]
fn missing_producer_feature_is_reported_without_shadowing_remaining_tools()
-> Result<(), NegotiationError> {
    let profiles = [
        ToolCatalog::runtime_map_v1(),
        ToolCatalog::game_information(),
    ];
    let gateway = CapabilityLayer::from_catalogs(CapabilityOwner::Gateway, &profiles)?;
    let producer = CapabilityLayer::from_catalog_for(CapabilityOwner::Producer, &profiles[0])?;
    let catalog =
        ToolCatalog::compose_profiles(&profiles, gateway, producer, CapabilityScope::ALL)?;

    assert!(
        catalog
            .tools()
            .iter()
            .any(|tool| tool.name == MAP_SNAPSHOT_TOOL)
    );
    assert!(
        !catalog
            .tools()
            .iter()
            .any(|tool| tool.name == GAME_INFORMATION_SEARCH_TOOL)
    );
    let Some(composition) = catalog.composition() else {
        return Err(NegotiationError::NoCompatibleOperations);
    };
    let Some(unavailable) = composition
        .unavailable()
        .find(|capability| capability.operation == GAME_INFORMATION_SEARCH_TOOL)
    else {
        return Err(NegotiationError::NoCompatibleOperations);
    };
    assert_eq!(unavailable.reason, UnavailableReason::MissingProducer);
    Ok(())
}

#[test]
fn revision_conflict_fails_negotiation() -> Result<(), NegotiationError> {
    let mcp = CapabilityLayer::new(CapabilityOwner::Mcp, "mcp-v1").with_offer(
        CapabilityOffer::supported(
            "example.read",
            "example-v1-mcp",
            CapabilityGroup::StaticReference,
            CapabilityScope::READ,
            ToolLimits::bounded(32, 64, 64, 1),
        ),
    )?;
    let gateway = CapabilityLayer::new(CapabilityOwner::Gateway, "gateway-v1").with_offer(
        CapabilityOffer::supported(
            "example.read",
            "example-v2-mcp",
            CapabilityGroup::StaticReference,
            CapabilityScope::READ,
            ToolLimits::bounded(32, 64, 64, 1),
        ),
    )?;
    let producer = CapabilityLayer::new(CapabilityOwner::Producer, "producer-v1").with_offer(
        CapabilityOffer::supported(
            "example.read",
            "example-v1-mcp",
            CapabilityGroup::StaticReference,
            CapabilityScope::READ,
            ToolLimits::bounded(32, 64, 64, 1),
        ),
    )?;

    assert!(matches!(
        NegotiatedCapabilitySet::negotiate(&NegotiationRequest::new(
            mcp,
            gateway,
            producer,
            CapabilityScope::ALL,
        )),
        Err(NegotiationError::RevisionConflict { .. })
    ));
    Ok(())
}

#[test]
fn scope_and_limits_are_intersected() -> Result<(), NegotiationError> {
    let offer = |owner, scope, limits| -> Result<CapabilityLayer, NegotiationError> {
        CapabilityLayer::new(owner, "v1").with_offer(
            CapabilityOffer::supported(
                "example.read",
                "example-v1-mcp",
                CapabilityGroup::StaticReference,
                CapabilityScope::READ,
                limits,
            )
            .with_scope(scope),
        )
    };
    let set = NegotiatedCapabilitySet::negotiate(&NegotiationRequest::new(
        offer(
            CapabilityOwner::Mcp,
            CapabilityScope::READ | CapabilityScope::PROFILE,
            ToolLimits::bounded(64, 256, 512, 32),
        )?,
        offer(
            CapabilityOwner::Gateway,
            CapabilityScope::READ,
            ToolLimits::bounded(32, 128, 256, 16),
        )?,
        offer(
            CapabilityOwner::Producer,
            CapabilityScope::READ,
            ToolLimits::bounded(16, 64, 128, 8),
        )?,
        CapabilityScope::READ | CapabilityScope::PROFILE,
    ))?;
    let Some(operation) = set.available("example.read") else {
        return Err(NegotiationError::NoCompatibleOperations);
    };
    assert_eq!(operation.effective_scope, CapabilityScope::READ);
    assert_eq!(operation.limits, ToolLimits::bounded(16, 64, 128, 8));
    Ok(())
}
