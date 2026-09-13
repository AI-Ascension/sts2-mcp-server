// SPDX-License-Identifier: MIT

use sts2_mcp_server::{
    ToolCatalog, verify_coop_native_artifact, verify_coop_receipt_query_artifact,
    verify_game_information_artifact, verify_runtime_v4_expert_rest_action_artifact,
};

use super::http::{
    GAME_INFORMATION_MAX_RESPONSE_BYTES, LEGACY_MAX_RESPONSE_BYTES, MAP_MAX_RESPONSE_BYTES,
    RUNTIME_V3_MAX_RESPONSE_BYTES,
};

/// One selected executable profile: its tool catalog and the gateway response
/// body limit that applies to it. The MCP frame limit is a catalog property
/// (`ToolCatalog::max_frame_bytes`).
pub(crate) struct RuntimeProfile {
    pub(crate) catalog: ToolCatalog,
    pub(crate) max_response_bytes: usize,
    /// Native producer routes require a private transport binding that is not
    /// part of the serialized MCP or coop-native-v1 contracts.
    pub(crate) requires_coop_native_peer_binding: bool,
}

pub(crate) fn profile_from_environment() -> Result<RuntimeProfile, String> {
    let profile = match std::env::var("STS2_RUNTIME_PROFILE") {
        Ok(value) => Some(value),
        Err(std::env::VarError::NotPresent) => None,
        Err(std::env::VarError::NotUnicode(_)) => {
            return Err(String::from("STS2_RUNTIME_PROFILE is not valid UTF-8"));
        }
    };
    let name = profile.as_deref().unwrap_or("runtime-v1");
    let mut selected = profile_for_name(Some(name))?;
    if name == "save-profile-v1" {
        let (read, mutate) = save_profile_capability_from_environment()?;
        selected.catalog = ToolCatalog::save_profile_v1_with_capabilities(read, mutate);
    }
    Ok(selected)
}

