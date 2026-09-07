// SPDX-License-Identifier: MIT

use super::*;

pub(super) fn validate_state(value: Option<&JsonValue>) -> Result<(), &'static str> {
    let state = value
        .and_then(JsonValue::as_object)
        .ok_or("Runtime-v4 state must be an object")?;
    let kind = state
        .get("state")
        .and_then(JsonValue::as_string)
        .ok_or("Runtime-v4 state discriminator is invalid")?;
    match kind {
        "setup" => {
            exact_fields(state, &["state", "characters"], "Runtime-v4 setup")?;
            array(
                state.get("characters"),
                MAX_ITEMS,
                validate_identity_value,
                "setup.characters",
            )
        }
        "map" => {
            exact_fields(
                state,
                &["state", "current_node_id", "nodes", "edges", "options"],
                "Runtime-v4 map",
            )?;
            optional_identity(state.get("current_node_id"), "map.current_node_id")?;
            nullable_array(
                state.get("nodes"),
                MAX_ITEMS,
                validate_map_node,
                "map.nodes",
            )?;
            nullable_array(
                state.get("edges"),
                MAX_ITEMS * 2,
                validate_map_edge,
                "map.edges",
            )?;
            array(
                state.get("options"),
                MAX_ITEMS,
                validate_identity_value,
                "map.options",
            )
        }
        "combat" => {
            exact_fields(
                state,
                &["state", "turn_index", "enemies"],
                "Runtime-v4 combat",
            )?;
            bounded_number(state.get("turn_index"), 0, 65_535, "combat.turn_index")?;
            nullable_array(
                state.get("enemies"),
                MAX_ITEMS,
                validate_enemy,
                "combat.enemies",
            )
        }
        "reward" | "event" | "rest" | "selection" => {
            exact_fields(state, &["state", "choices"], "Runtime-v4 choice state")?;
            nullable_array(
                state.get("choices"),
                MAX_ITEMS,
                validate_choice,
                "state.choices",
            )
        }
        "shop" => {
            exact_fields(state, &["state", "items"], "Runtime-v4 shop")?;
            nullable_array(
                state.get("items"),
                MAX_ITEMS,
                validate_shop_item,
                "shop.items",
            )
        }
        "victory" => exact_fields(state, &["state"], "Runtime-v4 victory"),
        "defeat" => {
            exact_fields(state, &["state", "reason"], "Runtime-v4 defeat")?;
            optional_text(state.get("reason"), "defeat.reason")
        }
        "recovery" => {
            exact_fields(state, &["state", "code"], "Runtime-v4 recovery")?;
            validate_identity(state.get("code"), "recovery.code")
        }
        _ => Err("Runtime-v4 state discriminator is unsupported"),
    }
}

fn validate_enemy(value: &JsonValue) -> Result<(), &'static str> {
    let enemy = exact_object(
        Some(value),
        &[
            "enemy_id", "name", "hp", "max_hp", "block", "powers", "statuses", "intent",
        ],
        "Runtime-v4 enemy",
    )?;
    validate_identity(enemy.get("enemy_id"), "enemy.enemy_id")?;
    validate_text(enemy.get("name"), "enemy.name")?;
    let hp = bounded_number(enemy.get("hp"), 0, 65_535, "enemy.hp")?;
    let max_hp = bounded_number(enemy.get("max_hp"), 0, 65_535, "enemy.max_hp")?;
    if hp > max_hp {
        return Err("Runtime-v4 enemy hp exceeds max_hp");
    }
    optional_number(enemy.get("block"), 0, 65_535, "enemy.block")?;
    nullable_array(
        enemy.get("powers"),
        MAX_ITEMS,
        validate_status,
        "enemy.powers",
    )?;
    nullable_array(
        enemy.get("statuses"),
        MAX_ITEMS,
        validate_status,
        "enemy.statuses",
    )?;
    validate_intent(enemy.get("intent"))
}

fn validate_intent(value: Option<&JsonValue>) -> Result<(), &'static str> {
    let intent = value
        .and_then(JsonValue::as_object)
        .ok_or("Runtime-v4 enemy intent must be an object")?;
    match intent.get("kind").and_then(JsonValue::as_string) {
        Some("attack") => {
            exact_fields(
                intent,
                &["kind", "damage", "hits", "target_ids"],
                "Runtime-v4 attack intent",
            )?;
            bounded_number(intent.get("damage"), 0, 65_535, "intent.damage")?;
            bounded_number(intent.get("hits"), 1, 255, "intent.hits")?;
            optional_identity_array(intent.get("target_ids"), "intent.target_ids")
        }
        Some("defend" | "buff" | "debuff" | "unknown") => {
            exact_fields(intent, &["kind", "target_ids"], "Runtime-v4 intent")?;
            optional_identity_array(intent.get("target_ids"), "intent.target_ids")
        }
        _ => Err("Runtime-v4 enemy intent kind is unsupported"),
    }
}

