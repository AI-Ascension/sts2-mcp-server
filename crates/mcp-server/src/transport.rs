// SPDX-License-Identifier: MIT

use crate::json;
use crate::protocol::{RpcRequest, RpcResponse};

/// Absolute MCP frame ceiling; only the Runtime-v3 semantic profile accepts frames this large.
pub const MAX_FRAME_BYTES: usize = 256 * 1024;
/// Historical frame limit kept by the poc, runtime-v1, and runtime-v2 profiles.
pub(crate) const LEGACY_MAX_FRAME_BYTES: usize = 16 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrameError {
    TooLarge,
    MultipleLines,
    InvalidJson,
    InvalidRequest,
}

pub struct FrameCodec;

impl FrameCodec {
    pub(crate) fn decode(
        frame: &str,
        max_frame_bytes: usize,
    ) -> Result<Option<RpcRequest>, FrameError> {
        if frame.len() > max_frame_bytes.min(MAX_FRAME_BYTES) {
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
