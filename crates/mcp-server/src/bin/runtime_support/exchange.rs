// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;
use std::net::TcpStream;
use std::time::{Duration, Instant};

use sts2_mcp_server::{
    COOP_NATIVE_PROTOCOL_VERSION, COOP_RECEIPT_QUERY_PROTOCOL_VERSION,
    GAME_INFORMATION_PROTOCOL_VERSION, GatewayError, GatewayMethod, GatewayRequest,
    GatewayResponse, JsonValue, LIVE_BOOTSTRAP_PROTOCOL_VERSION,
    RUNTIME_V3_GAMEPLAY_PROTOCOL_VERSION, RUNTIME_V4_EXPERT_ACTION_PROTOCOL_VERSION,
    RUNTIME_V4_EXPERT_REST_ACTION_PROTOCOL_VERSION, SEEDED_RUN_PROTOCOL_VERSION, parse_json,
};

use super::RuntimeConfig;
use super::binding::is_runtime_result;
use super::http::{self, ReadError, read_response, write_request};

pub(super) fn exchange(
    config: &RuntimeConfig,
    request: GatewayRequest,
    body: Vec<u8>,
    max_response_bytes: usize,
) -> Result<GatewayResponse, GatewayError> {
    let method = match request.method {
        GatewayMethod::Get => "GET",
        GatewayMethod::Post => "POST",
    };
    let correlation = request
        .headers
        .get("x-sts2-correlation-id")
        .cloned()
        .unwrap_or_else(|| request.correlation.mcp_request_id.stable_text());
    exchange_wire(
        config,
        method,
        &request.path,
        request.headers,
        &body,
        &correlation,
        max_response_bytes,
    )
}

pub(super) fn exchange_startup(
    config: &RuntimeConfig,
    method: GatewayMethod,
    path: &str,
    body: &[u8],
    correlation: &str,
    max_response_bytes: usize,
) -> Result<GatewayResponse, GatewayError> {
    if config.exact_restore_profile || config.recovery_profile {
        return Err(GatewayError::Rejected);
    }
    let method = match method {
        GatewayMethod::Get => "GET",
        GatewayMethod::Post => "POST",
    };
    exchange_wire(
        config,
        method,
        path,
        BTreeMap::from([(
            String::from("x-mcp-session-id"),
            config.mcp_session_id.clone(),
        )]),
        body,
        correlation,
        max_response_bytes,
    )
}

pub(super) fn exchange_startup_capabilities(
    config: &RuntimeConfig,
    path: &str,
    correlation: &str,
    max_response_bytes: usize,
) -> Result<GatewayResponse, GatewayError> {
    exchange_wire(
        config,
        "GET",
        path,
        BTreeMap::from([
            (
                String::from("x-mcp-session-id"),
                config.mcp_session_id.clone(),
            ),
            (
                String::from("x-sts2-capabilities-version"),
                String::from("sts2-gateway-negotiated-capabilities-v2"),
            ),
        ]),
        &[],
        correlation,
        max_response_bytes,
    )
}

fn exchange_wire(
    config: &RuntimeConfig,
    method: &str,
    path: &str,
    supplied_headers: BTreeMap<String, String>,
    body: &[u8],
    correlation: &str,
    max_response_bytes: usize,
) -> Result<GatewayResponse, GatewayError> {
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut stream = TcpStream::connect_timeout(&config.gateway_address, Duration::from_secs(2))
        .map_err(|error| map_io(http::classify_io(error)))?;
    let headers = request_headers(config, supplied_headers, correlation, body.len(), path);
    write_request(&mut stream, method, path, &headers, body, deadline).map_err(
        |error| match error {
            ReadError::Malformed | ReadError::Oversized => GatewayError::Rejected,
            error => map_io(error),
        },
    )?;
    let response = read_response(&mut stream, deadline, max_response_bytes).map_err(map_io)?;
    let body = parse_json(
        std::str::from_utf8(&response.body).map_err(|_| GatewayError::MalformedResponse)?,
    )
    .map_err(|_| GatewayError::MalformedResponse)?;
    Ok(GatewayResponse {
        status: response.status,
        body,
    })
}

pub(super) fn map_io(error: ReadError) -> GatewayError {
    match error {
        ReadError::Timeout => GatewayError::Timeout,
        ReadError::Malformed => GatewayError::MalformedResponse,
        ReadError::Oversized => GatewayError::ResponseTooLarge,
        ReadError::Unavailable => GatewayError::Unavailable,
    }
}

