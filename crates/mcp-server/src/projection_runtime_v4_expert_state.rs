// SPDX-License-Identifier: MIT

use super::*;

#[path = "projection_runtime_v4_expert_state_shape.rs"]
mod shape;

pub(super) fn project(body: &JsonValue) -> Result<JsonValue, &'static str> {
    let root = exact_object(Some(body), &ROOT_FIELDS, "Runtime-v4 expert state")?;
    validate_metadata(root)?;
    validate_identity(root.get("state_id"), "state_id")?;
    bounded_number(root.get("generation"), 0, MAX_GENERATION, "generation")?;
    optional_text(root.get("visible_seed"), "visible_seed")?;
    validate_run(root.get("run"))?;
    validate_player(root.get("player"))?;
    shape::validate_state(root.get("state"))?;
    shape::validate_legal_actions(root.get("legal_actions"))?;
    Ok(body.clone())
}

fn validate_metadata(root: &BTreeMap<String, JsonValue>) -> Result<(), &'static str> {
    if root.get("protocol_version").and_then(JsonValue::as_string)
        != Some(RUNTIME_V4_EXPERT_PROTOCOL_VERSION)
        || root.get("schema_digest").and_then(JsonValue::as_string)
            != Some(RUNTIME_V4_EXPERT_SCHEMA_DIGEST)
        || root.get("profile").and_then(JsonValue::as_string) != Some("expert-state")
    {
        return Err("Runtime-v4 expert metadata is unsupported");
    }
    let provenance = exact_object(
        root.get("provenance"),
        &["artifact", "source", "generator"],
        "Runtime-v4 provenance",
    )?;
    if provenance.get("artifact").and_then(JsonValue::as_string) != Some(RUNTIME_V4_EXPERT_ARTIFACT)
        || provenance.get("source").and_then(JsonValue::as_string)
            != Some(RUNTIME_V4_EXPERT_SCHEMA_SOURCE)
        || provenance.get("generator").and_then(JsonValue::as_string)
            != Some(RUNTIME_V4_EXPERT_GENERATOR)
    {
        return Err("Runtime-v4 provenance is unsupported");
    }
    Ok(())
}

fn validate_run(value: Option<&JsonValue>) -> Result<(), &'static str> {
    let run = exact_object(
        value,
        &["character_id", "act", "location"],
        "Runtime-v4 run",
    )?;
    optional_identity(run.get("character_id"), "run.character_id")?;
    optional_number(run.get("act"), 0, 255, "run.act")?;
    optional_identity(run.get("location"), "run.location")
}

fn validate_player(value: Option<&JsonValue>) -> Result<(), &'static str> {
    let player = exact_object(
        value,
        &[
            "hp",
            "max_hp",
            "block",
            "energy",
            "gold",
            "hand",
            "deck",
            "discard",
            "exhaust",
            "powers",
            "statuses",
            "relics",
            "potions",
            "potion_slots",
            "max_potion_slots",
        ],
        "Runtime-v4 player",
    )?;
    let hp = bounded_number(player.get("hp"), 0, 65_535, "player.hp")?;
    let max_hp = bounded_number(player.get("max_hp"), 0, 65_535, "player.max_hp")?;
    if hp > max_hp {
        return Err("Runtime-v4 player hp exceeds max_hp");
    }
    optional_number(player.get("block"), 0, 65_535, "player.block")?;
    bounded_number(player.get("energy"), 0, 255, "player.energy")?;
    bounded_number(player.get("gold"), 0, 4_294_967_295, "player.gold")?;
    nullable_array(player.get("hand"), MAX_ITEMS, validate_card, "player.hand")?;
    nullable_array(player.get("deck"), MAX_ITEMS, validate_card, "player.deck")?;
    nullable_array(
        player.get("discard"),
        MAX_ITEMS,
        validate_card,
        "player.discard",
    )?;
    nullable_array(
        player.get("exhaust"),
        MAX_ITEMS,
        validate_card,
        "player.exhaust",
    )?;
    nullable_array(
        player.get("powers"),
        MAX_ITEMS,
        validate_status,
        "player.powers",
    )?;
    nullable_array(
        player.get("statuses"),
        MAX_ITEMS,
        validate_status,
        "player.statuses",
    )?;
    nullable_array(
        player.get("relics"),
        MAX_ITEMS,
        validate_relic,
        "player.relics",
    )?;
    nullable_array(
        player.get("potions"),
        MAX_ITEMS,
        validate_potion,
        "player.potions",
    )?;
    optional_number(player.get("potion_slots"), 0, 255, "player.potion_slots")?;
    optional_number(
        player.get("max_potion_slots"),
        0,
        255,
        "player.max_potion_slots",
    )
}

fn validate_card(value: &JsonValue) -> Result<(), &'static str> {
    let card = exact_object(
        Some(value),
        &[
            "card_id",
            "name",
            "cost",
            "upgraded",
            "type",
            "rarity",
            "target",
            "description",
        ],
        "Runtime-v4 card",
    )?;
    validate_identity(card.get("card_id"), "card.card_id")?;
    validate_text(card.get("name"), "card.name")?;
    optional_number(card.get("cost"), 0, 255, "card.cost")?;
    require_bool(card.get("upgraded"), "card.upgraded")?;
    optional_identity(card.get("type"), "card.type")?;
    optional_identity(card.get("rarity"), "card.rarity")?;
    optional_identity(card.get("target"), "card.target")?;
    optional_text(card.get("description"), "card.description")
}

fn validate_status(value: &JsonValue) -> Result<(), &'static str> {
    let status = exact_object(
        Some(value),
        &["status_id", "name", "amount"],
        "Runtime-v4 status",
    )?;
    validate_identity(status.get("status_id"), "status.status_id")?;
    validate_text(status.get("name"), "status.name")?;
    optional_number(status.get("amount"), -65_535, 65_535, "status.amount")
}

fn validate_relic(value: &JsonValue) -> Result<(), &'static str> {
    let relic = exact_object(Some(value), &["relic_id", "name"], "Runtime-v4 relic")?;
    validate_identity(relic.get("relic_id"), "relic.relic_id")?;
    validate_text(relic.get("name"), "relic.name")
}

fn validate_potion(value: &JsonValue) -> Result<(), &'static str> {
    let potion = exact_object(
        Some(value),
        &["potion_id", "name", "slot", "usable", "target_mode"],
        "Runtime-v4 potion",
    )?;
    validate_identity(potion.get("potion_id"), "potion.potion_id")?;
    validate_text(potion.get("name"), "potion.name")?;
    optional_number(potion.get("slot"), 0, 255, "potion.slot")?;
    optional_bool(potion.get("usable"), "potion.usable")?;
    match potion.get("target_mode").and_then(JsonValue::as_string) {
        Some("self" | "any_enemy" | "all_enemies" | "none" | "unknown") => Ok(()),
        _ => Err("Runtime-v4 potion target_mode is invalid"),
    }
}
