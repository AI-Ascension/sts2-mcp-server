// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::json::JsonValue;

use super::offers::{CapabilityLayer, CapabilityOffer};
use super::types::{
    CAPABILITY_DISCOVERY_TOOL, CapabilityGroup, CapabilityOwner, CapabilityScope,
    NEGOTIATED_COMPOSITION_REVISION, NegotiationError, ToolLimits,
};
use crate::catalog::{
    CHECKPOINT_REFERENCE_TOOL, COOP_RECEIPT_QUERY_TOOL, DISPATCH_ACTION_TOOL,
    GAME_INFORMATION_AVAILABILITY_TOOL, GAME_INFORMATION_CAPABILITIES_TOOL,
    GAME_INFORMATION_DETAIL_TOOL, GAME_INFORMATION_GET_TOOL, GAME_INFORMATION_LIST_TOOL,
    GAME_INFORMATION_SEARCH_TOOL, LEGAL_ACTIONS_TOOL, MAP_SNAPSHOT_TOOL, OBSERVE_TOOL,
    RECOVER_TOOL, REOBSERVE_TOOL, ToolCatalog, ToolDescriptor, WAIT_FOR_TRANSITION_TOOL,
};

pub(crate) fn layer_from_catalog(
    owner: CapabilityOwner,
    catalog: &ToolCatalog,
) -> Result<CapabilityLayer, NegotiationError> {
    let mut layer = CapabilityLayer::new(owner, catalog.revision.clone());
    for tool in &catalog.tools {
        let (group, required_scope) = operation_shape(&tool.name);
        let revision = operation_revision(&tool.name, &catalog.revision);
        layer.insert(CapabilityOffer::supported(
            tool.name.clone(),
            revision,
            group,
            required_scope,
            limits_for(catalog, &tool.name),
        ))?;
    }
    Ok(layer)
}

pub(crate) fn layer_from_catalogs(
    owner: CapabilityOwner,
    catalogs: &[ToolCatalog],
) -> Result<CapabilityLayer, NegotiationError> {
    let mut layer = CapabilityLayer::new(owner, NEGOTIATED_COMPOSITION_REVISION);
    for catalog in catalogs {
        let source = layer_from_catalog(owner, catalog)?;
        for offer in source.operations() {
            if let Some(existing) = layer.offer(&offer.operation) {
                if existing != offer {
                    return Err(NegotiationError::RevisionConflict {
                        operation: offer.operation.clone(),
                        left: existing.revision.clone(),
                        right: offer.revision.clone(),
                    });
                }
                continue;
            }
            layer.insert(offer.clone())?;
        }
    }
    Ok(layer)
}

fn operation_shape(name: &str) -> (CapabilityGroup, CapabilityScope) {
    if name == CAPABILITY_DISCOVERY_TOOL {
        return (CapabilityGroup::ProfileReads, CapabilityScope::READ);
    }
    if matches!(
        name,
        GAME_INFORMATION_CAPABILITIES_TOOL
            | GAME_INFORMATION_LIST_TOOL
            | GAME_INFORMATION_SEARCH_TOOL
            | GAME_INFORMATION_GET_TOOL
    ) {
        return (CapabilityGroup::StaticReference, CapabilityScope::READ);
    }
    if matches!(
        name,
        GAME_INFORMATION_DETAIL_TOOL | GAME_INFORMATION_AVAILABILITY_TOOL
    ) {
        return (CapabilityGroup::LiveDetails, CapabilityScope::READ);
    }
    if name == MAP_SNAPSHOT_TOOL {
        return (CapabilityGroup::Maps, CapabilityScope::READ);
    }
    if name == COOP_RECEIPT_QUERY_TOOL || name.contains("research") {
        return (CapabilityGroup::ResearchReads, CapabilityScope::RESEARCH);
    }
    if name == CHECKPOINT_REFERENCE_TOOL || name.contains("profile") {
        return (CapabilityGroup::ProfileReads, CapabilityScope::PROFILE);
    }
    if name.starts_with("sts2.") {
        return (
            CapabilityGroup::GameplayActions,
            if name == DISPATCH_ACTION_TOOL {
                CapabilityScope::MUTATE
            } else {
                CapabilityScope::READ
            },
        );
    }
    (CapabilityGroup::ProfileReads, CapabilityScope::PROFILE)
}

fn operation_revision(name: &str, profile_revision: &str) -> String {
    if name == CAPABILITY_DISCOVERY_TOOL {
        return NEGOTIATED_COMPOSITION_REVISION.to_owned();
    }
    if name == MAP_SNAPSHOT_TOOL {
        return "runtime-map-v1-mcp".to_owned();
    }
    if name.starts_with("sts2.game_information_") {
        return "game-information-query-v1-mcp".to_owned();
    }
    if matches!(
        name,
        OBSERVE_TOOL
            | LEGAL_ACTIONS_TOOL
            | DISPATCH_ACTION_TOOL
            | WAIT_FOR_TRANSITION_TOOL
            | REOBSERVE_TOOL
            | RECOVER_TOOL
    ) && matches!(
        profile_revision,
        "runtime-map-v1-mcp" | "runtime-v3-gameplay-mcp" | NEGOTIATED_COMPOSITION_REVISION
    ) {
        return "runtime-v3-gameplay-mcp".to_owned();
    }
    profile_revision.to_owned()
}

fn limits_for(catalog: &ToolCatalog, operation: &str) -> ToolLimits {
    let frame = catalog.max_frame_bytes();
    let max_response_bytes = if catalog.revision == "runtime-map-v1-mcp" {
        if operation == MAP_SNAPSHOT_TOOL {
            256 * 1024
        } else {
            128 * 1024
        }
    } else if catalog.revision.starts_with("game-information") {
        crate::GAME_INFORMATION_MAX_MESSAGE_BYTES
    } else {
        frame.min(128 * 1024)
    };
    ToolLimits::bounded(
        16 * 1024,
        max_response_bytes,
        max_response_bytes,
        if catalog.revision.starts_with("game-information") {
            crate::GAME_INFORMATION_MAX_PAGE_ITEMS as usize
        } else {
            128
        },
    )
}

pub(crate) fn capability_discovery_descriptor() -> ToolDescriptor {
    ToolDescriptor {
        name: CAPABILITY_DISCOVERY_TOOL.to_owned(),
        description: String::from(
            "Read the negotiated capability set and bounded fallbacks. This local discovery call never forwards or escalates caller scope.",
        ),
        input_schema: JsonValue::object([
            ("type".to_owned(), JsonValue::string("object")),
            ("additionalProperties".to_owned(), JsonValue::Bool(false)),
            ("properties".to_owned(), JsonValue::Object(BTreeMap::new())),
        ]),
    }
}
