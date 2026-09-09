// SPDX-License-Identifier: MIT

use super::{CapabilityCatalog, MAX_IDENTIFIER_BYTES, ToolDescriptor};
use crate::json::JsonValue;

#[path = "catalog_coop_native_schema.rs"]
mod schema;

pub(super) const REVISION: &str = "coop-native-v1-mcp";
pub(super) const OBSERVATION_TOOL: &str = "sts2.coop_native_observation";
pub(super) const ACTION_TOOL: &str = "sts2.coop_native_action";
pub(super) const VOTE_TOOL: &str = "sts2.coop_native_vote";
pub(super) const REJOIN_TOOL: &str = "sts2.coop_native_rejoin";
pub(super) const EFFECT_TOOL: &str = "sts2.coop_native_effect";
pub(super) const RECOVER_TOOL: &str = "sts2.coop_native_recover";

pub(super) fn build() -> super::ToolCatalog {
    super::ToolCatalog {
        revision: String::from(REVISION),
        capabilities: CapabilityCatalog::default(),
        tools: vec![
            descriptor(
                OBSERVATION_TOOL,
                "Read the typed native co-op observation for one gateway-selected instance.",
                schema(
                    &["instance_id", "mcp_session_id", "lease_id", "lease_epoch"],
                    &[],
                ),
            ),
            descriptor(
                ACTION_TOOL,
                "Submit one host-admitted native co-op local action with its exact operation identity.",
                schema(
                    &[
                        "instance_id",
                        "mcp_session_id",
                        "lease_id",
                        "lease_epoch",
                        "operation_id",
                        "actor_peer",
                        "expected_host_generation",
                        "action",
                    ],
                    &[(String::from("action"), action_schema())],
                ),
            ),
            descriptor(
                VOTE_TOOL,
                "Submit one host-admitted native co-op shared vote without choosing a fallback policy.",
                schema(
                    &[
                        "instance_id",
                        "mcp_session_id",
                        "lease_id",
                        "lease_epoch",
                        "operation_id",
                        "actor_peer",
                        "expected_host_generation",
                        "vote",
                    ],
                    &[(String::from("vote"), vote_schema())],
                ),
            ),
            descriptor(
                REJOIN_TOOL,
                "Request first-party native peer rejoin using a bounded recovery epoch.",
                schema(
                    &[
                        "instance_id",
                        "mcp_session_id",
                        "lease_id",
                        "lease_epoch",
                        "operation_id",
                        "actor_peer",
                        "expected_host_generation",
                        "recovery",
                    ],
                    &[(String::from("recovery"), rejoin_schema())],
                ),
            ),
            descriptor(
                EFFECT_TOOL,
                "Validate and project a typed native effect response; this response-only seam never contacts a game process.",
                schema(
                    &[
                        "instance_id",
                        "mcp_session_id",
                        "lease_id",
                        "lease_epoch",
                        "envelope",
                    ],
                    &[(String::from("envelope"), schema::effect_envelope_schema())],
                ),
            ),
            descriptor(
                RECOVER_TOOL,
                "Reconcile one previously submitted native operation using the same operation identity.",
                schema(
                    &[
                        "instance_id",
                        "mcp_session_id",
                        "lease_id",
                        "lease_epoch",
                        "operation_id",
                        "recovery",
                    ],
                    &[(String::from("recovery"), schema::recovery_schema())],
                ),
            ),
        ],
    }
}

fn descriptor(name: &str, description: &str, input_schema: JsonValue) -> ToolDescriptor {
    ToolDescriptor {
        name: String::from(name),
        description: String::from(description),
        input_schema,
    }
}

fn schema(required: &[&str], extra: &[(String, JsonValue)]) -> JsonValue {
    let mut properties = common_properties();
    for (name, value) in extra {
        properties.push((name.clone(), value.clone()));
    }
    JsonValue::object([
        (String::from("type"), JsonValue::string("object")),
        (String::from("additionalProperties"), JsonValue::Bool(false)),
        (
            String::from("required"),
            JsonValue::Array(required.iter().map(|key| JsonValue::string(*key)).collect()),
        ),
        (String::from("properties"), JsonValue::object(properties)),
    ])
}