fn validate_map_node(value: &JsonValue) -> Result<(), &'static str> {
    let node = exact_object(
        Some(value),
        &["node_id", "act", "row", "col", "kind", "reachable"],
        "Runtime-v4 map node",
    )?;
    validate_identity(node.get("node_id"), "map_node.node_id")?;
    bounded_number(node.get("act"), 0, 255, "map_node.act")?;
    bounded_number(node.get("row"), 0, 255, "map_node.row")?;
    bounded_number(node.get("col"), 0, 255, "map_node.col")?;
    validate_identity(node.get("kind"), "map_node.kind")?;
    require_bool(node.get("reachable"), "map_node.reachable")
}

fn validate_map_edge(value: &JsonValue) -> Result<(), &'static str> {
    let edge = exact_object(Some(value), &["from", "to"], "Runtime-v4 map edge")?;
    validate_identity(edge.get("from"), "map_edge.from")?;
    validate_identity(edge.get("to"), "map_edge.to")
}

fn validate_choice(value: &JsonValue) -> Result<(), &'static str> {
    let choice = exact_object(
        Some(value),
        &["choice_id", "label", "kind", "domain"],
        "Runtime-v4 choice",
    )?;
    validate_identity(choice.get("choice_id"), "choice.choice_id")?;
    validate_text(choice.get("label"), "choice.label")?;
    validate_identity(choice.get("kind"), "choice.kind")?;
    optional_identity_array(choice.get("domain"), "choice.domain")
}

fn validate_shop_item(value: &JsonValue) -> Result<(), &'static str> {
    let item = exact_object(
        Some(value),
        &["item_id", "name", "kind", "price"],
        "Runtime-v4 shop item",
    )?;
    validate_identity(item.get("item_id"), "shop_item.item_id")?;
    validate_text(item.get("name"), "shop_item.name")?;
    validate_identity(item.get("kind"), "shop_item.kind")?;
    bounded_number(item.get("price"), 0, 4_294_967_295, "shop_item.price").map(|_| ())
}

pub(super) fn validate_legal_actions(value: Option<&JsonValue>) -> Result<(), &'static str> {
    array(value, MAX_ITEMS, validate_legal_action, "legal_actions")
}

fn validate_legal_action(value: &JsonValue) -> Result<(), &'static str> {
    let action = exact_object(
        Some(value),
        &["action_id", "action"],
        "Runtime-v4 legal action",
    )?;
    validate_identity(action.get("action_id"), "legal_action.action_id")?;
    validate_action(action.get("action"))
}

fn validate_action(value: Option<&JsonValue>) -> Result<(), &'static str> {
    let action = value
        .and_then(JsonValue::as_object)
        .ok_or("Runtime-v4 action must be an object")?;
    let kind = action
        .get("kind")
        .and_then(JsonValue::as_string)
        .ok_or("Runtime-v4 action kind is invalid")?;
    let fields = match kind {
        "start_run" | "select_character" => &["kind", "character_id"][..],
        "select_map_node" => &["kind", "node_id"][..],
        "play_card" => &["kind", "card_id", "target_id"][..],
        "use_potion" => &["kind", "potion_id", "target_id"][..],
        "end_turn" | "skip_reward" | "proceed" | "rest" | "confirm_victory" | "save_quit" => {
            &["kind"][..]
        }
        "rest_option" => &["kind", "rest_option_id"][..],
        "choose_reward" => &["kind", "reward_id"][..],
        "shop_purchase" => &["kind", "item_id"][..],
        "shop_remove" | "smith" => &["kind", "card_id"][..],
        "event_choice" => &["kind", "choice_id"][..],
        "select_card" => &["kind", "selection_id", "card_id"][..],
        "confirm_selection" | "cancel_selection" => &["kind", "selection_id"][..],
        _ => return Err("Runtime-v4 action kind is unsupported"),
    };
    exact_fields(action, fields, "Runtime-v4 action")?;
    for field in fields.iter().skip(1) {
        if *field == "target_id" || *field == "selection_id" {
            optional_identity(action.get(*field), "action identity")?;
        } else {
            validate_identity(action.get(*field), "action identity")?;
        }
    }
    Ok(())
}