pub(crate) fn profile_for_name(profile: Option<&str>) -> Result<RuntimeProfile, String> {
    match profile.unwrap_or("runtime-v1") {
        "runtime-v1" => Ok(RuntimeProfile {
            catalog: ToolCatalog::runtime_v1(),
            max_response_bytes: LEGACY_MAX_RESPONSE_BYTES,
            requires_coop_native_peer_binding: false,
        }),
        "runtime-v2" => Ok(RuntimeProfile {
            catalog: ToolCatalog::runtime_v2(),
            max_response_bytes: LEGACY_MAX_RESPONSE_BYTES,
            requires_coop_native_peer_binding: false,
        }),
        "runtime-v3-gameplay" => Ok(RuntimeProfile {
            catalog: ToolCatalog::runtime_v3_gameplay(),
            max_response_bytes: RUNTIME_V3_MAX_RESPONSE_BYTES,
            requires_coop_native_peer_binding: false,
        }),
        "runtime-v4-expert" => Ok(RuntimeProfile {
            catalog: ToolCatalog::runtime_v4_expert(),
            max_response_bytes: RUNTIME_V3_MAX_RESPONSE_BYTES,
            requires_coop_native_peer_binding: false,
        }),
        "runtime-v4-expert-rest-action" => Ok(RuntimeProfile {
            catalog: {
                verify_runtime_v4_expert_rest_action_artifact().map_err(|error| {
                    format!("Runtime-v4 expert REST-action artifact is invalid: {error}")
                })?;
                ToolCatalog::runtime_v4_expert_rest_action()
            },
            max_response_bytes: RUNTIME_V3_MAX_RESPONSE_BYTES,
            requires_coop_native_peer_binding: false,
        }),
        "checkpoint-reference-v1" => Ok(RuntimeProfile {
            catalog: ToolCatalog::checkpoint_reference_v1(),
            max_response_bytes: 8192,
            requires_coop_native_peer_binding: false,
        }),
        "runtime-map-v1" => Ok(RuntimeProfile {
            catalog: ToolCatalog::runtime_map_v1(),
            max_response_bytes: MAP_MAX_RESPONSE_BYTES,
            requires_coop_native_peer_binding: false,
        }),
        "coop-synchronization-v1" => Ok(RuntimeProfile {
            catalog: ToolCatalog::coop_synchronization(),
            max_response_bytes: 16 * 1024,
            requires_coop_native_peer_binding: false,
        }),
        "coop-receipt-query-v1" => Ok(RuntimeProfile {
            catalog: {
                verify_coop_receipt_query_artifact()
                    .map_err(|error| format!("co-op receipt-query artifact is invalid: {error}"))?;
                ToolCatalog::coop_receipt_query()
            },
            max_response_bytes: 16 * 1024,
            requires_coop_native_peer_binding: false,
        }),
        "seeded-run-v1" => Ok(RuntimeProfile {
            catalog: ToolCatalog::seeded_run_v1(),
            max_response_bytes: LEGACY_MAX_RESPONSE_BYTES,
            requires_coop_native_peer_binding: false,
        }),
        "coop-native-v1" => {
            verify_coop_native_artifact()
                .map_err(|error| format!("native co-op component artifact is invalid: {error}"))?;
            Ok(RuntimeProfile {
                catalog: ToolCatalog::coop_native(),
                max_response_bytes: RUNTIME_V3_MAX_RESPONSE_BYTES,
                requires_coop_native_peer_binding: true,
            })
        }
        "game-information-query-v1" => Ok(RuntimeProfile {
            catalog: {
                verify_game_information_artifact()
                    .map_err(|error| format!("game-information artifact is invalid: {error}"))?;
                ToolCatalog::game_information_query_v1()
            },
            max_response_bytes: GAME_INFORMATION_MAX_RESPONSE_BYTES,
            requires_coop_native_peer_binding: false,
        }),
        "save-profile-v1" => Ok(RuntimeProfile {
            catalog: ToolCatalog::save_profile_v1(),
            max_response_bytes: LEGACY_MAX_RESPONSE_BYTES,
            requires_coop_native_peer_binding: false,
        }),
        value => Err(format!(
            "STS2_RUNTIME_PROFILE must be runtime-v1, runtime-v2, runtime-v3-gameplay, runtime-v4-expert, runtime-v4-expert-rest-action, runtime-map-v1, coop-synchronization-v1, coop-receipt-query-v1, seeded-run-v1, coop-native-v1, checkpoint-reference-v1, game-information-query-v1, or save-profile-v1, got {value}"
        )),
    }
}

fn save_profile_capability_from_environment() -> Result<(bool, bool), String> {
    let value = [
        "STS2_SAVE_PROFILE_CAPABILITY",
        "STS2_SAVE_PROFILE_CAPABILITIES",
    ]
    .into_iter()
    .find_map(|name| match std::env::var(name) {
        Ok(value) => Some(Ok((name, value))),
        Err(std::env::VarError::NotPresent) => None,
        Err(std::env::VarError::NotUnicode(_)) => Some(Err(format!("{name} is not valid UTF-8"))),
    })
    .transpose()?
    .map(|(_, value)| value);
    save_profile_capability(value.as_deref())
}

fn save_profile_capability(value: Option<&str>) -> Result<(bool, bool), String> {
    match value
        .unwrap_or("unsupported")
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "unsupported" | "none" | "false" | "0" => Ok((false, false)),
        "read" | "read-only" | "readonly" => Ok((true, false)),
        "full" | "mutate" | "read-write" | "read_write" | "true" | "1" => Ok((true, true)),
        value => Err(format!(
            "STS2_SAVE_PROFILE_CAPABILITY must be unsupported, read, or read-write, got {value}"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::super::http::{GAME_INFORMATION_MAX_RESPONSE_BYTES, RUNTIME_V3_MAX_RESPONSE_BYTES};
    use super::{profile_for_name, save_profile_capability};

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
}
