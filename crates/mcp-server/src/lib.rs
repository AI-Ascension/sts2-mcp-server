// SPDX-License-Identifier: MIT

mod catalog;
mod gateway;
mod json;
mod mapping;
mod projection;
mod protocol;
mod protocol_artifact;
mod protocol_artifact_runtime_v2;
mod server;
mod transport;

pub use catalog::{
    CapabilityCatalog, GET_STATE_TOOL, RECONCILE_ACTION_TOOL, SUBMIT_ACTION_TOOL, ToolCatalog,
    ToolDescriptor,
};
pub use gateway::{
    Correlation, GatewayAdapter, GatewayError, GatewayMethod, GatewayRequest, GatewayResponse,
};
pub use json::{JsonValue, parse_json};
pub use protocol::{
    INVALID_PARAMS, INVALID_REQUEST, METHOD_NOT_FOUND, PARSE_ERROR, RequestId, RpcError,
    RpcResponse,
};
pub use protocol_artifact::{
    ArtifactError, POC_ARTIFACT, POC_GENERATOR, POC_MAX_GENERATION, POC_MAX_SETTLED_EFFECTS,
    POC_MAX_UNITS, POC_PROTOCOL_VERSION, POC_SCHEMA_DIGEST, POC_SCHEMA_SOURCE, RUNTIME_ACTION_ID,
    RUNTIME_ARTIFACT, RUNTIME_GENERATOR, RUNTIME_MAX_GENERATION, RUNTIME_PROTOCOL_VERSION,
    RUNTIME_SCHEMA_DIGEST, RUNTIME_SCHEMA_SOURCE, verify_poc_artifact,
};
pub use protocol_artifact_runtime_v2::{
    RUNTIME_V2_ACTION_ID, RUNTIME_V2_ARTIFACT, RUNTIME_V2_EFFECT_KIND, RUNTIME_V2_GENERATOR,
    RUNTIME_V2_MAX_GENERATION, RUNTIME_V2_MAX_TURN_INDEX, RUNTIME_V2_PLAYER_TURN_PHASE,
    RUNTIME_V2_PROTOCOL_VERSION, RUNTIME_V2_SCHEMA_DIGEST, RUNTIME_V2_SCHEMA_SOURCE,
    RuntimeV2ArtifactError, verify_runtime_v2_artifact,
};
pub use server::{MCP_PROTOCOL_VERSION, McpServer, SERVER_NAME, SERVER_VERSION};
pub use transport::{FrameCodec, FrameError, MAX_FRAME_BYTES};
