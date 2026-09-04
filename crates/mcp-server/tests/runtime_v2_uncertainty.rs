// SPDX-License-Identifier: MIT

#[path = "runtime_v2_support/mod.rs"]
#[allow(dead_code)]
mod support;

use sts2_mcp_server::{GatewayError, GatewayResponse, JsonValue, McpServer, ToolCatalog};
use support::{RecordingGateway, state_call, submit_call};

#[test]
fn failed_state_read_is_not_fabricated_as_an_operation_outcome() {
    for failure in [
        GatewayError::Timeout,
        GatewayError::Unavailable,
        GatewayError::MalformedResponse,
    ] {
        let mut server = McpServer::with_catalog(
            RecordingGateway::new([Err(failure)]),
            ToolCatalog::runtime_v2(),
        );
        let response = server.handle_frame(&state_call(
            "state-1",
            "instance-1",
            "session-1",
            "lease-1",
            1,
            4,
        ));
        assert!(response.contains("gateway state read unavailable"));
        assert!(response.contains("\"isError\":true"));
        assert!(!response.contains("operation_id"));
        assert!(!response.contains("end_turn"));
        assert_eq!(server.gateway().requests.len(), 1);
    }
}

#[test]
fn malformed_end_turn_response_remains_unknown_without_retry() {
    let mut server = McpServer::with_catalog(
        RecordingGateway::new([Err(GatewayError::MalformedResponse)]),
        ToolCatalog::runtime_v2(),
    );
    let response = server.handle_frame(&submit_call(
        "request-1",
        "instance-1",
        "session-1",
        "lease-1",
        1,
        4,
        "op-1",
    ));
    assert!(response.contains("\"isError\":true"));
    assert!(response.contains("status\\\":\\\"unknown"), "{response}");
    assert!(response.contains("op-1"));
    assert_eq!(server.gateway().requests.len(), 1);
}

#[test]
fn operations_that_cannot_be_addressed_by_the_read_route_are_never_dispatched() {
    let mut server = McpServer::with_catalog(RecordingGateway::new([]), ToolCatalog::runtime_v2());
    let response = server.handle_frame(&submit_call(
        "request-1",
        "instance-1",
        "session-1",
        "lease-1",
        1,
        4,
        "namespace/op-1",
    ));
    assert!(response.contains("unsafe or oversized"));
    assert!(server.gateway().requests.is_empty());
}

#[test]
fn invalid_end_turn_envelope_is_unknown_and_does_not_leak_payload() {
    let mut server = McpServer::with_catalog(
        RecordingGateway::new([Ok(GatewayResponse {
            status: 200,
            body: JsonValue::string("private-untrusted-payload"),
        })]),
        ToolCatalog::runtime_v2(),
    );
    let response = server.handle_frame(&submit_call(
        "request-1",
        "instance-1",
        "session-1",
        "lease-1",
        1,
        4,
        "op-1",
    ));
    assert!(
        response.contains("unknown_after_invalid_response"),
        "{response}"
    );
    assert!(!response.contains("private-untrusted-payload"));
    assert_eq!(server.gateway().requests.len(), 1);
}
