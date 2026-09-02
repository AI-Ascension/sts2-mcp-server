// SPDX-License-Identifier: MIT

use std::collections::VecDeque;

use sts2_mcp_server::{
    GatewayAdapter, GatewayError, GatewayRequest, GatewayResponse, JsonValue, RUNTIME_V2_ACTION_ID,
    RUNTIME_V2_ARTIFACT, RUNTIME_V2_EFFECT_KIND, RUNTIME_V2_GENERATOR, RUNTIME_V2_PROTOCOL_VERSION,
    RUNTIME_V2_SCHEMA_DIGEST, RUNTIME_V2_SCHEMA_SOURCE,
};

pub struct RecordingGateway {
    pub requests: Vec<GatewayRequest>,
    responses: VecDeque<Result<GatewayResponse, GatewayError>>,
}

impl RecordingGateway {
    pub fn new(responses: impl IntoIterator<Item = Result<GatewayResponse, GatewayError>>) -> Self {
        Self {
            requests: Vec::new(),
            responses: responses.into_iter().collect(),
        }
    }
}

impl GatewayAdapter for RecordingGateway {
    fn forward(&mut self, request: GatewayRequest) -> Result<GatewayResponse, GatewayError> {
        self.requests.push(request);
        self.responses
            .pop_front()
            .unwrap_or(Err(GatewayError::Unavailable))
    }
}

pub fn submit_call(
    id: &str,
    instance: &str,
    session: &str,
    lease: &str,
    lease_epoch: i64,
    generation: i64,
    operation: &str,
) -> String {
    format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":\"{id}\",\"method\":\"tools/call\",\
         \"params\":{{\"name\":\"submit_action\",\"arguments\":{{\
         \"instance_id\":\"{instance}\",\"mcp_session_id\":\"{session}\",\
         \"lease_id\":\"{lease}\",\"lease_epoch\":{lease_epoch},\
         \"generation\":{generation},\"operation_id\":\"{operation}\",\
         \"action_id\":\"{RUNTIME_V2_ACTION_ID}\"}}}}}}"
    )
}

#[allow(dead_code)]
pub fn reconcile_call(
    id: &str,
    instance: &str,
    session: &str,
    lease: &str,
    lease_epoch: i64,
    generation: i64,
    operation: &str,
) -> String {
    format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":\"{id}\",\"method\":\"tools/call\",\
         \"params\":{{\"name\":\"reconcile_action\",\"arguments\":{{\
         \"instance_id\":\"{instance}\",\"mcp_session_id\":\"{session}\",\
         \"lease_id\":\"{lease}\",\"lease_epoch\":{lease_epoch},\
         \"generation\":{generation},\"operation_id\":\"{operation}\"}}}}}}"
    )
}

pub fn observation(generation: i64, phase: &str, turn_index: i64) -> JsonValue {
    JsonValue::object([
        (String::from("combat_phase"), JsonValue::string(phase)),
        (String::from("turn_index"), JsonValue::Number(turn_index)),
        (String::from("host_ready"), JsonValue::Bool(true)),
        (String::from("generation"), JsonValue::Number(generation)),
    ])
}

#[allow(clippy::too_many_arguments)]
pub fn result(
    correlation: &str,
    instance: &str,
    session: &str,
    lease: &str,
    lease_epoch: i64,
    generation: i64,
    operation: &str,
    kind: &str,
    status: &str,
    observation_value: Option<JsonValue>,
    error_code: Option<&str>,
    witness_generation: Option<i64>,
) -> JsonValue {
    JsonValue::object([
        (
            String::from("protocol_version"),
            JsonValue::string(RUNTIME_V2_PROTOCOL_VERSION),
        ),
        (
            String::from("schema_digest"),
            JsonValue::string(RUNTIME_V2_SCHEMA_DIGEST),
        ),
        (
            String::from("provenance"),
            JsonValue::object([
                (
                    String::from("artifact"),
                    JsonValue::string(RUNTIME_V2_ARTIFACT),
                ),
                (
                    String::from("source"),
                    JsonValue::string(RUNTIME_V2_SCHEMA_SOURCE),
                ),
                (
                    String::from("generator"),
                    JsonValue::string(RUNTIME_V2_GENERATOR),
                ),
            ]),
        ),
        (
            String::from("correlation_id"),
            JsonValue::string(correlation),
        ),
        (String::from("instance_id"), JsonValue::string(instance)),
        (String::from("session_id"), JsonValue::string(session)),
        (String::from("lease_id"), JsonValue::string(lease)),
        (String::from("lease_epoch"), JsonValue::Number(lease_epoch)),
        (String::from("generation"), JsonValue::Number(generation)),
        (String::from("kind"), JsonValue::string(kind)),
        (String::from("operation_id"), JsonValue::string(operation)),
        (
            String::from("observation"),
            observation_value.unwrap_or(JsonValue::Null),
        ),
        (
            String::from("action"),
            JsonValue::object([(
                String::from("action_id"),
                JsonValue::string(RUNTIME_V2_ACTION_ID),
            )]),
        ),
        (String::from("status"), JsonValue::string(status)),
        (
            String::from("error_code"),
            error_code.map_or(JsonValue::Null, JsonValue::string),
        ),
        (
            String::from("effect_witness"),
            witness_generation.map_or(JsonValue::Null, |generation| {
                JsonValue::object([
                    (
                        String::from("kind"),
                        JsonValue::string(RUNTIME_V2_EFFECT_KIND),
                    ),
                    (String::from("generation"), JsonValue::Number(generation)),
                ])
            }),
        ),
    ])
}

pub fn accepted(correlation: &str, generation: i64) -> JsonValue {
    result(
        correlation,
        "instance-1",
        "session-1",
        "lease-1",
        1,
        generation,
        "op-1",
        "action_response",
        "accepted",
        Some(observation(generation, "combat/player_turn", 2)),
        None,
        None,
    )
}

pub fn settled(correlation: &str, generation: i64, operation: &str, kind: &str) -> JsonValue {
    result(
        correlation,
        "instance-1",
        "session-1",
        "lease-1",
        1,
        generation,
        operation,
        kind,
        "settled",
        Some(observation(generation, "combat/player_turn", 3)),
        None,
        Some(generation),
    )
}

pub fn rejected(correlation: &str, error_code: &str, operation: &str) -> JsonValue {
    result(
        correlation,
        "instance-1",
        "session-1",
        "lease-1",
        1,
        4,
        operation,
        "action_response",
        "rejected",
        Some(observation(4, "combat/player_turn", 2)),
        Some(error_code),
        None,
    )
}

pub fn contains_result_field(response: &str, key: &str, value: &str) -> bool {
    response.contains(&format!(r#"\"{key}\":\"{value}\""#))
}
