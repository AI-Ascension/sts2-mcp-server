// SPDX-License-Identifier: MIT

use sts2_mcp_server::{
    DISPATCH_ACTION_TOOL, GatewayAdapter, GatewayError, GatewayRequest, GatewayResponse,
    LEGAL_ACTIONS_TOOL, McpServer, OBSERVE_TOOL, RECOVER_TOOL, REOBSERVE_TOOL, ToolCatalog,
    WAIT_FOR_TRANSITION_TOOL,
};

struct UnavailableGateway;

impl GatewayAdapter for UnavailableGateway {
    fn forward(&mut self, _request: GatewayRequest) -> Result<GatewayResponse, GatewayError> {
        Err(GatewayError::Unavailable)
    }
}

#[test]
fn exo_catalog_is_exactly_the_six_semantic_runtime_tools() {
    let mut server =
        McpServer::with_catalog(UnavailableGateway, ToolCatalog::runtime_v3_gameplay());
    let response = server
        .handle_frame("{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\",\"params\":{}}");
    for name in [
        OBSERVE_TOOL,
        LEGAL_ACTIONS_TOOL,
        DISPATCH_ACTION_TOOL,
        WAIT_FOR_TRANSITION_TOOL,
        REOBSERVE_TOOL,
        RECOVER_TOOL,
    ] {
        assert!(response.contains(name), "catalog is missing {name}");
    }
    assert_eq!(response.matches("\"name\"").count(), 6);
    assert!(!response.contains("shell"));
    assert!(!response.contains("raw_memory"));
}
