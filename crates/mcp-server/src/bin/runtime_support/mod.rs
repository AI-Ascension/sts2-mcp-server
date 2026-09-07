// SPDX-License-Identifier: MIT

use std::net::SocketAddr;

mod binding;
use binding::is_runtime_result;
mod config;
#[cfg(test)]
use config::configured_value;
use config::{
    gateway_address, optional_recovery_proof, recovery_or_gateway_token, recovery_profile_selected,
    required_or_default, safe_header_value, safe_recovery_principal, safe_recovery_uuid,
    safe_token, value_is_recovery,
};
mod exchange;
mod http;
mod profiles;
use http::ReadError;
pub(crate) use profiles::profile_from_environment;

use sts2_mcp_server::{
    GatewayAdapter, GatewayError, GatewayRequest, GatewayResponse, JsonValue, RECOVERY_CONTRACT,
    RECOVERY_MAX_FRAME_BYTES, RUNTIME_V2_PROTOCOL_VERSION, RUNTIME_V3_GAMEPLAY_PROTOCOL_VERSION,
};

const MAX_BODY_BYTES: usize = 16 * 1024;
const DEFAULT_MCP_SESSION_ID: &str = "mcp-session-1";

pub(crate) struct RuntimeConfig {
    pub(crate) gateway_address: SocketAddr,
    pub(crate) gateway_token: String,
    pub(crate) instance_id: String,
    pub(crate) caller_id: String,
    pub(crate) session_id: String,
    pub(crate) mcp_session_id: String,
    pub(crate) lease_id: String,
    pub(crate) lease_epoch: i64,
    pub(crate) recovery_principal_id: String,
    pub(crate) recovery_role: String,
    pub(crate) recovery_proof: Option<String>,
}

impl RuntimeConfig {
    pub(crate) fn from_environment() -> Result<Self, String> {
        let gateway_address = gateway_address(&required_or_default(
            "STS2_GATEWAY_ADDR",
            "127.0.0.1:15525",
        )?)?;
        let gateway_token = recovery_or_gateway_token()?;
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
        if !safe_token(&gateway_token) {
            let token_name = if recovery_profile_selected() {
                "STS2_RECOVERY_TOKEN"
            } else {
                "STS2_GATEWAY_TOKEN"
            };
            return Err(format!("{token_name} is empty, unsafe, or oversized"));
        }
        if recovery_profile_selected() && !safe_recovery_uuid(&instance_id) {
            return Err(String::from(
                "STS2_INSTANCE_ID must be a UUID for the recovery profile",
            ));
        }
        let recovery_principal_id = required_or_default(
            "STS2_RECOVERY_PRINCIPAL_ID",
            "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
        )?;
        let recovery_role = required_or_default("STS2_RECOVERY_ROLE", "harness")?;
        if !safe_recovery_principal(&recovery_principal_id) {
            return Err(String::from("STS2_RECOVERY_PRINCIPAL_ID is not a UUID"));
        }
        if !matches!(
            recovery_role.as_str(),
            "gateway" | "watchdog" | "harness" | "host" | "mod" | "operator"
        ) {
            return Err(String::from(
                "STS2_RECOVERY_ROLE is not an approved actor role",
            ));
        }
        let recovery_proof = optional_recovery_proof()?;
        Ok(Self {
            gateway_address,
            gateway_token,
            instance_id,
            caller_id,
            session_id,
            mcp_session_id,
            lease_id,
            lease_epoch,
            recovery_principal_id,
            recovery_role,
            recovery_proof,
        })
    }
}

pub(crate) struct RuntimeGatewayAdapter {
    config: RuntimeConfig,
    max_response_bytes: usize,
}

impl RuntimeGatewayAdapter {
    /// `max_response_bytes` is the selected profile's gateway body limit
    /// (`RuntimeProfile::max_response_bytes`); legacy profiles keep 64 KiB.
    pub(crate) const fn new(config: RuntimeConfig, max_response_bytes: usize) -> Self {
        Self {
            config,
            max_response_bytes,
        }
    }

