// SPDX-License-Identifier: MIT

use super::super::http::{GAME_INFORMATION_MAX_RESPONSE_BYTES, RUNTIME_V3_MAX_RESPONSE_BYTES};
use super::{profile_for_name, profile_for_negotiation, save_profile_capability};
use sts2_mcp_server::{CapabilityLayer, CapabilityOwner, CapabilityScope, ToolCatalog};

#[test]
fn checkpoint_reference_profile_has_its_own_bounded_catalog() -> Result<(), String> {
    let profile = profile_for_name(Some("checkpoint-reference-v1"))?;
    assert_eq!(profile.catalog.revision, "checkpoint-reference-v1-mcp");
    assert_eq!(profile.max_response_bytes, 8192);
    assert!(!profile.requires_coop_native_peer_binding);
    Ok(())
}

#[test]
fn native_component_verifies_and_selects_its_catalog() -> Result<(), String> {
    let profile = profile_for_name(Some("coop-native-v1"))?;
    assert_eq!(profile.catalog.revision, "coop-native-v1-mcp");
    assert_eq!(profile.max_response_bytes, RUNTIME_V3_MAX_RESPONSE_BYTES);
    assert!(profile.requires_coop_native_peer_binding);
    Ok(())
}

#[test]
fn exact_restore_profile_verifies_its_artifacts_before_advertising_tools() -> Result<(), String> {
    let profile = profile_for_name(Some("exact-restore-v1"))?;
    assert_eq!(profile.catalog.revision, "exact-restore-v1-mcp");
    assert_eq!(profile.max_response_bytes, 16 * 1024);
    assert_eq!(profile.catalog.tools().len(), 5);
    Ok(())
}

#[test]
fn game_information_profile_verifies_and_selects_its_catalog() -> Result<(), String> {
    let profile = profile_for_name(Some("game-information-query-v1"))?;
    assert_eq!(profile.catalog.revision, "game-information-query-v1-mcp");
    assert_eq!(
        profile.max_response_bytes,
        GAME_INFORMATION_MAX_RESPONSE_BYTES
    );
    assert!(!profile.requires_coop_native_peer_binding);
    Ok(())
}

#[test]
fn executable_composition_profile_is_empty_without_external_evidence() -> Result<(), String> {
    let profile = profile_for_name(Some("negotiated-composition-v1"))?;
    assert_eq!(profile.catalog.revision, "negotiated-composition-v1-mcp");
    assert!(profile.catalog.tools().is_empty());
    assert!(!profile.requires_coop_native_peer_binding);
    Ok(())
}

#[test]
fn negotiated_profile_advertises_only_the_four_party_intersection() -> Result<(), String> {
    let profiles = [
        ToolCatalog::runtime_map_v1(),
        ToolCatalog::game_information(),
    ];
    let gateway = CapabilityLayer::from_catalogs(CapabilityOwner::Gateway, &profiles)
        .map_err(|error| error.to_string())?;
    let producer = CapabilityLayer::from_catalog_for(CapabilityOwner::Producer, &profiles[0])
        .map_err(|error| error.to_string())?;
    let profile = profile_for_negotiation(gateway, producer, CapabilityScope::READ)?;
    assert!(
        profile
            .catalog
            .tools()
            .iter()
            .any(|tool| tool.name == "sts2.map_snapshot")
    );
    assert!(
        !profile
            .catalog
            .tools()
            .iter()
            .any(|tool| tool.name == "sts2.game_information_search")
    );
    assert!(
        !profile
            .catalog
            .tools()
            .iter()
            .any(|tool| tool.name == "sts2.dispatch_action")
    );
    Ok(())
}

#[test]
fn save_profile_capability_modes_are_explicit_and_fail_closed() -> Result<(), String> {
    assert_eq!(save_profile_capability(None)?, (false, false));
    assert_eq!(save_profile_capability(Some("read-only"))?, (true, false));
    assert_eq!(save_profile_capability(Some("read-write"))?, (true, true));
    assert!(save_profile_capability(Some("owner-default")).is_err());
    let profile = profile_for_name(Some("save-profile-v1"))?;
    assert_eq!(profile.catalog.revision, "save-profile-v1-mcp");
    assert_eq!(
        profile.max_response_bytes,
        super::super::http::LEGACY_MAX_RESPONSE_BYTES
    );
    Ok(())
}
