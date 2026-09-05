// SPDX-License-Identifier: MIT

use sts2_mcp_server::ToolCatalog;

pub(crate) fn catalog_from_environment() -> Result<ToolCatalog, String> {
    let profile = match std::env::var("STS2_RUNTIME_PROFILE") {
        Ok(value) => Some(value),
        Err(std::env::VarError::NotPresent) => None,
        Err(std::env::VarError::NotUnicode(_)) => {
            return Err(String::from("STS2_RUNTIME_PROFILE is not valid UTF-8"));
        }
    };
    catalog_for_profile(profile.as_deref())
}

pub(crate) fn catalog_for_profile(profile: Option<&str>) -> Result<ToolCatalog, String> {
    match profile.unwrap_or("runtime-v1") {
        "runtime-v1" => Ok(ToolCatalog::runtime_v1()),
        "runtime-v2" => Ok(ToolCatalog::runtime_v2()),
        "runtime-v3-gameplay" => Ok(ToolCatalog::runtime_v3_gameplay()),
        value => Err(format!(
            "STS2_RUNTIME_PROFILE must be runtime-v1, runtime-v2, or runtime-v3-gameplay, got {value}"
        )),
    }
}
