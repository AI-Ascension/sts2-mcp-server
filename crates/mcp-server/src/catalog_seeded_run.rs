// SPDX-License-Identifier: MIT

use super::{CapabilityCatalog, MAX_IDENTIFIER_BYTES, ToolDescriptor};
use crate::json::JsonValue;
use crate::protocol_artifact_seeded_run::SEEDED_RUN_MAX_GENERATION;

pub(super) const REVISION: &str = "seeded-run-v1-mcp";
pub(super) const START_SEEDED_RUN_TOOL: &str = "start_seeded_run";
pub(super) const RECONCILE_SEEDED_RUN_TOOL: &str = "reconcile_seeded_run";
const IDENTITY_PATTERN: &str = "^[A-Za-z0-9_.:/-]{1,128}$";
const PATH_ID_PATTERN: &str = "^[A-Za-z0-9_-]{1,128}$";
const OPERATION_ID_PATTERN: &str = "^(?!.*\\.\\.)[A-Za-z0-9_.:-]{1,128}$";
const SEED_PATTERN: &str = "^[^\\u0000-\\u001f\\u007f]*$";
const DIGEST_PATTERN: &str = "^[0-9a-f]{64}$";

pub(super) fn build() -> super::ToolCatalog {
    super::ToolCatalog {
        revision: String::from(REVISION),
        capabilities: CapabilityCatalog::default(),
        tools: vec![
            ToolDescriptor {
                name: String::from(START_SEEDED_RUN_TOOL),
                description: String::from(
                    "Start one bounded explicit-seed run through the authenticated leased runtime; settlement requires a fresh native observation and run_started witness.",
                ),
                input_schema: start_schema(),
            },
            ToolDescriptor {
                name: String::from(RECONCILE_SEEDED_RUN_TOOL),
                description: String::from(
                    "Reconcile one uncertain seeded-run operation by path-safe operation_id without replaying the seed mutation.",
                ),
                input_schema: reconcile_schema(),
            },
        ],
    }
}

fn start_schema() -> JsonValue {
    operation_schema(
        [
            "instance_id",
            "mcp_session_id",
            "lease_id",
            "lease_epoch",
            "generation",
            "operation_id",
            "seed",
            "run_mode",
            "selected_context",
        ],
        [
            (String::from("instance_id"), bounded(PATH_ID_PATTERN)),
            (String::from("mcp_session_id"), bounded(IDENTITY_PATTERN)),
            (String::from("lease_id"), bounded(IDENTITY_PATTERN)),
            (
                String::from("lease_epoch"),
                counter(SEEDED_RUN_MAX_GENERATION),
            ),
            (
                String::from("generation"),
                counter(SEEDED_RUN_MAX_GENERATION),
            ),
            (String::from("operation_id"), bounded(OPERATION_ID_PATTERN)),
            (String::from("seed"), seed()),
            (String::from("run_mode"), mode()),
            (String::from("selected_context"), selected_context()),
        ],
    )
}

fn reconcile_schema() -> JsonValue {
    operation_schema(
        [
            "instance_id",
            "mcp_session_id",
            "lease_id",
            "lease_epoch",
            "generation",
            "operation_id",
        ],
        [
            (String::from("instance_id"), bounded(PATH_ID_PATTERN)),
            (String::from("mcp_session_id"), bounded(IDENTITY_PATTERN)),
            (String::from("lease_id"), bounded(IDENTITY_PATTERN)),
            (
                String::from("lease_epoch"),
                counter(SEEDED_RUN_MAX_GENERATION),
            ),
            (
                String::from("generation"),
                counter(SEEDED_RUN_MAX_GENERATION),
            ),
            (String::from("operation_id"), bounded(OPERATION_ID_PATTERN)),
        ],
    )
}

fn operation_schema(
    required: impl IntoIterator<Item = &'static str>,
    properties: impl IntoIterator<Item = (String, JsonValue)>,
) -> JsonValue {
    JsonValue::object([
        (String::from("type"), JsonValue::string("object")),
        (String::from("additionalProperties"), JsonValue::Bool(false)),
        (
            String::from("required"),
            JsonValue::Array(required.into_iter().map(JsonValue::string).collect()),
        ),
        (String::from("properties"), JsonValue::object(properties)),
    ])
}

