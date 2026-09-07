// SPDX-License-Identifier: MIT

mod catalog;
mod catalog_reobserve;
mod gateway;
mod json;
mod mapping;
mod projection;
mod protocol;
mod protocol_artifact;
mod protocol_artifact_recovery;
mod protocol_artifact_runtime_v2;
mod protocol_artifact_runtime_v3_gameplay;
mod recovery;
mod recovery_canonical;
mod recovery_fields;
mod recovery_payload;
mod recovery_projection;
mod recovery_validation;
mod server;
mod transport;

pub use catalog::COOP_SYNCHRONIZATION_TOOL;
pub use catalog::{
    BOOTSTRAP_TOOL, CapabilityCatalog, DISPATCH_ACTION_TOOL, GET_STATE_TOOL, HOST_FENCE_TOOL,
    LEASE_ACQUIRE_TOOL, LEASE_RENEW_TOOL, LEASE_REVOKE_TOOL, LEGAL_ACTIONS_TOOL, OBSERVE_TOOL,
    OPERATION_DISPATCH_TOOL, OPERATION_INTENT_TOOL, OPERATION_LOOKUP_TOOL,
    OPERATION_RECONCILE_TOOL, RECONCILE_ACTION_TOOL, RECOVER_TOOL, REOBSERVE_TOOL,
    SUBMIT_ACTION_TOOL, ToolCatalog, ToolDescriptor, WAIT_FOR_TRANSITION_TOOL,
};
pub use catalog_reobserve::catalog_reobserve_body;
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
pub use protocol_artifact_recovery::{
    RECOVERY_ARTIFACT, RECOVERY_CONTRACT, RECOVERY_MAX_ACTION_BYTES, RECOVERY_MAX_AUTH_PROOF_BYTES,
    RECOVERY_MAX_FRAME_BYTES, RECOVERY_MAX_RESPONSE_BYTES, RECOVERY_MAX_WIRE_INTEGER,
    RECOVERY_PROTOCOL_VERSION, RECOVERY_RUNTIME_V3_SCHEMA_DIGEST, RECOVERY_SCHEMA_DIGEST,
    RECOVERY_SCHEMA_SOURCE, RecoveryArtifactError, verify_recovery_artifact,
};
pub use protocol_artifact_runtime_v2::{
    RUNTIME_V2_ACTION_ID, RUNTIME_V2_ARTIFACT, RUNTIME_V2_EFFECT_KIND, RUNTIME_V2_GENERATOR,
    RUNTIME_V2_MAX_GENERATION, RUNTIME_V2_MAX_TURN_INDEX, RUNTIME_V2_PLAYER_TURN_PHASE,
    RUNTIME_V2_PROTOCOL_VERSION, RUNTIME_V2_SCHEMA_DIGEST, RUNTIME_V2_SCHEMA_SOURCE,
    RuntimeV2ArtifactError, verify_runtime_v2_artifact,
};
pub use protocol_artifact_runtime_v3_gameplay::{
    RUNTIME_V3_GAMEPLAY_ARTIFACT, RUNTIME_V3_GAMEPLAY_GENERATOR,
    RUNTIME_V3_GAMEPLAY_MAX_GENERATION, RUNTIME_V3_GAMEPLAY_MAX_WAIT_MILLIS,
    RUNTIME_V3_GAMEPLAY_PROTOCOL_VERSION, RUNTIME_V3_GAMEPLAY_SCHEMA_DIGEST,
    RUNTIME_V3_GAMEPLAY_SCHEMA_SOURCE,
};
pub use recovery::{
    recovery_capability, recovery_kind_for_path, validate_recovery_request,
    validate_recovery_response,
};
pub use server::{MCP_PROTOCOL_VERSION, McpServer, SERVER_NAME, SERVER_VERSION};
pub use transport::{FrameCodec, FrameError, MAX_FRAME_BYTES};
