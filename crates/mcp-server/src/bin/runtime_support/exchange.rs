// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;
use std::net::TcpStream;
use std::time::{Duration, Instant};

use sts2_mcp_server::{
    GatewayError, GatewayMethod, GatewayRequest, GatewayResponse, JsonValue,
    RUNTIME_V3_GAMEPLAY_PROTOCOL_VERSION, parse_json,
};

use super::binding::is_runtime_result;
use super::http::{self, ReadError, read_response, write_request};
use super::{RuntimeConfig, map_io};

pub(super) fn exchange(
    config: &RuntimeConfig,
    request: GatewayRequest,
    body: Vec<u8>,
    max_response_bytes: usize,
) -> Result<GatewayResponse, GatewayError> {
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut stream = TcpStream::connect_timeout(&config.gateway_address, Duration::from_secs(2))
        .map_err(|error| map_io(http::classify_io(error)))?;
    let method = match request.method {
        GatewayMethod::Get => "GET",
        GatewayMethod::Post => "POST",
    };
    let headers = request_headers(
        config,
        request.headers,
        &request.correlation.mcp_request_id.stable_text(),
        body.len(),
    );
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

fn request_headers(
    config: &RuntimeConfig,
    supplied: BTreeMap<String, String>,
    correlation: &str,
    body_length: usize,
) -> BTreeMap<String, String> {
    let mut headers = supplied;
    headers.insert(
        String::from("Authorization"),
        format!("Bearer {}", config.gateway_token),
    );
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
                    if object.get("protocol_version") == Some(&JsonValue::string(RUNTIME_V3_GAMEPLAY_PROTOCOL_VERSION))) =>
        {
            // The semantic projection validates the full envelope before surfacing it.
            // A received host uncertainty receipt is not a transport disconnect.
            Ok(GatewayResponse { status, body })
        }
        401 => Err(GatewayError::Unauthorized),
        403 => Err(GatewayError::Forbidden),
        404 => Err(GatewayError::NotFound),
        408 | 504 => Err(GatewayError::Timeout),
        502 | 503 => Err(GatewayError::Unavailable),
        400 | 409 | 413 | 422 if is_runtime_result(&body) => Ok(GatewayResponse { status, body }),
        400 | 409 | 413 | 422 => Err(GatewayError::Rejected),
        status => Ok(GatewayResponse { status, body }),
    }
}
