// SPDX-License-Identifier: MIT

use super::*;
use crate::protocol::RequestId;

pub(super) const VALID_GOLDENS: [(&str, &str); 6] = [
    (
        "LBR-VALID-DISCOVERY-INITIAL",
        include_str!(
            "../../../protocol-artifact/game-information-lookup-binding-v1/golden/discovery-response.json"
        ),
    ),
    (
        "LBR-VALID-DISCOVERY-OBSERVED",
        include_str!(
            "../../../protocol-artifact/game-information-lookup-binding-v1/golden/observation-response.json"
        ),
    ),
    (
        "LBR-VALID-REOBSERVE-REQUIRED",
        include_str!(
            "../../../protocol-artifact/game-information-lookup-binding-v1/golden/reobserve-required-response.json"
        ),
    ),
    (
        "LBR-VALID-REOBSERVE-EXHAUSTED",
        include_str!(
            "../../../protocol-artifact/game-information-lookup-binding-v1/golden/reobserve-exhausted-response.json"
        ),
    ),
    (
        "LBR-VALID-REOBSERVE-UNAVAILABLE",
        include_str!(
            "../../../protocol-artifact/game-information-lookup-binding-v1/golden/reobserve-unavailable-response.json"
        ),
    ),
    (
        "LBR-VALID-REOBSERVED",
        include_str!(
            "../../../protocol-artifact/game-information-lookup-binding-v1/golden/reobserved-response.json"
        ),
    ),
];

pub(super) const INVALID_FIXTURES: [(&str, &str); 12] = [
    (
        "LBR-INVALID-WRONG-SCOPE",
        include_str!(
            "../../../protocol-artifact/game-information-lookup-binding-v1/conformance/fixtures/game-information-lookup-binding-v1/invalid/wrong-scope.json"
        ),
    ),
    (
        "LBR-INVALID-WRONG-INSTANCE",
        include_str!(
            "../../../protocol-artifact/game-information-lookup-binding-v1/conformance/fixtures/game-information-lookup-binding-v1/invalid/wrong-instance.json"
        ),
    ),
    (
        "LBR-INVALID-FORGED-BINDING-ID",
        include_str!(
            "../../../protocol-artifact/game-information-lookup-binding-v1/conformance/fixtures/game-information-lookup-binding-v1/invalid/forged-binding-id.json"
        ),
    ),
    (
        "LBR-INVALID-MIXED-BINDING",
        include_str!(
            "../../../protocol-artifact/game-information-lookup-binding-v1/conformance/fixtures/game-information-lookup-binding-v1/invalid/mixed-binding.json"
        ),
    ),
    (
        "LBR-INVALID-STALE-OBSERVATION",
        include_str!(
            "../../../protocol-artifact/game-information-lookup-binding-v1/conformance/fixtures/game-information-lookup-binding-v1/invalid/stale-observation.json"
        ),
    ),
    (
        "LBR-INVALID-MISSING-CAPABILITY",
        include_str!(
            "../../../protocol-artifact/game-information-lookup-binding-v1/conformance/fixtures/game-information-lookup-binding-v1/invalid/missing-capability.json"
        ),
    ),
    (
        "LBR-INVALID-SUPERSEDES-MISMATCH",
        include_str!(
            "../../../protocol-artifact/game-information-lookup-binding-v1/conformance/fixtures/game-information-lookup-binding-v1/invalid/supersedes-mismatch.json"
        ),
    ),
    (
        "LBR-INVALID-UNKNOWN-VERSION",
        include_str!(
            "../../../protocol-artifact/game-information-lookup-binding-v1/conformance/fixtures/game-information-lookup-binding-v1/invalid/unknown-version.json"
        ),
    ),
    (
        "LBR-INVALID-REOBSERVE-UNAVAILABLE-SHAPE",
        include_str!(
            "../../../protocol-artifact/game-information-lookup-binding-v1/conformance/fixtures/game-information-lookup-binding-v1/invalid/reobserve-unavailable-shape.json"
        ),
    ),
    (
        "LBR-INVALID-EXHAUSTED-WITH-OBSERVATION",
        include_str!(
            "../../../protocol-artifact/game-information-lookup-binding-v1/conformance/fixtures/game-information-lookup-binding-v1/invalid/exhausted-with-observation.json"
        ),
    ),
    (
        "LBR-INVALID-STATE-SHAPE",
        include_str!(
            "../../../protocol-artifact/game-information-lookup-binding-v1/conformance/fixtures/game-information-lookup-binding-v1/invalid/state-shape.json"
        ),
    ),
    (
        "LBR-INVALID-DUPLICATE-KEY",
        include_str!(
            "../../../protocol-artifact/game-information-lookup-binding-v1/conformance/fixtures/game-information-lookup-binding-v1/invalid/duplicate-key.json"
        ),
    ),
];

