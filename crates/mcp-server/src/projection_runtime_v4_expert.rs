// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::json::JsonValue;
use crate::protocol_artifact_runtime_v4_expert::{
    RUNTIME_V4_EXPERT_ACTION_ARTIFACT, RUNTIME_V4_EXPERT_ACTION_GENERATOR,
    RUNTIME_V4_EXPERT_ACTION_PROTOCOL_VERSION, RUNTIME_V4_EXPERT_ACTION_SCHEMA_DIGEST,
    RUNTIME_V4_EXPERT_ACTION_SCHEMA_SOURCE, RUNTIME_V4_EXPERT_ARTIFACT,
    RUNTIME_V4_EXPERT_GENERATOR, RUNTIME_V4_EXPERT_PROTOCOL_VERSION,
    RUNTIME_V4_EXPERT_SCHEMA_DIGEST, RUNTIME_V4_EXPERT_SCHEMA_SOURCE,
};

const MAX_GENERATION: i64 = 9_007_199_254_740_991;
const MAX_TEXT_BYTES: usize = 512;
const MAX_ITEMS: usize = 256;
const ROOT_FIELDS: [&str; 11] = [
    "protocol_version",
    "schema_digest",
    "provenance",
    "profile",
    "state_id",
    "generation",
    "visible_seed",
    "run",
    "player",
    "state",
    "legal_actions",
];

pub(crate) fn project_runtime_v4_expert_gateway_body(
    body: &JsonValue,
) -> Result<JsonValue, &'static str> {
    let root = exact_object(Some(body), &ROOT_FIELDS, "Runtime-v4 expert state")?;
    validate_metadata(root)?;
    validate_identity(root.get("state_id"), "state_id")?;
    bounded_number(root.get("generation"), 0, MAX_GENERATION, "generation")?;
    optional_text(root.get("visible_seed"), "visible_seed")?;
    validate_run(root.get("run"))?;
    validate_player(root.get("player"))?;
    validate_state(root.get("state"))?;
    validate_legal_actions(root.get("legal_actions"))?;
    Ok(body.clone())
}

