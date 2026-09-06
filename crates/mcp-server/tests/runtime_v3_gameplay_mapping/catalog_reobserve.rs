// SPDX-License-Identifier: MIT

use super::*;

#[test]
fn catalog_reobserve_preserves_only_the_correlated_read_error_contract() {
    for (status, code) in [
        (409, "stale_generation"),
        (503, "host_not_configured"),
        (503, "host_observation_unavailable"),
    ] {
        let body = catalog_refusal(code, "request-1");
        let gateway = RecordingGateway::new([Ok(GatewayResponse { status, body })]);
        let mut server = McpServer::with_catalog(gateway, ToolCatalog::runtime_v3_gameplay());
        let result = server.handle_frame(&call(
            LEGAL_ACTIONS_TOOL,
            &context_arguments(",\"state_id\":\"combat-1\""),
        ));
        assert!(result.contains("\"isError\":true"));
        assert!(result.contains("reobserve"));
        assert!(result.contains(code));
    }
    for (status, code, correlation, tool, extra) in [
        (
            200,
            "stale_generation",
            "request-1",
            LEGAL_ACTIONS_TOOL,
            false,
        ),
        (
            503,
            "stale_generation",
            "request-1",
            LEGAL_ACTIONS_TOOL,
            false,
        ),
        (409, "secret-marker", "request-1", LEGAL_ACTIONS_TOOL, false),
        (409, "stale_generation", "wrong", LEGAL_ACTIONS_TOOL, false),
        (
            409,
            "stale_generation",
            "request-1",
            LEGAL_ACTIONS_TOOL,
            true,
        ),
        (409, "stale_generation", "request-1", OBSERVE_TOOL, false),
    ] {
        let mut body = catalog_refusal(code, correlation);
        if extra {
            assert!(matches!(&body, JsonValue::Object(_)));
            if let JsonValue::Object(ref mut fields) = body {
                fields.insert(String::from("private"), JsonValue::string("secret-marker"));
            }
        }
        let gateway = RecordingGateway::new([Ok(GatewayResponse { status, body })]);
        let mut server = McpServer::with_catalog(gateway, ToolCatalog::runtime_v3_gameplay());
        let arguments = context_arguments(if tool == LEGAL_ACTIONS_TOOL {
            ",\"state_id\":\"combat-1\""
        } else {
            ""
        });
        let result = server.handle_frame(&call(tool, &arguments));
        assert!(result.contains("\"isError\":true"));
        assert!(!result.contains("reobserve"));
        assert!(!result.contains("secret-marker"));
    }
}

fn catalog_refusal(code: &str, correlation: &str) -> JsonValue {
    JsonValue::object([
        (String::from("error_code"), JsonValue::string(code)),
        (
            String::from("correlation_id"),
            JsonValue::string(correlation),
        ),
        (String::from("recovery"), JsonValue::string("reobserve")),
    ])
}
