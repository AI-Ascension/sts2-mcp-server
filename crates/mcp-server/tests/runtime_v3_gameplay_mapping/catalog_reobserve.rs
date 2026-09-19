// SPDX-License-Identifier: MIT

use super::*;

#[test]
fn catalog_reobserve_preserves_only_the_correlated_read_error_contract() {
    for (status, code) in [
        (409, "stale_generation"),
        (503, "host_not_configured"),
        (503, "host_observation_unavailable"),
        // A refused launch contract answers 503 with the mod's refusal code rather than the
        // never-declared code, and it must reach the caller the same way.
        (503, "launch_contract_refused"),
        (503, "launch_contract_refused_isolated_user_dir_mismatch"),
        (503, "launch_contract_refused_campaign_required"),
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
        // A refusal is admitted only on its own status, and only in the shape the mod composes.
        (
            409,
            "launch_contract_refused_isolated_user_dir_mismatch",
            "request-1",
            LEGAL_ACTIONS_TOOL,
            false,
        ),
        (
            200,
            "launch_contract_refused_isolated_user_dir_mismatch",
            "request-1",
            LEGAL_ACTIONS_TOOL,
            false,
        ),
        (
            503,
            "launch_contract_refused_isolated_user_dir_mismatch",
            "request-1",
            OBSERVE_TOOL,
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

/// The admitted refusal shape is the producer's own rule, so a code the mod cannot compose must
/// stay refused here rather than be admitted as a neighbouring string.
#[test]
fn a_refused_launch_contract_code_is_admitted_only_in_the_shape_the_mod_composes() {
    let refusal = |code: &str| GatewayResponse {
        status: 503,
        body: catalog_refusal(code, "request-1"),
    };
    let admitted =
        |code: &str| sts2_mcp_server::catalog_reobserve_body(&refusal(code), "request-1").is_some();
    let longest = format!("launch_contract_refused_{}", "a".repeat(64));
    for code in [
        "launch_contract_refused",
        "launch_contract_refused_a",
        "launch_contract_refused_isolated_user_dir_mismatch",
        "launch_contract_refused_campaign_required",
        "launch_contract_refused_UPPER_lower-123_456",
        // `_` is a legal token character, so a token may begin with one.
        "launch_contract_refused__leading",
        longest.as_str(),
    ] {
        assert!(admitted(code), "the refusal code {code} must be admitted");
    }
    for code in [
        "launch_contract_refused_",
        "launch_contract_refused_..",
        "launch_contract_refused_a b",
        "launch_contract_refused_a.b",
        "launch_contract_refused_a/b",
        "launch_contract_refused_ünicode",
        "launch_contract_refusedx",
        "launch_contractrefused",
        "launch_contract",
        "host_not_configured_refused",
    ] {
        assert!(!admitted(code), "{code} must stay refused");
    }
    // One byte over the reason bound is the first token the producer degrades to the bare prefix,
    // so the neighbouring admitted code is the 64-byte token and not this.
    assert!(!admitted(&format!(
        "launch_contract_refused_{}",
        "a".repeat(65)
    )));
    // The refusal is admitted only on the refusal's own status.
    let code = "launch_contract_refused_isolated_user_dir_mismatch";
    for status in [200, 409, 500, 502] {
        let response = GatewayResponse {
            status,
            body: catalog_refusal(code, "request-1"),
        };
        assert!(
            sts2_mcp_server::catalog_reobserve_body(&response, "request-1").is_none(),
            "status {status} must not admit a refusal"
        );
    }
    assert!(
        sts2_mcp_server::catalog_reobserve_body(&refusal(code), "wrong").is_none(),
        "a refusal must stay correlated"
    );
}