    fn body(&self, request: &GatewayRequest) -> Result<Vec<u8>, GatewayError> {
        let Some(value) = &request.body else {
            return Ok(Vec::new());
        };
        let JsonValue::Object(mut object) = value.clone() else {
            return Err(GatewayError::Rejected);
        };
        let is_recovery = object.get("contract") == Some(&JsonValue::string(RECOVERY_CONTRACT));
        let is_runtime_v2 = matches!(
            object.get("protocol_version"),
            Some(JsonValue::String(value)) if value == RUNTIME_V2_PROTOCOL_VERSION
        );
        let is_runtime_v3 = matches!(
            object.get("protocol_version"),
            Some(JsonValue::String(value)) if value == RUNTIME_V3_GAMEPLAY_PROTOCOL_VERSION
        );
        if is_recovery {
            // Recovery frames are closed protocol objects.  Their authority and
            // identity fields are validated by `binding::admit`; injecting the
            // gameplay identity fields here would change the signed contract.
        } else if is_runtime_v2 || is_runtime_v3 {
            if object.get("instance_id")
                != Some(&JsonValue::string(self.config.instance_id.as_str()))
                || object.get("session_id")
                    != Some(&JsonValue::string(self.config.session_id.as_str()))
                || object.get("lease_id") != Some(&JsonValue::string(self.config.lease_id.as_str()))
                || object.get("lease_epoch") != Some(&JsonValue::Number(self.config.lease_epoch))
            {
                return Err(GatewayError::Rejected);
            }
        } else {
            object.insert(
                String::from("instance_id"),
                JsonValue::string(self.config.instance_id.as_str()),
            );
            object.insert(
                String::from("session_id"),
                JsonValue::string(self.config.session_id.as_str()),
            );
            object.insert(
                String::from("lease_id"),
                JsonValue::string(self.config.lease_id.as_str()),
            );
            object.insert(
                String::from("lease_epoch"),
                JsonValue::Number(self.config.lease_epoch),
            );
        }
        let encoded = JsonValue::Object(object).to_json();
        let limit = if value_is_recovery(request) {
            RECOVERY_MAX_FRAME_BYTES
        } else {
            MAX_BODY_BYTES
        };
        if encoded.len() > limit {
            return Err(GatewayError::Rejected);
        }
        Ok(encoded.into_bytes())
    }
}

impl GatewayAdapter for RuntimeGatewayAdapter {
    fn forward(&mut self, request: GatewayRequest) -> Result<GatewayResponse, GatewayError> {
        binding::admit(&self.config, &request)?;
        let recovery_kind = sts2_mcp_server::recovery_kind_for_path(&request.path);
        let response_kind = binding::response_kind(&self.config, &request);
        let correlation = request.correlation.mcp_request_id.stable_text();
        let catalog_read = request.method == sts2_mcp_server::GatewayMethod::Get
            && request.path == format!("/v3/instances/{}/legal-actions", self.config.instance_id);
        let body = self.body(&request)?;
        let response = exchange::exchange(&self.config, request, body, self.max_response_bytes)?;
        if let Some(kind) = recovery_kind {
            // A successful response must be a validated recovery frame.  For
            // an HTTP error, preserve a valid recovery error frame when the
            // gateway supplied one, while still mapping a bounded transport
            // error body to its typed status below (rather than converting an
            // ordinary 401/403/503 response into an unknown mutation).
            if (200..300).contains(&response.status) || binding::is_recovery_result(&response.body)
            {
                binding::recovery_response(&response.body, kind, &correlation)?;
            }
            return exchange::classify(response);
        }
        if catalog_read
            && sts2_mcp_server::catalog_reobserve_body(&response, &correlation).is_some()
        {
            return Ok(response);
        }
        if ((200..300).contains(&response.status) || is_runtime_result(&response.body))
            && let Some(kind) = response_kind
        {
            binding::response(&self.config, &response.body, &correlation, kind)?;
        }
        exchange::classify(response)
    }
}

fn map_io(error: ReadError) -> GatewayError {
    match error {
        ReadError::Timeout => GatewayError::Timeout,
        ReadError::Malformed => GatewayError::MalformedResponse,
        ReadError::Oversized => GatewayError::MalformedResponse,
        ReadError::Unavailable => GatewayError::Unavailable,
    }
}

#[cfg(test)]
#[path = "runtime_tests.rs"]
mod tests;
