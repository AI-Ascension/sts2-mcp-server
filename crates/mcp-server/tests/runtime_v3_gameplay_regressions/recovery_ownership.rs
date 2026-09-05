// SPDX-License-Identifier: MIT

//! `sts2.recover` exposes the lifecycle recovery vocabulary (`release_lease`,
//! `stop_episode`) but does not own it: every recovery kind is forwarded to the
//! one fixed gateway recover route with no other side effect, and the gateway
//! decides (it authorizes recovery with the `control` scope).

use super::*;

fn recover_call(recovery_kind: &str) -> String {
    call(
        RECOVER_TOOL,
        &context_arguments(&format!(",\"recovery_kind\":\"{recovery_kind}\"")),
    )
}

fn recover_response() -> JsonValue {
    let mut body = settled_response("recover_response", 4, 5, "combat-1");
    if let JsonValue::Object(fields) = &mut body {
        fields.insert(String::from("operation_id"), JsonValue::string("request-1"));
    }
    body
}

fn body_of(request: &GatewayRequest) -> Result<serde_json::Value, String> {
    let body = request.body.as_ref().ok_or("missing request body")?;
    serde_json::from_str(&body.to_json()).map_err(|error| error.to_string())
}

#[test]
fn lifecycle_recovery_kinds_map_only_to_the_fixed_recover_route() -> Result<(), String> {
    for kind in ["release_lease", "stop_episode"] {
        let mut server = McpServer::with_catalog(
            RecordingGateway::new([Ok(GatewayResponse {
                status: 200,
                body: recover_response(),
            })]),
            ToolCatalog::runtime_v3_gameplay(),
        );
        let output = server.handle_frame(&recover_call(kind));
        assert!(output.contains("\"isError\":false"), "{kind}: {output}");

        // Exactly one gateway request, to the one fixed recover route, and nothing else.
        let requests = &server.gateway().requests;
        assert_eq!(requests.len(), 1, "{kind}");
        let request = &requests[0];
        assert_eq!(request.method, GatewayMethod::Post, "{kind}");
        assert_eq!(request.path, "/v3/instances/instance-1/recover", "{kind}");
        assert!(
            !request.path.contains("lease") && !request.path.contains("episode"),
            "{kind}: no lifecycle route is constructed: {}",
            request.path
        );

        // The envelope names the recovery kind and carries no action, state, or
        // operation identity; MCP adds no lifecycle authority of its own.
        let value = body_of(request)?;
        assert_eq!(value["kind"], "recover_request", "{kind}");
        assert_eq!(value["recovery"]["kind"], kind);
        assert_eq!(value["recovery"]["operation_id"], serde_json::Value::Null);
        assert_eq!(value["action"], serde_json::Value::Null, "{kind}");
        assert_eq!(value["operation_id"], serde_json::Value::Null, "{kind}");
        assert_eq!(value["state_id"], serde_json::Value::Null, "{kind}");
        assert_eq!(value["instance_id"], "instance-1");
        assert_eq!(value["lease_id"], "lease-1");
        assert_eq!(value["lease_epoch"], 1);
    }
    Ok(())
}

#[test]
fn lifecycle_recovery_kinds_reject_operation_identity_before_forwarding() {
    for kind in ["release_lease", "stop_episode"] {
        let mut server = McpServer::with_catalog(
            RecordingGateway::new([]),
            ToolCatalog::runtime_v3_gameplay(),
        );
        let output = server.handle_frame(&call(
            RECOVER_TOOL,
            &context_arguments(&format!(
                ",\"recovery_kind\":\"{kind}\",\"operation_id\":\"operation-1\""
            )),
        ));
        assert!(output.contains("-32602"), "{kind}: {output}");
        assert!(server.gateway().requests.is_empty(), "{kind}");
    }
}

#[test]
fn gateway_scope_denial_of_lifecycle_recovery_is_a_typed_error_not_an_unknown_outcome() {
    for kind in ["release_lease", "stop_episode"] {
        let mut server = McpServer::with_catalog(
            RecordingGateway::new([Err(GatewayError::Forbidden)]),
            ToolCatalog::runtime_v3_gameplay(),
        );
        let output = server.handle_frame(&recover_call(kind));
        assert_eq!(server.gateway().requests.len(), 1, "{kind}");
        assert!(output.contains("-32007"), "{kind}: {output}");
        assert!(!output.contains("unknown"), "{kind}: {output}");
    }
}
