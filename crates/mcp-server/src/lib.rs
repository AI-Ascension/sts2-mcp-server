// SPDX-License-Identifier: MIT

mod catalog;
mod gateway;
mod json;
mod mapping;
mod protocol;
mod protocol_artifact;
mod server;
mod transport;

pub use catalog::{
    CapabilityCatalog, GET_STATE_TOOL, SUBMIT_ACTION_TOOL, ToolCatalog, ToolDescriptor,
};
pub use gateway::{
    Correlation, GatewayAdapter, GatewayError, GatewayMethod, GatewayRequest, GatewayResponse,
};
pub use json::JsonValue;
pub use protocol::{
    INVALID_PARAMS, INVALID_REQUEST, METHOD_NOT_FOUND, PARSE_ERROR, RequestId, RpcError,
    RpcResponse,
};
pub use protocol_artifact::{
    ArtifactError, POC_ARTIFACT, POC_GENERATOR, POC_MAX_SETTLED_EFFECTS, POC_MAX_UNITS,
    POC_PROTOCOL_VERSION, POC_SCHEMA_DIGEST, POC_SCHEMA_SOURCE, verify_poc_artifact,
};
pub use server::{McpServer, SERVER_NAME, SERVER_VERSION};
pub use transport::{FrameCodec, FrameError, MAX_FRAME_BYTES};
