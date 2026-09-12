# `exact-checkpoint-reference-v1` protocol artifact (consumed copy)

MCP-consumed copy of the `sts2-protocol/exact-checkpoint-reference-v1` release-like artifact (schema
digest `028e00d06f9f2b16cb9097f47aedd057e74046a7cb2ba97362978e18029f48ab`). It carries the closed,
digest-free public reference envelope, its goldens, and MCP-local checksums.

`crates/mcp-server/tests/exact_checkpoint_reference_artifact.rs` accepts this copy as the shared
shape for a public checkpoint reference and confirms that a privileged member, an unknown version,
or an unexpected property is rejected. The adapter maps bounded references only; it never receives
or forwards an exact-state, checkpoint, blob, or compatibility digest.
