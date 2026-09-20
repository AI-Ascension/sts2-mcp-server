// SPDX-License-Identifier: MIT

//! Projection of the whole-manifest read onto an MCP tool result.
//!
//! The gateway is the schema authority for this envelope: it validates the whole pinned schema,
//! pins the content revision, and refuses a manifest above its own admitted framing bound. This
//! projection therefore states the pinned artifact's own identity, member sets, refusal vocabulary,
//! and framing ceiling rather than re-deriving them, and relays a complete catalog verbatim
//! otherwise. A catalog that is not exactly the pinned ten members, a refusal that carries catalog
//! data, a refusal pairing the pinned schema forbids, and a body beyond what the fixed route can
//! produce all fail closed instead of reaching a consumer.

use std::collections::BTreeMap;

use crate::json::JsonValue;
use crate::protocol_artifact_game_information_content_manifest::{
    CONTENT_MANIFEST_ARTIFACT, CONTENT_MANIFEST_GATEWAY_MAX_RESPONSE_BYTES,
    CONTENT_MANIFEST_GENERATOR, CONTENT_MANIFEST_PROTOCOL_VERSION, CONTENT_MANIFEST_SCHEMA_DIGEST,
    CONTENT_MANIFEST_SCHEMA_SOURCE,
};

use super::GameInformationContext;
use super::RESPONSE_TOO_LARGE;

/// The envelope's exact member set, taken from the pinned schema's `base`.
const ENVELOPE_FIELDS: [&str; 7] = [
    "protocol_version",
    "schema_digest",
    "provenance",
    "correlation_id",
    "kind",
    "manifest",
    "error",
];
/// The catalog's exact member set, taken from the pinned schema's `manifest`.
const CATALOG_FIELDS: [&str; 10] = [
    "game_build",
    "adapter_compatibility",
    "catalog_generation",
    "locale",
    "packages",
    "families",
    "definitions",
    "content_set_revision",
    "localized_text_revision",
    "inventory_revision",
];

pub(crate) fn project_content_manifest(
    body: &JsonValue,
    context: &GameInformationContext,
) -> Result<(JsonValue, bool), &'static str> {
    let object = body
        .as_object()
        .ok_or("content-manifest response is not an object")?;
    if object.len() != ENVELOPE_FIELDS.len()
        || ENVELOPE_FIELDS
            .iter()
            .any(|field| !object.contains_key(*field))
    {
        return Err("content-manifest envelope has unknown or missing fields");
    }
    if object.get("protocol_version") != Some(&JsonValue::string(CONTENT_MANIFEST_PROTOCOL_VERSION))
        || object.get("schema_digest") != Some(&JsonValue::string(CONTENT_MANIFEST_SCHEMA_DIGEST))
        || object.get("correlation_id") != Some(&JsonValue::string(context.correlation_id.as_str()))
    {
        return Err("content-manifest envelope identity does not match the request");
    }
    provenance(object)?;
    let is_error = match object.get("kind").and_then(JsonValue::as_string) {
        Some("content_manifest_response") => {
            complete_catalog(object)?;
            false
        }
        Some("error_response") => {
            refusal(object)?;
            true
        }
        _ => return Err("content-manifest response kind is not supported"),
    };
    bounded(body)?;
    Ok((body.clone(), is_error))
}

fn provenance(object: &BTreeMap<String, JsonValue>) -> Result<(), &'static str> {
    let provenance = object
        .get("provenance")
        .and_then(JsonValue::as_object)
        .ok_or("content-manifest provenance is missing")?;
    if provenance.len() != 3
        || provenance.get("artifact") != Some(&JsonValue::string(CONTENT_MANIFEST_ARTIFACT))
        || provenance.get("source") != Some(&JsonValue::string(CONTENT_MANIFEST_SCHEMA_SOURCE))
        || provenance.get("generator") != Some(&JsonValue::string(CONTENT_MANIFEST_GENERATOR))
    {
        return Err("content-manifest provenance does not match the pin");
    }
    Ok(())
}

fn complete_catalog(object: &BTreeMap<String, JsonValue>) -> Result<(), &'static str> {
    if object.get("error") != Some(&JsonValue::Null) {
        return Err("content-manifest response carries error data");
    }
    let catalog = object
        .get("manifest")
        .and_then(JsonValue::as_object)
        .ok_or("content-manifest response carries no catalog")?;
    if catalog.len() != CATALOG_FIELDS.len()
        || CATALOG_FIELDS
            .iter()
            .any(|field| !catalog.contains_key(*field))
    {
        return Err("content-manifest catalog has unknown or missing members");
    }
    Ok(())
}

fn refusal(object: &BTreeMap<String, JsonValue>) -> Result<(), &'static str> {
    if object.get("manifest") != Some(&JsonValue::Null) {
        return Err("content-manifest error response carries catalog data");
    }
    let error = object
        .get("error")
        .and_then(JsonValue::as_object)
        .ok_or("content-manifest error is not an object")?;
    if error.len() != 2 || !error.contains_key("code") || !error.contains_key("reason") {
        return Err("content-manifest error has unknown or missing fields");
    }
    let code = error
        .get("code")
        .and_then(JsonValue::as_string)
        .ok_or("content-manifest error code is not a token")?;
    let reason = match error.get("reason") {
        Some(JsonValue::Null) => None,
        Some(JsonValue::String(reason)) => Some(reason.as_str()),
        _ => return Err("content-manifest error reason is not a token"),
    };
    if !refusal_is_pinned(code, reason) {
        return Err("content-manifest refusal is outside the pinned vocabulary");
    }
    Ok(())
}

/// Whether one refusal is the pairing the pinned schema admits for its code.
///
/// The schema closes each code onto its own reason vocabulary, so `access_denied` carrying a
/// malformed-catalog reason is not a refusal this hop may relay.
fn refusal_is_pinned(code: &str, reason: Option<&str>) -> bool {
    let allowed: &[&str] = match code {
        "missing_capability" => &["source_unavailable"],
        "access_denied" => &["source_access_denied"],
        "malformed" => &[
            "source_malformed",
            "catalog_changed",
            "invalid_identity",
            "invalid_semantic_input",
            "invalid_localized_text",
            "invalid_package_version",
            "duplicate_package",
            "invalid_package_order",
            "duplicate_entity_kind",
            "unknown_entity_kind",
            "duplicate_definition",
            "unknown_origin_package",
            "origin_package_version_mismatch",
            "duplicate_override_reference",
        ],
        "result_limit_exceeded" => &["serialized_payload_too_large"],
        _ => return false,
    };
    reason.is_none_or(|reason| allowed.contains(&reason))
}

/// The framing ceiling this hop relays under.
///
/// The pinned profile admits a 16 MiB *message*, which is the producer's own serialization
/// ceiling, while the fixed route admits only the gateway's smaller framing bound. A body beyond
/// that bound is not a response the route can produce, so it is refused as a size error here
/// instead of being relayed as a plausible catalog.
fn bounded(value: &JsonValue) -> Result<(), &'static str> {
    if value.to_json().len() <= CONTENT_MANIFEST_GATEWAY_MAX_RESPONSE_BYTES {
        Ok(())
    } else {
        Err(RESPONSE_TOO_LARGE)
    }
}