pub(crate) fn project_runtime_v4_expert_action_gateway_body(
    body: &JsonValue,
) -> Result<JsonValue, &'static str> {
    let root = exact_object(
        Some(body),
        &[
            "protocol_version",
            "schema_digest",
            "provenance",
            "profile",
            "correlation_id",
            "instance_id",
            "session_id",
            "lease_id",
            "lease_epoch",
            "generation",
            "state_id",
            "operation_id",
            "kind",
            "action",
            "status",
            "observation",
            "transition",
            "error_code",
        ],
        "Runtime-v4 expert action",
    )?;
    if root.get("protocol_version").and_then(JsonValue::as_string)
        != Some(RUNTIME_V4_EXPERT_ACTION_PROTOCOL_VERSION)
        || root.get("schema_digest").and_then(JsonValue::as_string)
            != Some(RUNTIME_V4_EXPERT_ACTION_SCHEMA_DIGEST)
        || root.get("profile").and_then(JsonValue::as_string) != Some("expert-action")
        || root.get("kind").and_then(JsonValue::as_string) != Some("action_response")
    {
        return Err("Runtime-v4 expert action metadata is unsupported");
    }
    let provenance = exact_object(
        root.get("provenance"),
        &["artifact", "source", "generator"],
        "Runtime-v4 action provenance",
    )?;
    if provenance.get("artifact").and_then(JsonValue::as_string)
        != Some(RUNTIME_V4_EXPERT_ACTION_ARTIFACT)
        || provenance.get("source").and_then(JsonValue::as_string)
            != Some(RUNTIME_V4_EXPERT_ACTION_SCHEMA_SOURCE)
        || provenance.get("generator").and_then(JsonValue::as_string)
            != Some(RUNTIME_V4_EXPERT_ACTION_GENERATOR)
    {
        return Err("Runtime-v4 action provenance is unsupported");
    }
    for field in [
        "correlation_id",
        "instance_id",
        "session_id",
        "lease_id",
        "state_id",
        "operation_id",
    ] {
        validate_identity(root.get(field), "Runtime-v4 action identity")?;
    }
    bounded_number(root.get("lease_epoch"), 0, MAX_GENERATION, "lease_epoch")?;
    let generation = bounded_number(root.get("generation"), 0, MAX_GENERATION, "generation")?;
    let status = root
        .get("status")
        .and_then(JsonValue::as_string)
        .ok_or("Runtime-v4 action status is invalid")?;
    if !matches!(
        status,
        "accepted" | "settled" | "rejected" | "unknown" | "cancelled"
    ) {
        return Err("Runtime-v4 action status is invalid");
    }
    match status {
        "accepted" => {
            validate_action_reference(root.get("action"))?;
            require_null(root.get("observation"), "action.observation")?;
            require_null(root.get("transition"), "action.transition")?;
            require_null(root.get("error_code"), "action.error_code")?;
        }
        "settled" => {
            validate_action_reference(root.get("action"))?;
            let observation = root
                .get("observation")
                .filter(|value| !matches!(value, JsonValue::Null))
                .ok_or("Runtime-v4 settled observation is missing")?;
            let observation = project_runtime_v4_expert_gateway_body(observation)?;
            let observed_generation = observation
                .as_object()
                .and_then(|value| value.get("generation"))
                .and_then(|value| match value {
                    JsonValue::Number(value) => Some(*value),
                    _ => None,
                })
                .ok_or("Runtime-v4 settled observation generation is missing")?;
            if observed_generation != generation {
                return Err("Runtime-v4 settled observation generation does not match response");
            }
            let transition = exact_object(
                root.get("transition"),
                &[
                    "kind",
                    "before_generation",
                    "after_generation",
                    "potion_id",
                    "removed",
                ],
                "Runtime-v4 action transition",
            )?;
            if transition.get("kind").and_then(JsonValue::as_string) != Some("potion_use_settled")
                || transition.get("removed") != Some(&JsonValue::Bool(true))
            {
                return Err("Runtime-v4 action transition is invalid");
            }
            let before = bounded_number(
                transition.get("before_generation"),
                0,
                MAX_GENERATION,
                "before_generation",
            )?;
            let after = bounded_number(
                transition.get("after_generation"),
                0,
                MAX_GENERATION,
                "after_generation",
            )?;
            if after <= before || after != generation {
                return Err("Runtime-v4 action transition generation is invalid");
            }
            let potion_id = transition
                .get("potion_id")
                .and_then(JsonValue::as_string)
                .ok_or("Runtime-v4 action transition potion_id is invalid")?;
            let action_potion_id = root
                .get("action")
                .and_then(JsonValue::as_object)
                .and_then(|value| value.get("action"))
                .and_then(JsonValue::as_object)
                .and_then(|value| value.get("potion_id"))
                .and_then(JsonValue::as_string)
                .ok_or("Runtime-v4 settled action potion_id is invalid")?;
            if potion_id != action_potion_id {
                return Err("Runtime-v4 action transition potion_id does not match action");
            }
            require_null(root.get("error_code"), "action.error_code")?;
        }
        "rejected" | "unknown" | "cancelled" => {
            optional_action_reference(root.get("action"))?;
            require_null(root.get("observation"), "action.observation")?;
            require_null(root.get("transition"), "action.transition")?;
            validate_identity(root.get("error_code"), "action.error_code")?;
        }
        _ => unreachable!(),
    }
    Ok(body.clone())
}

fn optional_action_reference(value: Option<&JsonValue>) -> Result<(), &'static str> {
    match value {
        Some(JsonValue::Null) => Ok(()),
        Some(value) => validate_action_reference(Some(value)),
        None => Err("Runtime-v4 action reference is missing"),
    }
}

fn require_null(value: Option<&JsonValue>, _field: &str) -> Result<(), &'static str> {
    matches!(value, Some(JsonValue::Null))
        .then_some(())
        .ok_or("Runtime-v4 action nullable field is not null")
}

fn validate_action_reference(value: Option<&JsonValue>) -> Result<(), &'static str> {
    let action = exact_object(
        value,
        &["action_id", "action"],
        "Runtime-v4 action reference",
    )?;
    validate_identity(action.get("action_id"), "action_id")?;
    let payload = exact_object(
        action.get("action"),
        &["kind", "potion_id", "target_id"],
        "Runtime-v4 potion action",
    )?;
    if payload.get("kind").and_then(JsonValue::as_string) != Some("use_potion") {
        return Err("Runtime-v4 potion action kind is invalid");
    }
    validate_identity(payload.get("potion_id"), "potion_id")?;
    optional_identity(payload.get("target_id"), "target_id")
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

fn validate_state(value: Option<&JsonValue>) -> Result<(), &'static str> {
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

fn validate_legal_actions(value: Option<&JsonValue>) -> Result<(), &'static str> {
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

fn exact_object<'a>(
    value: Option<&'a JsonValue>,
    fields: &[&str],
    label: &str,
) -> Result<&'a BTreeMap<String, JsonValue>, &'static str> {
    let object = value
        .and_then(JsonValue::as_object)
        .ok_or("Runtime-v4 value must be an object")?;
    exact_fields(object, fields, label)?;
    Ok(object)
}

