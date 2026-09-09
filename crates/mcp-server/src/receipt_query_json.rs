// SPDX-License-Identifier: MIT

//! Canonical wire serializer for the neutral retained-receipt profile.
//!
//! `JsonValue` stores object members in a sorted map, while this profile has a
//! documented wire order.  The gateway checks that order byte-for-byte, so the
//! MCP HTTP adapter uses this serializer for the request body instead of the
//! ordinary map serializer.

use std::collections::BTreeMap;

use crate::json::JsonValue;

pub const TOP_LEVEL_FIELDS: [&str; 24] = [
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
    "action_kind",
    "action_fingerprint",
    "run_id",
    "location",
    "actor_id",
    "authority_id",
    "authority_epoch",
    "expected_host_generation",
    "before_host_generation",
    "participant_ids",
    "status",
    "evidence_scope",
    "receipt",
    "error_code",
];
const PROVENANCE_FIELDS: [&str; 3] = ["artifact", "source", "generator"];
const LOCATION_FIELDS: [&str; 3] = ["act_index", "room_id", "coord"];
const COORDINATE_FIELDS: [&str; 2] = ["col", "row"];
const RECEIPT_FIELDS: [&str; 7] = [
    "status",
    "after_host_generation",
    "checkpoint_id",
    "state_digest",
    "effect_id",
    "effect_kind",
    "error_code",
];

pub fn canonical_coop_receipt_query(value: &JsonValue) -> Option<String> {
    let object = value.as_object()?;
    if object.len() != TOP_LEVEL_FIELDS.len() {
        return None;
    }
    let mut output = String::from("{");
    for (index, field) in TOP_LEVEL_FIELDS.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        append_key(&mut output, field);
        match *field {
            "provenance" => append_provenance(&mut output, object.get(*field)?)?,
            "location" => append_location(&mut output, object.get(*field)?)?,
            "participant_ids" => append_participants(&mut output, object.get(*field)?)?,
            "receipt" => append_receipt(&mut output, object.get(*field)?)?,
            _ => append_scalar(&mut output, object.get(*field)?)?,
        }
    }
    output.push('}');
    output.push('\n');
    Some(output)
}

fn append_key(output: &mut String, key: &str) {
    output.push_str(&JsonValue::string(key).to_json());
    output.push(':');
}

fn append_scalar(output: &mut String, value: &JsonValue) -> Option<()> {
    match value {
        JsonValue::Null | JsonValue::Bool(_) | JsonValue::Number(_) | JsonValue::String(_) => {
            output.push_str(&value.to_json());
            Some(())
        }
        JsonValue::Array(_) | JsonValue::Object(_) => None,
    }
}

fn append_provenance(output: &mut String, value: &JsonValue) -> Option<()> {
    append_ordered_object(output, value, &PROVENANCE_FIELDS, |field, value, output| {
        append_scalar_member(field, value, output)
    })
}

fn append_location(output: &mut String, value: &JsonValue) -> Option<()> {
    append_ordered_object(output, value, &LOCATION_FIELDS, |field, value, output| {
        if field != "coord" || value == &JsonValue::Null {
            return append_scalar_member(field, value, output);
        }
        append_key(output, field);
        append_ordered_object(output, value, &COORDINATE_FIELDS, |field, value, output| {
            append_scalar_member(field, value, output)
        })
    })
}

fn append_participants(output: &mut String, value: &JsonValue) -> Option<()> {
    let values = value.as_array()?;
    output.push('[');
    for (index, value) in values.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        append_scalar(output, value)?;
    }
    output.push(']');
    Some(())
}

fn append_receipt(output: &mut String, value: &JsonValue) -> Option<()> {
    if value == &JsonValue::Null {
        return append_scalar(output, value);
    }
    append_ordered_object(output, value, &RECEIPT_FIELDS, |field, value, output| {
        append_scalar_member(field, value, output)
    })
}

fn append_ordered_object(
    output: &mut String,
    value: &JsonValue,
    fields: &[&str],
    mut append_value: impl FnMut(&str, &JsonValue, &mut String) -> Option<()>,
) -> Option<()> {
    let object: &BTreeMap<String, JsonValue> = value.as_object()?;
    if object.len() != fields.len() {
        return None;
    }
    output.push('{');
    for (index, field) in fields.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        append_value(field, object.get(*field)?, output)?;
    }
    output.push('}');
    Some(())
}

fn append_scalar_member(field: &str, value: &JsonValue, output: &mut String) -> Option<()> {
    append_key(output, field);
    append_scalar(output, value)
}
