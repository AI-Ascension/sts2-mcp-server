// SPDX-License-Identifier: MIT

use std::collections::{BTreeMap, BTreeSet};

use crate::json::JsonValue;

#[path = "projection_coop_native_observation.rs"]
mod observation;
#[path = "projection_coop_native_shapes.rs"]
mod shapes;
use crate::protocol_artifact_coop_native::{
    COOP_NATIVE_ARTIFACT, COOP_NATIVE_GENERATOR, COOP_NATIVE_MAX_BODY_BYTES,
    COOP_NATIVE_PROTOCOL_VERSION, COOP_NATIVE_SCHEMA_DIGEST, COOP_NATIVE_SCHEMA_SOURCE,
};
use shapes::{effect_response, observation_response, recovery_response};

const TOP_LEVEL_FIELDS: [&str; 18] = [
    "protocol_version",
    "schema_digest",
    "provenance",
    "correlation_id",
    "instance_id",
    "session_id",
    "lease_id",
    "lease_epoch",
    "kind",
    "operation_id",
    "actor_peer",
    "expected_host_generation",
    "action",
    "vote",
    "status",
    "observation",
    "effect",
    "recovery",
];
const MAX_LEGAL_CATALOG_BYTES: usize = 128 * 1024;
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct NativeContext {
    pub(crate) correlation: String,
    pub(crate) instance: String,
    pub(crate) session: String,
    pub(crate) lease: String,
    pub(crate) epoch: i64,
}

pub(crate) fn project_coop_native_response(
    body: &JsonValue,
    context: &NativeContext,
    expected_kind: &str,
    status_code: u16,
    expected_operation: Option<&str>,
) -> Result<(JsonValue, bool), &'static str> {
    let object = exact_object(body, &TOP_LEVEL_FIELDS, "native co-op response")?;
    if body.to_json().len() > COOP_NATIVE_MAX_BODY_BYTES {
        return Err("native co-op response exceeds the protocol body limit");
    }
    validate_metadata(object, context, expected_kind)?;
    if let Some(expected_operation) = expected_operation
        && object.get("operation_id").and_then(JsonValue::as_string) != Some(expected_operation)
    {
        return Err("native co-op response operation identity mismatched");
    }
    match expected_kind {
        "observation" => observation_response(object)?,
        "effect_response" => effect_response(object)?,
        "recovery_response" => recovery_response(object)?,
        _ => return Err("native co-op response kind is unsupported"),
    }
    let status = object
        .get("status")
        .and_then(JsonValue::as_string)
        .unwrap_or("");
    Ok((
        body.clone(),
        !(200..300).contains(&status_code) || matches!(status, "rejected" | "unknown"),
    ))
}

pub(crate) fn project_coop_native_effect(
    body: &JsonValue,
    context: &NativeContext,
) -> Result<(JsonValue, bool), &'static str> {
    project_coop_native_response(body, context, "effect_response", 200, None)
}

