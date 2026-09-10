// SPDX-License-Identifier: MIT

use sts2_mcp_server::{
    ToolCatalog, verify_coop_native_artifact, verify_coop_receipt_query_artifact,
    verify_runtime_v4_expert_rest_action_artifact,
};

use super::http::{
    LEGACY_MAX_RESPONSE_BYTES, MAP_MAX_RESPONSE_BYTES, RUNTIME_V3_MAX_RESPONSE_BYTES,
};

/// One selected executable profile: its tool catalog and the gateway response
/// body limit that applies to it. The MCP frame limit is a catalog property
/// (`ToolCatalog::max_frame_bytes`).
pub(crate) struct RuntimeProfile {
    pub(crate) catalog: ToolCatalog,
    pub(crate) max_response_bytes: usize,
}

pub(crate) fn profile_from_environment() -> Result<RuntimeProfile, String> {
    let profile = match std::env::var("STS2_RUNTIME_PROFILE") {
        Ok(value) => Some(value),
        Err(std::env::VarError::NotPresent) => None,
        Err(std::env::VarError::NotUnicode(_)) => {
            return Err(String::from("STS2_RUNTIME_PROFILE is not valid UTF-8"));
        }
    };
    profile_for_name(profile.as_deref())
}

pub(crate) fn profile_for_name(profile: Option<&str>) -> Result<RuntimeProfile, String> {
    match profile.unwrap_or("runtime-v1") {
        "runtime-v1" => Ok(RuntimeProfile {
            catalog: ToolCatalog::runtime_v1(),
            max_response_bytes: LEGACY_MAX_RESPONSE_BYTES,
        }),
        "runtime-v2" => Ok(RuntimeProfile {
            catalog: ToolCatalog::runtime_v2(),
            max_response_bytes: LEGACY_MAX_RESPONSE_BYTES,
        }),
        "runtime-v3-gameplay" => Ok(RuntimeProfile {
            catalog: ToolCatalog::runtime_v3_gameplay(),
            max_response_bytes: RUNTIME_V3_MAX_RESPONSE_BYTES,
        }),
        "runtime-v4-expert" => Ok(RuntimeProfile {
            catalog: ToolCatalog::runtime_v4_expert(),
            max_response_bytes: RUNTIME_V3_MAX_RESPONSE_BYTES,
        }),
        "runtime-v4-expert-rest-action" => Ok(RuntimeProfile {
            catalog: {
                verify_runtime_v4_expert_rest_action_artifact().map_err(|error| {
                    format!("Runtime-v4 expert REST-action artifact is invalid: {error}")
                })?;
                ToolCatalog::runtime_v4_expert_rest_action()
            },
            max_response_bytes: RUNTIME_V3_MAX_RESPONSE_BYTES,
        }),
        "runtime-map-v1" => Ok(RuntimeProfile {
            catalog: ToolCatalog::runtime_map_v1(),
            max_response_bytes: MAP_MAX_RESPONSE_BYTES,
        }),
        "coop-synchronization-v1" => Ok(RuntimeProfile {
            catalog: ToolCatalog::coop_synchronization(),
            max_response_bytes: 16 * 1024,
        }),
        "coop-receipt-query-v1" => Ok(RuntimeProfile {
            catalog: {
                verify_coop_receipt_query_artifact()
                    .map_err(|error| format!("co-op receipt-query artifact is invalid: {error}"))?;
                ToolCatalog::coop_receipt_query()
            },
            max_response_bytes: 16 * 1024,
        }),
        "seeded-run-v1" => Ok(RuntimeProfile {
            catalog: ToolCatalog::seeded_run_v1(),
            max_response_bytes: LEGACY_MAX_RESPONSE_BYTES,
        }),
        "coop-native-v1" => {
            verify_coop_native_artifact()
                .map_err(|error| format!("native co-op component artifact is invalid: {error}"))?;
            Ok(RuntimeProfile {
                catalog: ToolCatalog::coop_native(),
                max_response_bytes: RUNTIME_V3_MAX_RESPONSE_BYTES,
            })
        }
        value => Err(format!(
            "STS2_RUNTIME_PROFILE must be runtime-v1, runtime-v2, runtime-v3-gameplay, runtime-v4-expert, runtime-v4-expert-rest-action, runtime-map-v1, coop-synchronization-v1, coop-receipt-query-v1, seeded-run-v1, or coop-native-v1, got {value}"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::super::http::RUNTIME_V3_MAX_RESPONSE_BYTES;
    use super::profile_for_name;

    #[test]
    fn native_component_verifies_and_selects_its_catalog() -> Result<(), String> {
        let profile = profile_for_name(Some("coop-native-v1"))?;
        assert_eq!(profile.catalog.revision, "coop-native-v1-mcp");
        assert_eq!(profile.max_response_bytes, RUNTIME_V3_MAX_RESPONSE_BYTES);
        Ok(())
    }
}
