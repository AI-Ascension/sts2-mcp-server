// SPDX-License-Identifier: MIT
// Nested test module shared by the game-information integration seam.
#![allow(clippy::expect_used, clippy::panic)]

use super::*;
use sts2_mcp_server::{
    CONTENT_MANIFEST_GATEWAY_MAX_RESPONSE_BYTES, CapabilityGroup, CapabilityLayer, CapabilityOwner,
    CapabilityScope, GAME_INFORMATION_CONTENT_MANIFEST_TOOL, verify_content_manifest_artifact,
};

pub(super) fn manifest_golden(name: &str) -> JsonValue {
    let text = match name {
        "canonical" => include_str!(
            "../../../../protocol-artifact/game-information-content-manifest-v1/golden/canonical-manifest-response.json"
        ),
        "refusal" => include_str!(
            "../../../../protocol-artifact/game-information-content-manifest-v1/golden/access-denied-error-response.json"
        ),
        _ => panic!("unsupported content-manifest golden {name}"),
    };
    parse_json(text).expect("accepted content-manifest golden JSON")
}

pub(super) fn member<'a>(body: &'a mut JsonValue, key: &str) -> &'a mut JsonValue {
    let JsonValue::Object(object) = body else {
        panic!("envelope is not an object");
    };
    object.get_mut(key).expect("envelope member")
}

pub(super) fn catalog_member<'a>(body: &'a mut JsonValue, key: &str) -> &'a mut JsonValue {
    let JsonValue::Object(catalog) = member(body, "manifest") else {
        panic!("manifest is not an object");
    };
    catalog.get_mut(key).expect("catalog member")
}

pub(super) fn manifest_server(
    responses: impl IntoIterator<Item = Result<GatewayResponse, GatewayError>>,
) -> McpServer<FakeGateway> {
    McpServer::with_catalog_and_sessions(
        FakeGateway::new(responses),
        ToolCatalog::game_information_query_v1(),
        "gateway-session-1",
        "mcp-session-1",
    )
}

#[test]
fn content_manifest_read_is_bodyless_and_relayed_verbatim() {
    verify_content_manifest_artifact().expect("content-manifest artifact is pinned");
    let mut body = manifest_golden("canonical");
    set_correlation(&mut body, "manifest");
    let expected = body.to_json();
    let mut server = manifest_server([Ok(GatewayResponse { status: 200, body })]);

    let output = wire_value(&server.handle_frame(&frame(
        "manifest",
        GAME_INFORMATION_CONTENT_MANIFEST_TOOL,
        context(),
    )));
    assert_eq!(output["result"]["isError"], false, "{output}");
    assert_eq!(
        output["result"]["content"][0]["text"].as_str(),
        Some(expected.as_str())
    );
    assert_eq!(server.gateway().requests.len(), 1);
    let forwarded = &server.gateway().requests[0];
    assert_eq!(forwarded.method, GatewayMethod::Get);
    assert_eq!(
        forwarded.path,
        "/v1/instances/instance-1/game-information/content-manifest"
    );
    assert!(forwarded.body.is_none());
    for (name, value) in [
        ("x-sts2-instance-id", "instance-1"),
        ("x-sts2-session-id", "gateway-session-1"),
        ("x-sts2-lease-id", "lease-1"),
        ("x-sts2-lease-epoch", "7"),
        ("x-mcp-session-id", "mcp-session-1"),
        ("x-mcp-request-id", "manifest"),
    ] {
        assert_eq!(
            forwarded.headers.get(name).map(String::as_str),
            Some(value),
            "{name}"
        );
    }
}

#[test]
fn content_manifest_refusal_is_a_typed_error_result() {
    let mut body = manifest_golden("refusal");
    set_correlation(&mut body, "denied");
    let expected = body.to_json();
    let mut server = manifest_server([Ok(GatewayResponse { status: 403, body })]);

    let output = wire_value(&server.handle_frame(&frame(
        "denied",
        GAME_INFORMATION_CONTENT_MANIFEST_TOOL,
        context(),
    )));
    assert_eq!(output["result"]["isError"], true, "{output}");
    assert_eq!(
        output["result"]["structuredContent"]["error"]["code"],
        "access_denied"
    );
    assert_eq!(
        output["result"]["structuredContent"]["error"]["category"],
        "denied"
    );
    assert_eq!(
        output["result"]["content"][0]["text"].as_str(),
        Some(expected.as_str())
    );
}

