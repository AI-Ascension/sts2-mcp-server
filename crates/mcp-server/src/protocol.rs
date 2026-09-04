// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;
use std::fmt;

use crate::json::JsonValue;

pub const PARSE_ERROR: i32 = -32700;
pub const INVALID_REQUEST: i32 = -32600;
pub const METHOD_NOT_FOUND: i32 = -32601;
pub const INVALID_PARAMS: i32 = -32602;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RequestId {
    Number(i64),
    String(String),
}

impl RequestId {
    pub(crate) fn from_json(value: &JsonValue) -> Option<Self> {
        match value {
            JsonValue::Number(value) => Some(Self::Number(*value)),
            JsonValue::String(value) => Some(Self::String(value.clone())),
            _ => None,
        }
    }

    pub(crate) fn to_json(&self) -> JsonValue {
        match self {
            Self::Number(value) => JsonValue::Number(*value),
            Self::String(value) => JsonValue::String(value.clone()),
        }
    }

    pub fn stable_text(&self) -> String {
        match self {
            Self::Number(value) => value.to_string(),
            Self::String(value) => value.clone(),
        }
    }
}

impl fmt::Display for RequestId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.stable_text())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RpcRequest {
    pub(crate) id: RequestId,
    pub(crate) method: String,
    pub(crate) params: JsonValue,
}

impl RpcRequest {
    pub(crate) fn from_json(value: JsonValue) -> Result<Self, &'static str> {
        let object = value.as_object().ok_or("request must be a JSON object")?;
        let version = object
            .get("jsonrpc")
            .and_then(JsonValue::as_string)
            .ok_or("request jsonrpc must be a string")?;
        if version != "2.0" {
            return Err("request jsonrpc version must be 2.0");
        }
        let id = object
            .get("id")
            .and_then(RequestId::from_json)
            .ok_or("request id must be a string or integer")?;
        let method = object
            .get("method")
            .and_then(JsonValue::as_string)
            .filter(|value| !value.is_empty())
            .ok_or("request method must be a non-empty string")?;
        let params = object
            .get("params")
            .cloned()
            .unwrap_or_else(|| JsonValue::Object(BTreeMap::new()));
        if !matches!(params, JsonValue::Object(_) | JsonValue::Array(_)) {
            return Err("request params must be structured");
        }
        Ok(Self {
            id,
            method: method.to_owned(),
            params,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RpcError {
    pub code: i32,
    pub message: String,
}

impl RpcError {
    pub(crate) fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RpcResponse {
    id: Option<RequestId>,
    result: Option<JsonValue>,
    pub error: Option<RpcError>,
}

impl RpcResponse {
    pub(crate) fn success(id: RequestId, result: JsonValue) -> Self {
        Self {
            id: Some(id),
            result: Some(result),
            error: None,
        }
    }

    pub(crate) fn failure(id: Option<RequestId>, error: RpcError) -> Self {
        Self {
            id,
            result: None,
            error: Some(error),
        }
    }

    pub(crate) fn to_json(&self) -> String {
        let mut response = BTreeMap::new();
        response.insert("jsonrpc".to_owned(), JsonValue::string("2.0"));
        response.insert(
            "id".to_owned(),
            self.id.as_ref().map_or(JsonValue::Null, RequestId::to_json),
        );
        match (&self.result, &self.error) {
            (Some(result), None) => {
                response.insert("result".to_owned(), result.clone());
            }
            (None, Some(error)) => {
                response.insert(
                    "error".to_owned(),
                    JsonValue::object([
                        ("code".to_owned(), JsonValue::Number(i64::from(error.code))),
                        (
                            "message".to_owned(),
                            JsonValue::string(error.message.as_str()),
                        ),
                    ]),
                );
            }
            _ => {
                response.insert(
                    "error".to_owned(),
                    JsonValue::object([
                        ("code".to_owned(), JsonValue::Number(-32603)),
                        (
                            "message".to_owned(),
                            JsonValue::string("invalid response state"),
                        ),
                    ]),
                );
            }
        }
        JsonValue::Object(response).to_json()
    }
}
