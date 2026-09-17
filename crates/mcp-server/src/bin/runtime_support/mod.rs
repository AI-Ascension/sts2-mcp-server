// SPDX-License-Identifier: MIT

use std::collections::HashSet;
use std::net::SocketAddr;

mod binding;
#[path = "config_coop_native.rs"]
mod coop_native_config;
use coop_native_config::CoopNativePeerBinding;
mod exchange;
mod gateway_adapter;
mod http;
mod negotiated_discovery;
mod negotiated_offers;
mod negotiated_startup;
mod profiles;
const DEFAULT_MCP_SESSION_ID: &str = "mcp-session-1";

pub(crate) fn startup_from_environment() -> Result<(RuntimeConfig, profiles::RuntimeProfile), String>
{
    if profiles::runtime_profile_name()? == "negotiated-composition-v1" {
        let config = RuntimeConfig::from_environment(false, false)?;
        let profile = negotiated_startup::bootstrap(&config)?;
        return Ok((config, profile));
    }
    let profile = profiles::profile_from_environment()?;
    let config = RuntimeConfig::from_environment(
        profile.requires_coop_native_peer_binding,
        profile.catalog.revision == "exact-restore-v1-mcp",
    )?;
    Ok((config, profile))
}

pub(crate) struct RuntimeConfig {
    pub(crate) gateway_address: SocketAddr,
    pub(crate) gateway_token: String,
    pub(crate) instance_id: String,
    pub(crate) caller_id: String,
    pub(crate) session_id: String,
    pub(crate) mcp_session_id: String,
    pub(crate) lease_id: String,
    pub(crate) lease_epoch: i64,
    recovery_token: Option<String>,
    exact_restore_profile: bool,
    coop_native_peer_binding: Option<CoopNativePeerBinding>,
}

impl RuntimeConfig {
    pub(crate) fn from_environment(
        requires_coop_native_peer_binding: bool,
        exact_restore_profile: bool,
    ) -> Result<Self, String> {
        let gateway_address = gateway_address(&required_or_default(
            "STS2_GATEWAY_ADDR",
            "127.0.0.1:15525",
        )?)?;
        let (gateway_token, recovery_token) = if exact_restore_profile {
            let token = required("STS2_RECOVERY_TOKEN")?;
            if !safe_token(&token) {
                return Err(String::from(
                    "STS2_RECOVERY_TOKEN is empty, unsafe, or oversized",
                ));
            }
            (String::new(), Some(token))
        } else {
            let token = required("STS2_GATEWAY_TOKEN")?;
            if !safe_token(&token) {
                return Err(String::from(
                    "STS2_GATEWAY_TOKEN is empty, unsafe, or oversized",
                ));
            }
            (token, None)
        };
        let instance_id = required_or_default("STS2_INSTANCE_ID", "instance-1")?;
        let caller_id = required_or_default("STS2_CALLER_ID", "harness")?;
        let session_id = required_or_default("STS2_SESSION_ID", "session-1")?;
        let mcp_session_id = required_or_default("STS2_MCP_SESSION_ID", DEFAULT_MCP_SESSION_ID)?;
        let lease_id = required_or_default("STS2_LEASE_ID", "lease-1")?;
        let lease_epoch = required_or_default("STS2_LEASE_EPOCH", "1")?
            .parse::<i64>()
            .map_err(|_| String::from("STS2_LEASE_EPOCH must be a nonnegative integer"))?;
        if lease_epoch < 0 {
            return Err(String::from("STS2_LEASE_EPOCH must be nonnegative"));
        }
        for (name, value) in [
            ("STS2_INSTANCE_ID", &instance_id),
            ("STS2_CALLER_ID", &caller_id),
            ("STS2_SESSION_ID", &session_id),
            ("STS2_MCP_SESSION_ID", &mcp_session_id),
            ("STS2_LEASE_ID", &lease_id),
        ] {
            if !safe_header_value(value) {
                return Err(format!("{name} is empty, unsafe, or oversized"));
            }
        }
        // The native credential pair is meaningful only to the native profile.
        // Do not make unrelated profiles fail because an operator has a partial
        // native configuration in their process environment.
        let coop_native_peer_binding = if requires_coop_native_peer_binding {
            coop_native_config::from_environment(true)?
        } else {
            None
        };
        Ok(Self {
            gateway_address,
            gateway_token,
            instance_id,
            caller_id,
            session_id,
            mcp_session_id,
            lease_id,
            lease_epoch,
            recovery_token,
            exact_restore_profile,
            coop_native_peer_binding,
        })
    }

    pub(crate) fn native_peer_id(&self) -> Option<&str> {
        self.coop_native_peer_binding
            .as_ref()
            .map(|binding| binding.peer_id.as_str())
    }
}

pub(crate) struct RuntimeGatewayAdapter {
    config: RuntimeConfig,
    max_response_bytes: usize,
    wire_limits: std::collections::BTreeMap<String, profiles::GatewayWireLimits>,
    enforce_wire_limits: bool,
    lookup_only_operations: HashSet<String>,
}

impl RuntimeGatewayAdapter {
    /// `max_response_bytes` is the selected profile's gateway body limit
    /// (`RuntimeProfile::max_response_bytes`); legacy profiles keep 64 KiB.
    #[cfg(test)]
    pub(crate) fn new(config: RuntimeConfig, max_response_bytes: usize) -> Self {
        Self::new_with_wire_limits(
            config,
            max_response_bytes,
            std::collections::BTreeMap::new(),
            false,
        )
    }

    pub(crate) fn new_with_wire_limits(
        config: RuntimeConfig,
        max_response_bytes: usize,
        wire_limits: std::collections::BTreeMap<String, profiles::GatewayWireLimits>,
        enforce_wire_limits: bool,
    ) -> Self {
        Self {
            config,
            max_response_bytes,
            wire_limits,
            enforce_wire_limits,
            lookup_only_operations: HashSet::new(),
        }
    }
}

fn required(name: &str) -> Result<String, String> {
    std::env::var(name).map_err(|_| format!("{name} is required"))
}

fn gateway_address(value: &str) -> Result<SocketAddr, String> {
    let address: SocketAddr = value
        .parse()
        .map_err(|_| String::from("STS2_GATEWAY_ADDR must be a numeric loopback socket address"))?;
    if !address.ip().is_loopback() || address.port() == 0 {
        return Err(String::from(
            "STS2_GATEWAY_ADDR must be loopback with a nonzero port",
        ));
    }
    Ok(address)
}

fn safe_token(value: &str) -> bool {
    !value.is_empty() && value.len() <= 256 && value.bytes().all(|byte| byte.is_ascii_graphic())
}

fn required_or_default(name: &str, default: &str) -> Result<String, String> {
    configured_value(name, std::env::var(name), default)
}

fn configured_value(
    name: &str,
    supplied: Result<String, std::env::VarError>,
    default: &str,
) -> Result<String, String> {
    match supplied {
        Ok(value) if !value.is_empty() => Ok(value),
        Ok(_) => Err(format!("{name} must not be empty")),
        Err(std::env::VarError::NotPresent) => Ok(String::from(default)),
        Err(std::env::VarError::NotUnicode(_)) => Err(format!("{name} is not valid UTF-8")),
    }
}

fn safe_header_value(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && !value.contains("..")
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':' | b'/')
        })
}

#[cfg(test)]
#[path = "runtime_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "exact_restore_runtime_tests.rs"]
mod exact_restore_runtime_tests;