pub(crate) fn project_coop_native_legal_catalog(
    body: &JsonValue,
    context: &NativeContext,
    expected_generation: i64,
    status_code: u16,
) -> Result<(JsonValue, bool), &'static str> {
    if !(200..300).contains(&status_code) {
        return Err("native legal catalog request was rejected");
    }
    if body.to_json().len() > MAX_LEGAL_CATALOG_BYTES {
        return Err("native legal catalog exceeds the response limit");
    }
    let object = exact_object(
        body,
        &[
            "instance_id",
            "session_id",
            "lease_id",
            "lease_epoch",
            "host_generation",
            "actor_peer",
            "legal_actions",
            "legal_votes",
        ],
        "native legal catalog",
    )?;
    if object.get("instance_id").and_then(JsonValue::as_string) != Some(context.instance.as_str())
        || object.get("session_id").and_then(JsonValue::as_string) != Some(context.session.as_str())
        || object.get("lease_id").and_then(JsonValue::as_string) != Some(context.lease.as_str())
        || object.get("lease_epoch") != Some(&JsonValue::Number(context.epoch))
        || object.get("host_generation") != Some(&JsonValue::Number(expected_generation))
    {
        return Err("native legal catalog identity or generation mismatched");
    }
    let actor = object
        .get("actor_peer")
        .and_then(JsonValue::as_string)
        .filter(|value| legal_peer_identity(value))
        .ok_or("native legal catalog actor is invalid")?;
    let actions = object
        .get("legal_actions")
        .and_then(JsonValue::as_array)
        .ok_or("native legal action catalog is missing")?;
    let votes = object
        .get("legal_votes")
        .and_then(JsonValue::as_array)
        .ok_or("native legal vote catalog is missing")?;
    if actions.len() > 256 || votes.len() > 256 {
        return Err("native legal catalog exceeds its bound");
    }
    let mut ids = BTreeSet::new();
    for action in actions {
        let value = exact_object(
            action,
            &["kind", "action_id", "target_peer"],
            "native legal action",
        )?;
        if !matches!(
            value.get("kind").and_then(JsonValue::as_string),
            Some("play_card" | "end_turn" | "select_card" | "choose_reward" | "confirm_selection")
        ) || !value
            .get("action_id")
            .and_then(JsonValue::as_string)
            .is_some_and(observation::safe_identity)
            || !value.get("target_peer").is_some_and(|target| {
                matches!(target, JsonValue::Null)
                    || target.as_string().is_some_and(legal_peer_identity)
            })
            || !value
                .get("action_id")
                .and_then(JsonValue::as_string)
                .is_some_and(|id| ids.insert(id.to_owned()))
        {
            return Err("native legal action catalog entry is invalid");
        }
    }
    for vote in votes {
        let value = exact_object(
            vote,
            &["proposal_id", "voter_peer", "choice"],
            "native legal vote",
        )?;
        let proposal = value
            .get("proposal_id")
            .and_then(JsonValue::as_string)
            .filter(|v| observation::safe_identity(v));
        let voter = value.get("voter_peer").and_then(JsonValue::as_string);
        let choice = value
            .get("choice")
            .and_then(JsonValue::as_string)
            .filter(|v| observation::safe_identity(v));
        let Some((proposal, voter, choice)) =
            proposal.zip(voter).zip(choice).map(|((a, b), c)| (a, b, c))
        else {
            return Err("native legal vote catalog entry is invalid");
        };
        if voter != actor || !legal_peer_identity(voter) {
            return Err("native legal vote voter is invalid");
        }
        let id = format!("vote:{proposal}:{choice}");
        if !observation::safe_identity(&id) || !ids.insert(id) {
            return Err("native legal catalog IDs are not unique");
        }
    }
    Ok((body.clone(), false))
}

fn legal_peer_identity(value: &str) -> bool {
    value
        .strip_prefix("peer:")
        .is_some_and(|suffix| suffix.len() >= 5 && observation::safe_identity(value))
}

fn validate_metadata(
    object: &BTreeMap<String, JsonValue>,
    context: &NativeContext,
    expected_kind: &str,
) -> Result<(), &'static str> {
    for (field, expected) in [
        ("protocol_version", COOP_NATIVE_PROTOCOL_VERSION),
        ("schema_digest", COOP_NATIVE_SCHEMA_DIGEST),
        ("correlation_id", context.correlation.as_str()),
        ("instance_id", context.instance.as_str()),
        ("session_id", context.session.as_str()),
        ("lease_id", context.lease.as_str()),
        ("kind", expected_kind),
    ] {
        if object.get(field).and_then(JsonValue::as_string) != Some(expected) {
            return Err("native co-op response metadata or identity mismatched");
        }
    }
    if object.get("lease_epoch") != Some(&JsonValue::Number(context.epoch)) {
        return Err("native co-op response lease epoch mismatched");
    }
    let provenance = exact_object(
        object
            .get("provenance")
            .ok_or("native co-op response provenance is missing")?,
        &["artifact", "source", "generator"],
        "native co-op response provenance",
    )?;
    for (field, expected) in [
        ("artifact", COOP_NATIVE_ARTIFACT),
        ("source", COOP_NATIVE_SCHEMA_SOURCE),
        ("generator", COOP_NATIVE_GENERATOR),
    ] {
        if provenance.get(field).and_then(JsonValue::as_string) != Some(expected) {
            return Err("native co-op response provenance is unsupported");
        }
    }
    Ok(())
}

fn exact_object<'a>(
    value: &'a JsonValue,
    fields: &[&str],
    label: &'static str,
) -> Result<&'a BTreeMap<String, JsonValue>, &'static str> {
    let object = value.as_object().ok_or(label)?;
    if object.len() != fields.len() || fields.iter().any(|field| !object.contains_key(*field)) {
        return Err(label);
    }
    Ok(object)
}
