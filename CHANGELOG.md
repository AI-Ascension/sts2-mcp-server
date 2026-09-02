# Changelog

All notable changes are recorded here. The project follows Semantic Versioning once a release contract
exists.

## Unreleased

- Added the offline `sts2-protocol/poc-v1` artifact copy and exactly two MCP tools, `get_state` and
  `submit_action`, with deterministic fixed GET/POST gateway mapping tests. No live transport or
  runtime claim is added.
- Added target-local repository governance, policy tooling, workflow guards, and tailored architecture
  documentation for the external MCP-to-gateway boundary.
- Added a non-live Rust MCP framing, capability/catalog, and gateway-adapter mapping seam with a
  deterministic fake-gateway test suite.
- Added no live listener, cross-repository dependency, game integration, provider call, or release
  artifact.
