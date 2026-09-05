// SPDX-License-Identifier: MIT

use sts2_mcp_server::ToolCatalog;

use super::http::{LEGACY_MAX_RESPONSE_BYTES, RUNTIME_V3_MAX_RESPONSE_BYTES};

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
        value => Err(format!(
            "STS2_RUNTIME_PROFILE must be runtime-v1, runtime-v2, or runtime-v3-gameplay, got {value}"
        )),
    }
}
