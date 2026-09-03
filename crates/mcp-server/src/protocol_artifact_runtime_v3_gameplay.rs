// SPDX-License-Identifier: MIT

/// Version consumed by the bounded gameplay MCP mapping.
pub const RUNTIME_V3_GAMEPLAY_PROTOCOL_VERSION: &str = "runtime-v3-gameplay";
/// SHA-256 of the canonical Runtime-v3 gameplay schema source bytes.
pub const RUNTIME_V3_GAMEPLAY_SCHEMA_DIGEST: &str =
    "c961bbde893f0422f80233d14ea9ae8b648ee9032136e5370aa5f6b949f6575e";
/// Release-like artifact identity recorded by the protocol owner.
pub const RUNTIME_V3_GAMEPLAY_ARTIFACT: &str = "sts2-protocol/runtime-v3-gameplay";
/// Repository-relative source recorded in the v3 provenance.
pub const RUNTIME_V3_GAMEPLAY_SCHEMA_SOURCE: &str = "schemas/runtime-v3-gameplay.schema.json";
/// Generator recorded in the v3 provenance.
pub const RUNTIME_V3_GAMEPLAY_GENERATOR: &str = "hand-authored";
/// The only mutation admitted by this first gameplay expansion.
pub const RUNTIME_V3_GAMEPLAY_ACTION_ID: &str = "play_card";
/// The witness required for authoritative card-play settlement.
pub const RUNTIME_V3_GAMEPLAY_EFFECT_KIND: &str = "play_card_settled";
pub const RUNTIME_V3_GAMEPLAY_MAX_GENERATION: i64 = 9_007_199_254_740_991;
pub const RUNTIME_V3_GAMEPLAY_MAX_TURN_INDEX: i64 = 1024;
pub const RUNTIME_V3_GAMEPLAY_MAX_CARD_INDEX: i64 = 64;
pub const RUNTIME_V3_GAMEPLAY_MAX_ENERGY: i64 = 999;
pub const RUNTIME_V3_GAMEPLAY_MAX_PILE_COUNT: i64 = 1024;
pub const RUNTIME_V3_GAMEPLAY_MAX_ENEMIES: usize = 16;