fn selected_context() -> JsonValue {
    operation_schema(
        [
            "context_id",
            "game_mode",
            "character",
            "ascension",
            "modifiers",
            "acts",
            "selection_policy",
            "profile_baseline",
            "save_policy",
            "compatibility",
            "context_digest",
        ],
        [
            (String::from("context_id"), bounded(IDENTITY_PATTERN)),
            (
                String::from("game_mode"),
                JsonValue::object([(String::from("const"), JsonValue::string("standard"))]),
            ),
            (
                String::from("character"),
                JsonValue::object([(String::from("const"), JsonValue::string("ironclad"))]),
            ),
            (
                String::from("ascension"),
                JsonValue::object([
                    (String::from("type"), JsonValue::string("integer")),
                    (String::from("minimum"), JsonValue::Number(0)),
                    (String::from("maximum"), JsonValue::Number(20)),
                ]),
            ),
            (
                String::from("modifiers"),
                JsonValue::object([
                    (String::from("type"), JsonValue::string("array")),
                    (String::from("maxItems"), JsonValue::Number(32)),
                    (String::from("uniqueItems"), JsonValue::Bool(true)),
                    (String::from("items"), bounded(IDENTITY_PATTERN)),
                ]),
            ),
            (
                String::from("acts"),
                JsonValue::object([
                    (String::from("type"), JsonValue::string("array")),
                    (String::from("minItems"), JsonValue::Number(1)),
                    (String::from("maxItems"), JsonValue::Number(8)),
                    (String::from("uniqueItems"), JsonValue::Bool(true)),
                    (String::from("items"), bounded(IDENTITY_PATTERN)),
                ]),
            ),
            (String::from("selection_policy"), bounded(IDENTITY_PATTERN)),
            (String::from("profile_baseline"), profile_baseline()),
            (
                String::from("save_policy"),
                JsonValue::object([(
                    String::from("enum"),
                    JsonValue::Array(
                        ["disabled", "enabled"]
                            .into_iter()
                            .map(JsonValue::string)
                            .collect(),
                    ),
                )]),
            ),
            (String::from("compatibility"), compatibility()),
            (String::from("context_digest"), bounded(DIGEST_PATTERN)),
        ],
    )
}

fn profile_baseline() -> JsonValue {
    operation_schema(
        ["kind", "identity", "digest"],
        [
            (
                String::from("kind"),
                JsonValue::object([(
                    String::from("enum"),
                    JsonValue::Array(
                        ["fresh", "existing"]
                            .into_iter()
                            .map(JsonValue::string)
                            .collect(),
                    ),
                )]),
            ),
            (String::from("identity"), bounded(IDENTITY_PATTERN)),
            (String::from("digest"), bounded(DIGEST_PATTERN)),
        ],
    )
}

fn compatibility() -> JsonValue {
    operation_schema(
        ["game", "mod"],
        [
            (String::from("game"), identity_digest()),
            (String::from("mod"), identity_digest()),
        ],
    )
}

fn identity_digest() -> JsonValue {
    operation_schema(
        ["identity", "digest"],
        [
            (String::from("identity"), bounded(IDENTITY_PATTERN)),
            (String::from("digest"), bounded(DIGEST_PATTERN)),
        ],
    )
}

fn bounded(pattern: &str) -> JsonValue {
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

fn counter(maximum: i64) -> JsonValue {
    JsonValue::object([
        (String::from("type"), JsonValue::string("integer")),
        (String::from("minimum"), JsonValue::Number(0)),
        (String::from("maximum"), JsonValue::Number(maximum)),
    ])
}

fn seed() -> JsonValue {
    JsonValue::object([
        (String::from("type"), JsonValue::string("string")),
        (String::from("minLength"), JsonValue::Number(1)),
        (String::from("maxLength"), JsonValue::Number(64)),
        (String::from("pattern"), JsonValue::string(SEED_PATTERN)),
    ])
}

fn mode() -> JsonValue {
    JsonValue::object([(
        String::from("enum"),
        JsonValue::Array(
            ["seeded_training", "seeded_replay", "diagnostic"]
                .into_iter()
                .map(JsonValue::string)
                .collect(),
        ),
    )])
}
