// SPDX-License-Identifier: MIT

use crate::json;
use crate::protocol::{RpcRequest, RpcResponse};

pub const MAX_FRAME_BYTES: usize = 16 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrameError {
    TooLarge,
    MultipleLines,
    InvalidJson,
    InvalidRequest,
}

pub struct FrameCodec;

impl FrameCodec {
    pub(crate) fn decode(frame: &str) -> Result<Option<RpcRequest>, FrameError> {
        if frame.len() > MAX_FRAME_BYTES {
            return Err(FrameError::TooLarge);
        }
        if frame.contains(['\n', '\r']) {
            return Err(FrameError::MultipleLines);
        }
        let mut value = json::parse(frame).map_err(|_| FrameError::InvalidJson)?;
        let notification = if let json::JsonValue::Object(object) = &mut value {
            if !object.contains_key("id") {
                object.insert(String::from("id"), json::JsonValue::Number(0));
                true
            } else {
                false
            }
        } else {
            false
        };
        let request = RpcRequest::from_json(value).map_err(|_| FrameError::InvalidRequest)?;
        Ok(if notification { None } else { Some(request) })
    }

    pub(crate) fn encode(response: &RpcResponse) -> String {
        response.to_json()
    }
}
