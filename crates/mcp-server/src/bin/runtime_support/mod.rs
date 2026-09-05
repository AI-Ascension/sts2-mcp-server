// SPDX-License-Identifier: MIT

use std::net::{SocketAddr, TcpStream};
use std::time::{Duration, Instant};

mod binding;
use binding::is_runtime_result;
mod http;
mod profiles;
use http::{ReadError, read_response, write_request};
pub(crate) use profiles::catalog_from_environment;

use sts2_mcp_server::{
    GatewayAdapter, GatewayError, GatewayMethod, GatewayRequest, GatewayResponse, JsonValue,
    RUNTIME_V2_PROTOCOL_VERSION, RUNTIME_V3_GAMEPLAY_PROTOCOL_VERSION, parse_json,
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
}

impl RuntimeConfig {
    pub(crate) fn from_environment() -> Result<Self, String> {
        let gateway_address = gateway_address(&required_or_default(
            "STS2_GATEWAY_ADDR",
            "127.0.0.1:15525",
        )?)?;
        let gateway_token = required("STS2_GATEWAY_TOKEN")?;
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
            return Err(String::from(
                "STS2_GATEWAY_TOKEN is empty, unsafe, or oversized",
            ));
        }
        Ok(Self {
            gateway_address,
            gateway_token,
            instance_id,
            caller_id,
            session_id,
            mcp_session_id,
            lease_id,
            lease_epoch,
        })
    }
}

pub(crate) struct RuntimeGatewayAdapter {
    config: RuntimeConfig,
}

impl RuntimeGatewayAdapter {
    pub(crate) const fn new(config: RuntimeConfig) -> Self {
        Self { config }
    }

    fn body(&self, request: &GatewayRequest) -> Result<Vec<u8>, GatewayError> {
        let Some(value) = &request.body else {
            return Ok(Vec::new());
        };
        let JsonValue::Object(mut object) = value.clone() else {
            return Err(GatewayError::Rejected);
        };
        let is_runtime_v2 = matches!(
            object.get("protocol_version"),
            Some(JsonValue::String(value)) if value == RUNTIME_V2_PROTOCOL_VERSION
        );
        let is_runtime_v3 = matches!(
            object.get("protocol_version"),
            Some(JsonValue::String(value)) if value == RUNTIME_V3_GAMEPLAY_PROTOCOL_VERSION
        );
        if is_runtime_v2 || is_runtime_v3 {
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
        if encoded.len() > MAX_BODY_BYTES {
            return Err(GatewayError::Rejected);
        }
        Ok(encoded.into_bytes())
    }
}

impl GatewayAdapter for RuntimeGatewayAdapter {
    fn forward(&mut self, request: GatewayRequest) -> Result<GatewayResponse, GatewayError> {
        binding::admit(&self.config, &request)?;
        let response_kind = binding::response_kind(&self.config, &request);
        let correlation = request.correlation.mcp_request_id.stable_text();
        let body = self.body(&request)?;
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut stream =
            TcpStream::connect_timeout(&self.config.gateway_address, Duration::from_secs(2))
                .map_err(|error| map_io(http::classify_io(error)))?;
        let method = match request.method {
            GatewayMethod::Get => "GET",
            GatewayMethod::Post => "POST",
        };
        let mut headers = request.headers;
        headers.insert(
            String::from("Authorization"),
            format!("Bearer {}", self.config.gateway_token),
        );
        headers.insert(
            String::from("Host"),
            self.config.gateway_address.to_string(),
        );
        headers.insert(
            String::from("x-sts2-instance-id"),
            self.config.instance_id.clone(),
        );
        headers.insert(
            String::from("x-sts2-caller-id"),
            self.config.caller_id.clone(),
        );
        headers.insert(
            String::from("x-sts2-session-id"),
            self.config.session_id.clone(),
        );
        headers.insert(
            String::from("x-sts2-lease-id"),
            self.config.lease_id.clone(),
        );
        headers.insert(
            String::from("x-sts2-lease-epoch"),
            self.config.lease_epoch.to_string(),
        );
        headers.insert(
            String::from("x-sts2-correlation-id"),
            request.correlation.mcp_request_id.stable_text(),
        );
        headers.insert(String::from("Content-Length"), body.len().to_string());
        if !body.is_empty() {
            headers.insert(
                String::from("Content-Type"),
                String::from("application/json"),
            );
        }
        write_request(
            &mut stream,
            method,
            &request.path,
            &headers,
            &body,
            deadline,
        )
        .map_err(|error| match error {
            ReadError::Malformed | ReadError::Oversized => GatewayError::Rejected,
            error => map_io(error),
        })?;
        let response = read_response(&mut stream, deadline).map_err(map_io)?;
        let body = parse_json(
            std::str::from_utf8(&response.body).map_err(|_| GatewayError::MalformedResponse)?,
        )
        .map_err(|_| GatewayError::MalformedResponse)?;
        if ((200..300).contains(&response.status) || is_runtime_result(&body))
            && let Some(kind) = response_kind
        {
            binding::response(&self.config, &body, &correlation, kind)?;
        }
        match response.status {
            408 | 502 | 503 | 504
                if is_runtime_result(&body)
                    && matches!(&body, JsonValue::Object(object)
                    if object.get("protocol_version") == Some(&JsonValue::string(RUNTIME_V3_GAMEPLAY_PROTOCOL_VERSION))) =>
            {
                // The semantic projection validates the full envelope before surfacing it.
                // A received host uncertainty receipt is not a transport disconnect.
                Ok(GatewayResponse {
                    status: response.status,
                    body,
                })
            }
            401 => Err(GatewayError::Unauthorized),
            403 => Err(GatewayError::Forbidden),
            404 => Err(GatewayError::NotFound),
            408 | 504 => Err(GatewayError::Timeout),
            502 | 503 => Err(GatewayError::Unavailable),
            400 | 409 | 413 | 422 if is_runtime_result(&body) => Ok(GatewayResponse {
                status: response.status,
                body,
            }),
            400 | 409 | 413 | 422 => Err(GatewayError::Rejected),
            status => Ok(GatewayResponse { status, body }),
        }
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