#[test]
fn content_manifest_projection_fails_closed_on_unpinned_answers() {
    let refusal = manifest_golden("refusal");
    let cases = [
        ("foreign", {
            let mut body = manifest_golden("canonical");
            set_correlation(&mut body, "other");
            body
        }),
        ("drifted-digest", {
            let mut body = manifest_golden("canonical");
            *member(&mut body, "schema_digest") = JsonValue::string("a".repeat(64));
            body
        }),
        ("unknown-member", {
            let mut body = manifest_golden("canonical");
            if let JsonValue::Object(object) = &mut body {
                object.insert(String::from("extra"), JsonValue::Bool(true));
            }
            body
        }),
        ("shortened-catalog", {
            let mut body = manifest_golden("canonical");
            if let JsonValue::Object(catalog) = member(&mut body, "manifest") {
                catalog.remove("inventory_revision");
            }
            body
        }),
        ("catalog-carrying-refusal", {
            let mut body = refusal.clone();
            let mut canonical = manifest_golden("canonical");
            *member(&mut body, "manifest") = member(&mut canonical, "manifest").clone();
            body
        }),
        ("error-carrying-read", {
            let mut body = manifest_golden("canonical");
            *member(&mut body, "error") = JsonValue::object([
                (String::from("code"), JsonValue::string("malformed")),
                (String::from("reason"), JsonValue::Null),
            ]);
            body
        }),
        ("unpinned-code", {
            let mut body = refusal.clone();
            *member(&mut body, "error") = JsonValue::object([
                (
                    String::from("code"),
                    JsonValue::string("source_unavailable"),
                ),
                (
                    String::from("reason"),
                    JsonValue::string("source_access_denied"),
                ),
            ]);
            body
        }),
        ("unpinned-reason-pairing", {
            let mut body = refusal.clone();
            *member(&mut body, "error") = JsonValue::object([
                (String::from("code"), JsonValue::string("access_denied")),
                (
                    String::from("reason"),
                    JsonValue::string("source_malformed"),
                ),
            ]);
            body
        }),
        ("unknown-kind", {
            let mut body = manifest_golden("canonical");
            *member(&mut body, "kind") = JsonValue::string("manifest_response");
            body
        }),
        ("catalogless-read", {
            let mut body = manifest_golden("canonical");
            *member(&mut body, "manifest") = JsonValue::Null;
            body
        }),
    ];
    let responses = cases.iter().map(|(id, body)| {
        let mut body = body.clone();
        if *id != "foreign" {
            set_correlation(&mut body, id);
        }
        Ok(GatewayResponse { status: 200, body })
    });
    let mut server = manifest_server(responses);
    for (id, _) in &cases {
        let output = wire_value(&server.handle_frame(&frame(
            id,
            GAME_INFORMATION_CONTENT_MANIFEST_TOOL,
            context(),
        )));
        assert_eq!(output["result"]["isError"], true, "{id}: {output}");
        assert_eq!(
            output["result"]["structuredContent"]["error"]["code"],
            "game_information_malformed_response",
            "{id}: {output}"
        );
    }
    assert_eq!(server.gateway().requests.len(), cases.len());
}

#[test]
fn content_manifest_beyond_the_route_ceiling_is_a_size_error() {
    let mut body = manifest_golden("canonical");
    if let JsonValue::Array(definitions) = catalog_member(&mut body, "definitions") {
        let template = definitions[0].clone();
        for _ in 1..900 {
            definitions.push(template.clone());
        }
    }
    assert!(body.to_json().len() > CONTENT_MANIFEST_GATEWAY_MAX_RESPONSE_BYTES);
    set_correlation(&mut body, "oversized");
    let mut server = manifest_server([Ok(GatewayResponse { status: 200, body })]);

    let output = wire_value(&server.handle_frame(&frame(
        "oversized",
        GAME_INFORMATION_CONTENT_MANIFEST_TOOL,
        context(),
    )));
    assert_eq!(output["result"]["isError"], true, "{output}");
    assert_eq!(
        output["result"]["structuredContent"]["error"]["code"],
        "game_information_response_too_large"
    );
    assert_eq!(
        output["result"]["structuredContent"]["error"]["category"],
        "size"
    );
}

#[test]
fn content_manifest_arguments_are_closed_before_gateway_io() {
    let mut server = manifest_server([]);
    for (id, arguments) in [
        ("extra", {
            let mut value = context();
            if let JsonValue::Object(object) = &mut value {
                object.insert(String::from("content_manifest_id"), JsonValue::string("x"));
            }
            value
        }),
        ("missing-lease", {
            let mut value = context();
            if let JsonValue::Object(object) = &mut value {
                object.remove("lease_epoch");
            }
            value
        }),
    ] {
        let output = server.handle_frame(&frame(
            id,
            GAME_INFORMATION_CONTENT_MANIFEST_TOOL,
            arguments,
        ));
        assert!(output.contains("\"code\":-32602"), "{id}: {output}");
    }
    assert!(server.gateway().requests.is_empty());
}

#[test]
fn content_manifest_read_requires_both_remote_layers_when_composed() {
    let profiles = [ToolCatalog::game_information_query_v1()];
    let local = CapabilityLayer::from_catalogs(CapabilityOwner::Mcp, &profiles).expect("mcp layer");
    let offer = local
        .offer(GAME_INFORMATION_CONTENT_MANIFEST_TOOL)
        .expect("content-manifest offer");
    assert_eq!(offer.revision, "game-information-content-manifest-v1-mcp");
    assert_eq!(offer.group, CapabilityGroup::StaticReference);
    assert_eq!(offer.required_scope, CapabilityScope::READ);

    let gateway =
        CapabilityLayer::from_catalogs(CapabilityOwner::Gateway, &profiles).expect("gateway layer");
    let producer = CapabilityLayer::from_catalogs(CapabilityOwner::Producer, &profiles)
        .expect("producer layer");
    let composed =
        ToolCatalog::compose_profiles(&profiles, gateway, producer, CapabilityScope::READ)
            .expect("composed profile");
    assert!(
        composed
            .tools()
            .iter()
            .any(|tool| tool.name == GAME_INFORMATION_CONTENT_MANIFEST_TOOL)
    );

    // The fixed route carries no negotiated offer today, so a composition without that offer must
    // drop the tool instead of advertising a read the gateway never admitted.
    let no_gateway = ToolCatalog::compose_profiles(
        &profiles,
        CapabilityLayer::new(CapabilityOwner::Gateway, "no-content-manifest-offer"),
        CapabilityLayer::from_catalogs(CapabilityOwner::Producer, &profiles)
            .expect("producer layer"),
        CapabilityScope::READ,
    )
    .expect("composition without the gateway offer");
    assert!(
        !no_gateway
            .tools()
            .iter()
            .any(|tool| tool.name == GAME_INFORMATION_CONTENT_MANIFEST_TOOL)
    );
}