pub(super) fn member<'a>(value: &'a JsonValue, name: &str) -> Result<&'a JsonValue, String> {
    value
        .as_object()
        .and_then(|object| object.get(name))
        .ok_or_else(|| format!("missing {name}"))
}

fn nested<'a>(value: &'a JsonValue, path: &[&str]) -> Result<&'a JsonValue, String> {
    let mut current = value;
    for name in path {
        current = member(current, name)?;
    }
    Ok(current)
}

pub(super) fn string(value: &JsonValue, name: &str) -> Result<String, String> {
    member(value, name)?
        .as_string()
        .map(String::from)
        .ok_or_else(|| format!("{name} is not a string"))
}

pub(super) fn vector_call(
    body: &JsonValue,
    fixture_context: Option<&JsonValue>,
    operation_override: Option<&str>,
) -> Result<(GameInformationContext, JsonValue), String> {
    let binding = member(body, "binding").ok();
    let scope = match fixture_context {
        Some(context) => member(context, "scope")?,
        None => nested(
            binding.ok_or_else(|| String::from("binding is missing"))?,
            &["scope"],
        )?,
    };
    let instance_id = match fixture_context {
        Some(context) => string(context, "instance_id")?,
        None => string(
            binding.ok_or_else(|| String::from("binding is missing"))?,
            "instance_id",
        )?,
    };
    let authority_epoch = binding
        .and_then(|binding| member(binding, "authority_epoch").ok())
        .cloned()
        .unwrap_or(JsonValue::Number(7));
    let operation = operation_override.map(String::from).unwrap_or_else(|| {
        match string(body, "kind").as_deref() {
            Ok("lookup_binding_discovery_response") => String::from("discovery"),
            _ => String::from("observe"),
        }
    });
    let correlation_id = string(body, "correlation_id")?;
    let request_id = RequestId::String(correlation_id.clone());
    let context = GameInformationContext {
        instance_id,
        mcp_session_id: String::from("mcp-session-1"),
        gateway_session_id: String::from("gateway-session-1"),
        lease_id: String::from("lease-1"),
        lease_epoch: 9,
        correlation_id,
        request_id,
    };
    let request = JsonValue::object([
        (String::from("operation"), JsonValue::string(operation)),
        (
            String::from("project_id"),
            member(scope, "project_id")?.clone(),
        ),
        (String::from("run_id"), member(scope, "run_id")?.clone()),
        (
            String::from("episode_id"),
            member(scope, "episode_id")?.clone(),
        ),
        (String::from("agent_id"), member(scope, "agent_id")?.clone()),
        (String::from("authority_epoch"), authority_epoch),
    ]);
    Ok((context, request))
}

pub(super) fn indexed_ids(index: &JsonValue, name: &str) -> Result<Vec<String>, String> {
    member(index, name)?
        .as_array()
        .ok_or_else(|| format!("{name} is not an array"))?
        .iter()
        .map(|value| {
            value
                .as_string()
                .map(String::from)
                .ok_or_else(|| format!("{name} contains a non-string id"))
        })
        .collect()
}

pub(super) fn descriptor_ids(index: &JsonValue, name: &str) -> Result<Vec<String>, String> {
    member(index, name)?
        .as_array()
        .ok_or_else(|| format!("{name} is not an array"))?
        .iter()
        .map(|descriptor| string(descriptor, "id"))
        .collect()
}