fn exact_fields(
    object: &BTreeMap<String, JsonValue>,
    fields: &[&str],
    _label: &str,
) -> Result<(), &'static str> {
    if object.len() == fields.len() && fields.iter().all(|field| object.contains_key(*field)) {
        Ok(())
    } else {
        Err("Runtime-v4 value contains unknown or missing fields")
    }
}

fn validate_identity(value: Option<&JsonValue>, _field: &str) -> Result<(), &'static str> {
    match value.and_then(JsonValue::as_string) {
        Some(value) if valid_identity(value) => Ok(()),
        _ => Err("Runtime-v4 identity is invalid"),
    }
}

fn validate_identity_value(value: &JsonValue) -> Result<(), &'static str> {
    validate_identity(Some(value), "identity")
}

fn optional_identity(value: Option<&JsonValue>, _field: &str) -> Result<(), &'static str> {
    match value {
        Some(JsonValue::Null) => Ok(()),
        Some(value) => validate_identity(Some(value), "identity"),
        None => Err("Runtime-v4 optional identity is missing"),
    }
}

fn validate_text(value: Option<&JsonValue>, _field: &str) -> Result<(), &'static str> {
    match value.and_then(JsonValue::as_string) {
        Some(value) if valid_text(value) => Ok(()),
        _ => Err("Runtime-v4 text is invalid"),
    }
}

fn optional_text(value: Option<&JsonValue>, _field: &str) -> Result<(), &'static str> {
    match value {
        Some(JsonValue::Null) => Ok(()),
        Some(value) => validate_text(Some(value), "text"),
        None => Err("Runtime-v4 optional text is missing"),
    }
}

fn require_bool(value: Option<&JsonValue>, _field: &str) -> Result<(), &'static str> {
    matches!(value, Some(JsonValue::Bool(_)))
        .then_some(())
        .ok_or("Runtime-v4 boolean is invalid")
}

fn optional_bool(value: Option<&JsonValue>, _field: &str) -> Result<(), &'static str> {
    match value {
        Some(JsonValue::Null) => Ok(()),
        Some(value) => require_bool(Some(value), "boolean"),
        None => Err("Runtime-v4 optional boolean is missing"),
    }
}

fn bounded_number(
    value: Option<&JsonValue>,
    minimum: i64,
    maximum: i64,
    _field: &str,
) -> Result<i64, &'static str> {
    match value {
        Some(JsonValue::Number(value)) if (*value >= minimum) && (*value <= maximum) => Ok(*value),
        _ => Err("Runtime-v4 number is outside its bound"),
    }
}

fn optional_number(
    value: Option<&JsonValue>,
    minimum: i64,
    maximum: i64,
    _field: &str,
) -> Result<(), &'static str> {
    match value {
        Some(JsonValue::Null) => Ok(()),
        Some(value) => bounded_number(Some(value), minimum, maximum, "number").map(|_| ()),
        None => Err("Runtime-v4 optional number is missing"),
    }
}

fn array(
    value: Option<&JsonValue>,
    maximum: usize,
    validate: fn(&JsonValue) -> Result<(), &'static str>,
    _field: &str,
) -> Result<(), &'static str> {
    let values = value
        .and_then(|value| match value {
            JsonValue::Array(values) => Some(values),
            _ => None,
        })
        .ok_or("Runtime-v4 value must be an array")?;
    if values.len() > maximum {
        return Err("Runtime-v4 array exceeds its bound");
    }
    values.iter().try_for_each(validate)
}

fn nullable_array(
    value: Option<&JsonValue>,
    maximum: usize,
    validate: fn(&JsonValue) -> Result<(), &'static str>,
    _field: &str,
) -> Result<(), &'static str> {
    match value {
        Some(JsonValue::Null) => Ok(()),
        Some(_) => array(value, maximum, validate, "array"),
        None => Err("Runtime-v4 nullable array is missing"),
    }
}

fn optional_identity_array(value: Option<&JsonValue>, field: &str) -> Result<(), &'static str> {
    match value {
        Some(JsonValue::Null) => Ok(()),
        Some(_) => array(value, 16, validate_identity_value, field),
        None => Err("Runtime-v4 target identity array is missing"),
    }
}

fn valid_identity(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_TEXT_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._:/-".contains(&byte))
}

fn valid_text(value: &str) -> bool {
    !value.is_empty() && value.len() <= MAX_TEXT_BYTES && !value.chars().any(char::is_control)
}
