// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::json::JsonValue;
use crate::protocol_artifact_coop_native::{
    COOP_NATIVE_ARTIFACT, COOP_NATIVE_GENERATOR, COOP_NATIVE_PROTOCOL_VERSION,
    COOP_NATIVE_SCHEMA_DIGEST, COOP_NATIVE_SCHEMA_SOURCE,
};

use super::{NativeContext, exact_object, observation};

pub(super) fn legal_peer_identity(value: &str) -> bool {
    value
        .strip_prefix("peer:")
        .is_some_and(|suffix| suffix.len() >= 5 && observation::safe_identity(value))
}

pub(super) fn validate_metadata(
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
