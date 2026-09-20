// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::json::JsonValue;
use crate::protocol::RequestId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GatewayMethod {
    Get,
    Post,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Correlation {
    pub mcp_session_id: String,
    pub mcp_request_id: RequestId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GatewayRequest {
    pub method: GatewayMethod,
    pub path: String,
    pub headers: BTreeMap<String, String>,
    pub body: Option<JsonValue>,
    pub correlation: Correlation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GatewayResponse {
    pub status: u16,
    pub body: JsonValue,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GatewayError {
    Unauthorized,
    Forbidden,
    NotFound,
    Unavailable,
    Timeout,
    MalformedResponse,
    ResponseTooLarge,
    Rejected,
}

pub trait GatewayAdapter {
    fn forward(&mut self, request: GatewayRequest) -> Result<GatewayResponse, GatewayError>;

    /// The configured gateway caller identity for profiles whose outgoing
    /// frames must bind `actor`/`auth` to the authenticated caller.
    ///
    /// The value is never taken from a tool argument, so a profile that needs
    /// it must report it here rather than trusting the caller-supplied
    /// envelope. Profiles without such an identity return `None` and their
    /// requests are unaffected.
    fn frame_principal(&self) -> Option<&str> {
        None
    }
}