fn request_headers(
    config: &RuntimeConfig,
    supplied: BTreeMap<String, String>,
    correlation: &str,
    body_length: usize,
    path: &str,
) -> BTreeMap<String, String> {
    let mut headers = supplied;
    let token = config
        .recovery_token
        .as_deref()
        .unwrap_or(config.gateway_token.as_str());
    headers.insert(String::from("Authorization"), format!("Bearer {token}"));
    if config.exact_restore_profile {
        headers.insert(
            String::from("x-sts2-recovery-capability"),
            String::from("exact_restore"),
        );
    }
    // The recovery sideband route demands the capability of the operation it
    // carries, so it is derived from the route rather than sent as a constant.
    if config.recovery_profile
        && let Some(operation) = sts2_mcp_server::RecoveryOperation::from_path(path)
    {
        headers.insert(
            String::from("x-sts2-recovery-capability"),
            String::from(operation.capability()),
        );
    }
    headers.insert(String::from("Host"), config.gateway_address.to_string());
    headers.insert(
        String::from("x-sts2-instance-id"),
        config.instance_id.clone(),
    );
    headers.insert(String::from("x-sts2-caller-id"), config.caller_id.clone());
    headers.insert(String::from("x-sts2-session-id"), config.session_id.clone());
    headers.insert(String::from("x-sts2-lease-id"), config.lease_id.clone());
    headers.insert(
        String::from("x-sts2-lease-epoch"),
        config.lease_epoch.to_string(),
    );
    headers.insert(
        String::from("x-sts2-correlation-id"),
        correlation.to_owned(),
    );
    headers.insert(String::from("Content-Length"), body_length.to_string());
    if body_length != 0 {
        headers.insert(
            String::from("Content-Type"),
            String::from("application/json"),
        );
    }
    headers
}

pub(super) fn classify(response: GatewayResponse) -> Result<GatewayResponse, GatewayError> {
    let GatewayResponse { status, body } = response;
    match status {
        408 | 502 | 503 | 504
            if is_runtime_result(&body)
                && matches!(&body, JsonValue::Object(object)
                    if object.get("protocol_version") == Some(&JsonValue::string(RUNTIME_V3_GAMEPLAY_PROTOCOL_VERSION))
                        || object.get("protocol_version") == Some(&JsonValue::string(RUNTIME_V4_EXPERT_ACTION_PROTOCOL_VERSION))
                        || object.get("protocol_version") == Some(&JsonValue::string(RUNTIME_V4_EXPERT_REST_ACTION_PROTOCOL_VERSION))
                        || object.get("protocol_version") == Some(&JsonValue::string(COOP_RECEIPT_QUERY_PROTOCOL_VERSION))
                        || object.get("protocol_version") == Some(&JsonValue::string(COOP_NATIVE_PROTOCOL_VERSION))
                        || object.get("protocol_version") == Some(&JsonValue::string(SEEDED_RUN_PROTOCOL_VERSION))
                        || object.get("protocol_version") == Some(&JsonValue::string(LIVE_BOOTSTRAP_PROTOCOL_VERSION))
                        || object.get("protocol_version") == Some(&JsonValue::string(GAME_INFORMATION_PROTOCOL_VERSION))) =>
        {
            // The semantic projection validates the full envelope before surfacing it.
            // A received host uncertainty receipt is not a transport disconnect.
            Ok(GatewayResponse { status, body })
        }
        401 => Err(GatewayError::Unauthorized),
        404 if is_runtime_result(&body)
            && matches!(&body, JsonValue::Object(object)
                    if object.get("protocol_version")
                        == Some(&JsonValue::string(RUNTIME_V4_EXPERT_REST_ACTION_PROTOCOL_VERSION))
                        || object.get("protocol_version")
                            == Some(&JsonValue::string(LIVE_BOOTSTRAP_PROTOCOL_VERSION))
                        || object.get("protocol_version")
                            == Some(&JsonValue::string(GAME_INFORMATION_PROTOCOL_VERSION))) =>
        {
            Ok(GatewayResponse { status, body })
        }
        403 => Err(GatewayError::Forbidden),
        404 => Err(GatewayError::NotFound),
        408 | 504 => Err(GatewayError::Timeout),
        502 | 503 => Err(GatewayError::Unavailable),
        400 | 409 | 413 | 422 if is_runtime_result(&body) => Ok(GatewayResponse { status, body }),
        400 | 409 | 413 | 422 => Err(GatewayError::Rejected),
        status => Ok(GatewayResponse { status, body }),
    }
}
