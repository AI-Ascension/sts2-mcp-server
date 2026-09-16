// SPDX-License-Identifier: MIT

use super::vector_data::{INVALID_FIXTURES, VALID_GOLDENS, member, string, vector_call};
use super::*;

#[test]
fn valid_lbr_response_goldens_pass_the_mcp_projection_boundary() -> Result<(), String> {
    for (id, source) in VALID_GOLDENS {
        let body = parse_json(source)?;
        let (context, request) = vector_call(&body, None, None)?;
        let (_, is_error) = super::response::project_binding(&body, &context, &request)
            .map_err(|error| format!("{id} did not pass the MCP response boundary: {error}"))?;
        if is_error != (id == "LBR-VALID-REOBSERVE-UNAVAILABLE") {
            return Err(format!("{id} had the wrong projected error state"));
        }
    }
    Ok(())
}

#[test]
fn mcp_rejects_shared_static_invalid_response_vectors() -> Result<(), String> {
    // Retained-generation, negotiated-capability, and retained-observation checks belong to the
    // harness session. This adapter enforces the shared schema plus request scope/instance identity.
    let rejected = [
        "LBR-INVALID-WRONG-SCOPE",
        "LBR-INVALID-WRONG-INSTANCE",
        "LBR-INVALID-FORGED-BINDING-ID",
        "LBR-INVALID-MIXED-BINDING",
        "LBR-INVALID-UNKNOWN-VERSION",
        "LBR-INVALID-REOBSERVE-UNAVAILABLE-SHAPE",
        "LBR-INVALID-EXHAUSTED-WITH-OBSERVATION",
        "LBR-INVALID-STATE-SHAPE",
    ];
    for (id, source) in INVALID_FIXTURES {
        if id == "LBR-INVALID-DUPLICATE-KEY" {
            let fixture = parse_json(source)?;
            let raw = string(&fixture, "raw")?;
            if parse_json(&raw).is_ok() {
                return Err(String::from(
                    "shared duplicate-key fixture passed the production JSON parser",
                ));
            }
            continue;
        }
        if !rejected.contains(&id) {
            continue;
        }
        let fixture = parse_json(source)?;
        let body = member(&fixture, "document")?;
        let fixture_context = member(&fixture, "context").ok();
        let operation = (id == "LBR-INVALID-REOBSERVE-UNAVAILABLE-SHAPE").then_some("discovery");
        let (context, request) = vector_call(body, fixture_context, operation)?;
        if super::response::project_binding(body, &context, &request).is_ok() {
            return Err(format!("{id} passed the MCP response boundary"));
        }
    }
    Ok(())
}

#[test]
fn canonical_binding_identity_vectors_match_the_mcp_digest_calculation() -> Result<(), String> {
    for source in [
        include_str!(
            "../../../protocol-artifact/game-information-lookup-binding-v1/conformance/fixtures/game-information-lookup-binding-v1/valid/binding-identity-input.json"
        ),
        include_str!(
            "../../../protocol-artifact/game-information-lookup-binding-v1/conformance/fixtures/game-information-lookup-binding-v1/valid/binding-identity-epoch.json"
        ),
    ] {
        let vector = parse_json(source)?;
        let identity = member(&vector, "identity_input")?;
        let identity_fields = identity
            .as_object()
            .ok_or_else(|| String::from("identity_input is not an object"))?;
        let scope = JsonValue::object(
            ["agent_id", "episode_id", "project_id", "run_id"]
                .into_iter()
                .map(|key| (String::from(key), identity_fields[key].clone())),
        );
        let binding = JsonValue::object([
            (String::from("scope"), scope),
            (
                String::from("authority_epoch"),
                identity_fields["authority_epoch"].clone(),
            ),
            (
                String::from("content_manifest_id"),
                identity_fields["content_manifest_id"].clone(),
            ),
            (
                String::from("game_profile"),
                identity_fields["game_profile"].clone(),
            ),
            (String::from("locale"), identity_fields["locale"].clone()),
        ]);
        let actual = super::response::binding_validation::canonical_binding_id(
            binding
                .as_object()
                .ok_or_else(|| String::from("canonical binding is not an object"))?,
        )
        .map_err(String::from)?;
        if actual != string(&vector, "binding_id")? {
            return Err(format!(
                "{} digest did not match the shared vector",
                string(&vector, "id")?
            ));
        }
    }
    Ok(())
}

#[test]
fn response_correlation_kind_and_terminal_error_semantics_are_enforced() -> Result<(), String> {
    let body = parse_json(VALID_GOLDENS[0].1)?;
    let (context, request) = vector_call(&body, None, None)?;
    let mut wrong_correlation = body.clone();
    set_member(
        &mut wrong_correlation,
        "correlation_id",
        JsonValue::string("corr:wrong"),
    )?;
    if super::response::project_binding(&wrong_correlation, &context, &request).is_ok() {
        return Err(String::from("wrong response correlation was accepted"));
    }
    let mut wrong_kind = body.clone();
    set_member(
        &mut wrong_kind,
        "kind",
        JsonValue::string("lookup_binding_observation_response"),
    )?;
    if super::response::project_binding(&wrong_kind, &context, &request).is_ok() {
        return Err(String::from("response kind did not match the request"));
    }
    let mut unknown_version = body.clone();
    set_member(
        &mut unknown_version,
        "protocol_version",
        JsonValue::string("game-information-lookup-binding-v2"),
    )?;
    let version_error = super::response::project_binding(&unknown_version, &context, &request)
        .err()
        .ok_or_else(|| String::from("unknown response version was accepted"))?;
    if super::response::projection_error_code(version_error)
        != ("game_information_unsupported_version", "unsupported")
    {
        return Err(String::from(
            "unknown response version was not projected as unsupported",
        ));
    }
    let unavailable = parse_json(VALID_GOLDENS[4].1)?;
    let (context, request) = vector_call(&unavailable, None, Some("observe"))?;
    if !super::response::project_binding(&unavailable, &context, &request)?.1 {
        return Err(String::from(
            "terminal reobserve_unavailable was not preserved as an MCP error",
        ));
    }
    if super::response::protocol_error_category("reobserve_unavailable") != "stale"
        || super::response::protocol_error_category("mixed_binding") != "stale"
        || super::response::protocol_error_category("denied_scope") != "denied"
    {
        return Err(String::from("binding protocol error categories changed"));
    }
    let mut mislabeled_error = unavailable.clone();
    nested_mut(&mut mislabeled_error, &["error", "code"])?
        .clone_from(&JsonValue::string("missing_capability"));
    if super::response::project_binding(&mislabeled_error, &context, &request).is_ok() {
        return Err(String::from(
            "a generic error carried terminal binding state",
        ));
    }
    Ok(())
}

fn nested_mut<'a>(value: &'a mut JsonValue, path: &[&str]) -> Result<&'a mut JsonValue, String> {
    let mut current = value;
    for name in path {
        current = match current {
            JsonValue::Object(object) => object.get_mut(*name),
            _ => None,
        }
        .ok_or_else(|| format!("missing mutable member {name}"))?;
    }
    Ok(current)
}

fn set_member(value: &mut JsonValue, name: &str, member: JsonValue) -> Result<(), String> {
    match value {
        JsonValue::Object(object) => {
            object.insert(String::from(name), member);
            Ok(())
        }
        _ => Err(String::from("golden response is not an object")),
    }
}
