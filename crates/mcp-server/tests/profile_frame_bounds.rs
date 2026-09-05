// SPDX-License-Identifier: MIT

//! The MCP frame limit is a profile property: the poc, runtime-v1, and runtime-v2
//! profiles keep their historical 16 KiB limit, and only the Runtime-v3 semantic
//! profile accepts frames up to the 256 KiB ceiling. Oversized frames are rejected
//! before any gateway access.

use sts2_mcp_server::{
    GatewayAdapter, GatewayError, GatewayRequest, GatewayResponse, MAX_FRAME_BYTES, McpServer,
    ToolCatalog,
};

const LEGACY_MAX_FRAME_BYTES: usize = 16 * 1024;

struct CountingGateway {
    requests: usize,
}

impl GatewayAdapter for CountingGateway {
    fn forward(&mut self, _request: GatewayRequest) -> Result<GatewayResponse, GatewayError> {
        self.requests += 1;
        Err(GatewayError::Unavailable)
    }
}

/// A `tools/list` frame padded to exactly `total_bytes` with an ignored parameter.
fn padded_frame(total_bytes: usize) -> String {
    let prefix = "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\",\"params\":{\"pad\":\"";
    let suffix = "\"}}";
    let frame = format!(
        "{prefix}{}{suffix}",
        "x".repeat(total_bytes - prefix.len() - suffix.len())
    );
    assert_eq!(frame.len(), total_bytes);
    frame
}

fn profiles() -> [(&'static str, ToolCatalog, usize); 4] {
    [
        ("poc-v1-mcp", ToolCatalog::default(), LEGACY_MAX_FRAME_BYTES),
        (
            "runtime-v1-mcp",
            ToolCatalog::runtime_v1(),
            LEGACY_MAX_FRAME_BYTES,
        ),
        (
            "runtime-v2-mcp",
            ToolCatalog::runtime_v2(),
            LEGACY_MAX_FRAME_BYTES,
        ),
        (
            "runtime-v3-gameplay-mcp",
            ToolCatalog::runtime_v3_gameplay(),
            MAX_FRAME_BYTES,
        ),
    ]
}

#[test]
fn frame_limits_are_profile_scoped() {
    assert_eq!(MAX_FRAME_BYTES, 256 * 1024);
    for (revision, catalog, limit) in profiles() {
        assert_eq!(catalog.revision, revision);
        assert_eq!(catalog.max_frame_bytes(), limit, "{revision}");
    }
}

#[test]
fn every_profile_accepts_a_frame_at_its_limit_and_rejects_one_byte_more() {
    for (revision, catalog, limit) in profiles() {
        let mut server = McpServer::with_catalog(CountingGateway { requests: 0 }, catalog);

        let accepted = server.handle_frame(&padded_frame(limit));
        assert!(
            accepted.contains("\"tools\"") && accepted.contains(revision),
            "{revision} at {limit} bytes: {accepted}"
        );

        let rejected = server.handle_frame(&padded_frame(limit + 1));
        assert!(
            rejected.contains("\"code\":-32700")
                && rejected.contains("MCP frame exceeds the byte limit")
                && rejected.contains("\"id\":null"),
            "{revision} at {} bytes: {rejected}",
            limit + 1
        );
        assert_eq!(server.gateway().requests, 0, "{revision}");
    }
}

#[test]
fn legacy_profiles_reject_frames_the_semantic_profile_accepts() {
    // A frame between the legacy limit and the ceiling proves the limit is per profile,
    // not a single global value.
    let frame = padded_frame(LEGACY_MAX_FRAME_BYTES + 1);
    for catalog in [
        ToolCatalog::default(),
        ToolCatalog::runtime_v1(),
        ToolCatalog::runtime_v2(),
    ] {
        let revision = catalog.revision.clone();
        let mut server = McpServer::with_catalog(CountingGateway { requests: 0 }, catalog);
        let response = server.handle_frame(&frame);
        assert!(
            response.contains("\"code\":-32700"),
            "{revision}: {response}"
        );
        assert_eq!(server.gateway().requests, 0, "{revision}");
    }
    let mut server = McpServer::with_catalog(
        CountingGateway { requests: 0 },
        ToolCatalog::runtime_v3_gameplay(),
    );
    let response = server.handle_frame(&frame);
    assert!(
        response.contains("\"tools\"") && !response.contains("-32700"),
        "{response}"
    );
    assert_eq!(server.gateway().requests, 0);
}