fn common_properties() -> Vec<(String, JsonValue)> {
    vec![
        (
            String::from("instance_id"),
            identity("^[A-Za-z0-9_-]{1,128}$"),
        ),
        (
            String::from("mcp_session_id"),
            identity("^[A-Za-z0-9_.:/-]{1,128}$"),
        ),
        (
            String::from("lease_id"),
            identity("^[A-Za-z0-9_.:/-]{1,128}$"),
        ),
        (String::from("lease_epoch"), generation()),
    ]
}

fn identity(pattern: &str) -> JsonValue {
    JsonValue::object([
        (String::from("type"), JsonValue::string("string")),
        (String::from("minLength"), JsonValue::Number(1)),
        (
            String::from("maxLength"),
            JsonValue::Number(MAX_IDENTIFIER_BYTES as i64),
        ),
        (String::from("pattern"), JsonValue::string(pattern)),
    ])
}

fn peer_identity() -> JsonValue {
    identity("^peer:[A-Za-z0-9_.:/-]{5,507}$")
}

fn generation() -> JsonValue {
    JsonValue::object([
        (String::from("type"), JsonValue::string("integer")),
        (String::from("minimum"), JsonValue::Number(0)),
        (
            String::from("maximum"),
            JsonValue::Number(9_007_199_254_740_991),
        ),
    ])
}

fn nullable(schema: JsonValue) -> JsonValue {
    JsonValue::object([(
        String::from("anyOf"),
        JsonValue::Array(vec![
            schema,
            JsonValue::object([(String::from("type"), JsonValue::string("null"))]),
        ]),
    )])
}

fn action_schema() -> JsonValue {
    JsonValue::object([
        (String::from("type"), JsonValue::string("object")),
        (String::from("additionalProperties"), JsonValue::Bool(false)),
        (
            String::from("required"),
            JsonValue::Array(
                ["kind", "action_id", "target_peer"]
                    .into_iter()
                    .map(JsonValue::string)
                    .collect(),
            ),
        ),
        (
            String::from("properties"),
            JsonValue::object([
                (
                    String::from("kind"),
                    JsonValue::object([(
                        String::from("enum"),
                        JsonValue::Array(
                            [
                                "play_card",
                                "end_turn",
                                "select_card",
                                "choose_reward",
                                "confirm_selection",
                            ]
                            .into_iter()
                            .map(JsonValue::string)
                            .collect(),
                        ),
                    )]),
                ),
                (
                    String::from("action_id"),
                    identity("^[A-Za-z0-9_.:/-]{1,128}$"),
                ),
                (String::from("target_peer"), nullable(peer_identity())),
            ]),
        ),
    ])
}

fn vote_schema() -> JsonValue {
    JsonValue::object([
        (String::from("type"), JsonValue::string("object")),
        (String::from("additionalProperties"), JsonValue::Bool(false)),
        (
            String::from("required"),
            JsonValue::Array(
                ["proposal_id", "voter_peer", "choice"]
                    .into_iter()
                    .map(JsonValue::string)
                    .collect(),
            ),
        ),
        (
            String::from("properties"),
            JsonValue::object([
                (
                    String::from("proposal_id"),
                    identity("^[A-Za-z0-9_.:/-]{1,128}$"),
                ),
                (String::from("voter_peer"), peer_identity()),
                (
                    String::from("choice"),
                    identity("^[A-Za-z0-9_.:/-]{1,128}$"),
                ),
            ]),
        ),
    ])
}

fn rejoin_schema() -> JsonValue {
    JsonValue::object([
        (String::from("type"), JsonValue::string("object")),
        (String::from("additionalProperties"), JsonValue::Bool(false)),
        (
            String::from("required"),
            JsonValue::Array(
                ["kind", "rejoin_epoch"]
                    .into_iter()
                    .map(JsonValue::string)
                    .collect(),
            ),
        ),
        (
            String::from("properties"),
            JsonValue::object([
                (
                    String::from("kind"),
                    JsonValue::object([(String::from("const"), JsonValue::string("rejoin"))]),
                ),
                (String::from("rejoin_epoch"), generation()),
            ]),
        ),
    ])
}
