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
    pub(crate) fn decode(frame: &str) -> Result<RpcRequest, FrameError> {
        if frame.len() > MAX_FRAME_BYTES {
            return Err(FrameError::TooLarge);
        }
        if frame.contains(['\n', '\r']) {
            return Err(FrameError::MultipleLines);
        }
        let value = json::parse(frame).map_err(|_| FrameError::InvalidJson)?;
        RpcRequest::from_json(value).map_err(|_| FrameError::InvalidRequest)
    }

    pub(crate) fn encode(response: &RpcResponse) -> String {
        response.to_json()
    }
}
