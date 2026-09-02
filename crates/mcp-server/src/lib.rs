// SPDX-License-Identifier: MIT

mod catalog;
mod gateway;
mod json;
mod protocol;
mod server;
mod transport;

pub use catalog::{CapabilityCatalog, ToolCatalog, ToolDescriptor};
pub use gateway::{
    Correlation, GatewayAdapter, GatewayError, GatewayMethod, GatewayRequest, GatewayResponse,
};
pub use json::JsonValue;
pub use protocol::{
    INVALID_PARAMS, INVALID_REQUEST, METHOD_NOT_FOUND, PARSE_ERROR, RequestId, RpcError,
    RpcResponse,
};
pub use server::{McpServer, SERVER_NAME, SERVER_VERSION};
pub use transport::{FrameCodec, FrameError, MAX_FRAME_BYTES};
