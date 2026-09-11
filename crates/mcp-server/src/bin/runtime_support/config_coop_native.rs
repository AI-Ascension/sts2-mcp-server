// SPDX-License-Identifier: MIT

use super::safe_header_value;

/// Runtime-only native-route authorization material. Neither field is placed
/// in an MCP catalog, tool argument, or `coop-native-v1` envelope.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct CoopNativePeerBinding {
    pub(super) token: String,
    pub(super) peer_id: String,
}

pub(super) fn from_environment(required: bool) -> Result<Option<CoopNativePeerBinding>, String> {
    let token = optional("STS2_COOP_NATIVE_PEER_TOKEN")?;
    let peer_id = optional("STS2_COOP_NATIVE_PEER_ID")?;
    from_values(required, token.as_deref(), peer_id.as_deref())
}

pub(super) fn from_values(
    required: bool,
    token: Option<&str>,
    peer_id: Option<&str>,
) -> Result<Option<CoopNativePeerBinding>, String> {
    match (token, peer_id) {
        (None, None) if required => Err(String::from(
            "STS2_COOP_NATIVE_PEER_TOKEN and STS2_COOP_NATIVE_PEER_ID are required for STS2_RUNTIME_PROFILE=coop-native-v1",
        )),
        (None, None) => Ok(None),
        (Some(_), None) | (None, Some(_)) => Err(String::from(
            "STS2_COOP_NATIVE_PEER_TOKEN and STS2_COOP_NATIVE_PEER_ID must be configured together",
        )),
        (Some(token), Some(peer_id)) => {
            if !safe_header_value(token) {
                return Err(String::from(
                    "STS2_COOP_NATIVE_PEER_TOKEN is empty, unsafe, or oversized",
                ));
            }
            if !canonical_peer_identity(peer_id) {
                return Err(String::from(
                    "STS2_COOP_NATIVE_PEER_ID is not a canonical coop-native-v1 peer identity",
                ));
            }
            if token == peer_id {
                return Err(String::from(
                    "STS2_COOP_NATIVE_PEER_TOKEN must be distinct from STS2_COOP_NATIVE_PEER_ID",
                ));
            }
            Ok(Some(CoopNativePeerBinding {
                token: String::from(token),
                peer_id: String::from(peer_id),
            }))
        }
    }
}

fn optional(name: &str) -> Result<Option<String>, String> {
    match std::env::var(name) {
        Ok(value) if !value.is_empty() => Ok(Some(value)),
        Ok(_) => Err(format!("{name} must not be empty")),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(std::env::VarError::NotUnicode(_)) => Err(format!("{name} is not valid UTF-8")),
    }
}

fn canonical_peer_identity(value: &str) -> bool {
    value
        .strip_prefix("peer:")
        .is_some_and(|suffix| (5..=507).contains(&suffix.len()) && value.len() <= 512)
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':' | b'/')
        })
}
