// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

use sts2_mcp_server::{
    GatewayAdapter, GatewayError, GatewayMethod, GatewayRequest, GatewayResponse, JsonValue,
    parse_json,
};

const MAX_BODY_BYTES: usize = 16 * 1024;
const MAX_RESPONSE_BYTES: usize = 64 * 1024;

pub(crate) struct RuntimeConfig {
    pub(crate) gateway_address: String,
    pub(crate) gateway_token: String,
    pub(crate) instance_id: String,
    pub(crate) caller_id: String,
    pub(crate) session_id: String,
    pub(crate) lease_id: String,
    pub(crate) lease_epoch: i64,
}

impl RuntimeConfig {
    pub(crate) fn from_environment() -> Result<Self, String> {
        let gateway_address = required_or_default("STS2_GATEWAY_ADDR", "127.0.0.1:15525")?;
        let gateway_token = required("STS2_GATEWAY_TOKEN")?;
        let instance_id = required_or_default("STS2_INSTANCE_ID", "instance-1")?;
        let caller_id = required_or_default("STS2_CALLER_ID", "harness")?;
        let session_id = required_or_default("STS2_SESSION_ID", "session-1")?;
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
            ("STS2_LEASE_ID", &lease_id),
        ] {
            if !safe_header_value(value) {
                return Err(format!("{name} is empty, unsafe, or oversized"));
            }
        }
        if gateway_token.is_empty()
            || gateway_token.len() > 256
            || gateway_token.bytes().any(|byte| byte.is_ascii_whitespace())
        {
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
        let encoded = JsonValue::Object(object).to_json();
        if encoded.len() > MAX_BODY_BYTES {
            return Err(GatewayError::Rejected);
        }
        Ok(encoded.into_bytes())
    }
}

impl GatewayAdapter for RuntimeGatewayAdapter {
    fn forward(&mut self, request: GatewayRequest) -> Result<GatewayResponse, GatewayError> {
        let body = self.body(&request)?;
        let address = self
            .config
            .gateway_address
            .to_socket_addrs()
            .map_err(|_| GatewayError::Unavailable)?
            .next()
            .ok_or(GatewayError::Unavailable)?;
        let mut stream = TcpStream::connect_timeout(&address, Duration::from_secs(2))
            .map_err(|_| GatewayError::Unavailable)?;
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .map_err(|_| GatewayError::Unavailable)?;
        stream
            .set_write_timeout(Some(Duration::from_secs(5)))
            .map_err(|_| GatewayError::Unavailable)?;
        let method = match request.method {
            GatewayMethod::Get => "GET",
            GatewayMethod::Post => "POST",
        };
        let mut headers = request.headers;
        headers.insert(
            String::from("Authorization"),
            format!("Bearer {}", self.config.gateway_token),
        );
        headers.insert(String::from("Host"), self.config.gateway_address.clone());
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
        write_request(&mut stream, method, &request.path, &headers, &body)
            .map_err(|_| GatewayError::Unavailable)?;
        let response = read_response(&mut stream).map_err(|error| match error {
            ReadError::Timeout => GatewayError::Timeout,
            ReadError::Malformed => GatewayError::MalformedResponse,
            ReadError::Oversized => GatewayError::Rejected,
            ReadError::Unavailable => GatewayError::Unavailable,
        })?;
        let body = parse_json(
            std::str::from_utf8(&response.body).map_err(|_| GatewayError::MalformedResponse)?,
        )
        .map_err(|_| GatewayError::MalformedResponse)?;
        match response.status {
            401 => Err(GatewayError::Unauthorized),
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

fn is_runtime_result(body: &JsonValue) -> bool {
    matches!(
        body,
        JsonValue::Object(object)
            if matches!(
                object.get("kind"),
                Some(JsonValue::String(kind))
                    if matches!(kind.as_str(), "state_response" | "action_response")
            )
    )
}

fn write_request(
    stream: &mut TcpStream,
    method: &str,
    path: &str,
    headers: &BTreeMap<String, String>,
    body: &[u8],
) -> std::io::Result<()> {
    let mut request = format!("{method} {path} HTTP/1.1\r\n");
    for (name, value) in headers {
        request.push_str(name);
        request.push_str(": ");
        request.push_str(value);
        request.push_str("\r\n");
    }
    request.push_str("Connection: close\r\n\r\n");
    stream.write_all(request.as_bytes())?;
    stream.write_all(body)
}

struct HttpResponse {
    status: u16,
    body: Vec<u8>,
}

enum ReadError {
    Timeout,
    Malformed,
    Oversized,
    Unavailable,
}

fn read_response(stream: &mut TcpStream) -> Result<HttpResponse, ReadError> {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 2048];
    let header_end = loop {
        if let Some(end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
            break end;
        }
        if bytes.len() >= 8 * 1024 {
            return Err(ReadError::Oversized);
        }
        let read = stream.read(&mut buffer).map_err(classify_io)?;
        if read == 0 {
            return Err(ReadError::Malformed);
        }
        bytes.extend_from_slice(&buffer[..read]);
    };
    let header = std::str::from_utf8(&bytes[..header_end]).map_err(|_| ReadError::Malformed)?;
    let mut lines = header.split("\r\n");
    let status_line = lines.next().ok_or(ReadError::Malformed)?;
    let mut parts = status_line.split_ascii_whitespace();
    if parts.next() != Some("HTTP/1.1") {
        return Err(ReadError::Malformed);
    }
    let status = parts
        .next()
        .ok_or(ReadError::Malformed)?
        .parse::<u16>()
        .map_err(|_| ReadError::Malformed)?;
    let mut content_length = None;
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            return Err(ReadError::Malformed);
        };
        if name.eq_ignore_ascii_case("content-length") {
            if content_length.is_some() {
                return Err(ReadError::Malformed);
            }
            content_length = Some(
                value
                    .trim()
                    .parse::<usize>()
                    .map_err(|_| ReadError::Malformed)?,
            );
        }
    }
    let content_length = content_length.ok_or(ReadError::Malformed)?;
    if content_length > MAX_RESPONSE_BYTES {
        return Err(ReadError::Oversized);
    }
    let body_start = header_end + 4;
    let available = bytes.len().saturating_sub(body_start);
    if available > content_length {
        return Err(ReadError::Malformed);
    }
    let mut body = bytes[body_start..].to_vec();
    while body.len() < content_length {
        let remaining = content_length - body.len();
        let read_capacity = remaining.min(buffer.len());
        let read = stream
            .read(&mut buffer[..read_capacity])
            .map_err(classify_io)?;
        if read == 0 {
            return Err(ReadError::Malformed);
        }
        body.extend_from_slice(&buffer[..read]);
    }
    Ok(HttpResponse { status, body })
}

fn classify_io(error: std::io::Error) -> ReadError {
    if matches!(
        error.kind(),
        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
    ) {
        ReadError::Timeout
    } else {
        ReadError::Unavailable
    }
}

fn required(name: &str) -> Result<String, String> {
    std::env::var(name).map_err(|_| format!("{name} is required"))
}

fn required_or_default(name: &str, default: &str) -> Result<String, String> {
    match std::env::var(name) {
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
