# Coding standards

## Current phase

This target is an early Rust workspace. It contains a small no-I/O MCP seam and the target-local
repo-policy tool. Do not add an empty product crate, a generic common crate, or a dependency merely to
make a workspace command green.

## Rust and module boundaries

- Use the pinned Rust `1.97.1` toolchain and edition 2024.
- Keep MCP framing, gateway mapping, schemas, validation, and diagnostics in focused modules. The current
  seam uses an in-tree bounded JSON representation and has no external runtime dependency.
- Keep domain meaning and host behavior outside the adapter; consume only declared boundary contracts.
- Use explicit types for MCP sessions, gateway sessions, game instances, requests, operations, and
  correlation IDs. Never merge namespaces because two fields are both named `id`.
- Use structured `Result` errors and map each error once at its boundary. Do not leak debug strings,
  paths, panic text, credentials, or raw untrusted downstream content.
- Bound request bodies, response content, log fields, retries, polling, and diagnostic output.
- Keep ownership, cancellation, queue capacity, lock ordering, and shutdown behavior explicit.

## Safety and provenance

Unsafe Rust is forbidden in this target. A future exception would need a separate reviewed boundary
decision and narrow safety invariants. Do not use `unwrap`, `expect`, `panic!`, `todo!`, or
`unimplemented!` in production paths. Do not ignore `Result` values. Prefer a conservative explicit
rejection when the contract is incomplete.

All source and documentation must be original or carry recorded redistribution rights. Do not copy,
vendor, transliterate, or use reference implementation symbols as a product plan. Proprietary host
assemblies, saves, credentials, personal paths, and generated output remain outside the repository.

## Size and review budgets

The policy checker counts nonblank physical lines, including comments:

| Artifact | Preferred | Hard |
| --- | ---: | ---: |
| Production Rust | 300 | 400 |
| Rust tests | 400 | 600 |
| Managed C# | 250 | 350 |
| Managed C# tests | 350 | 500 |
| Workflow | 160 | 200 |
| Markdown | 500 | 700 |

Functions should normally stay at or below 40 lines. Split by responsibility when a module or function
exceeds its preferred budget. Exemptions are exact, justified, and recorded in `policy.toml`; they are
not a way to retain copied source.

## Review expectations

Public MCP or gateway contract changes require a decision, version classification, exact serialization
fixtures, mapping tests, and compatibility notes. The local wave2-local-v0 catalog is a preparation
profile and must not be presented as a frozen external protocol. Security, lifecycle, timeout,
cancellation, or concurrency changes require deterministic negative and recovery tests. Documentation must distinguish
`confirmed`, `source-derived`, `inferred`, `proposed`, `unsupported`, and `unverified` evidence.

## Aggregate naming authority

Use the aggregate NAMING_CONVENTIONS.md and naming-registry.yaml for casing,
identity namespaces, lifecycle vocabulary, evidence states, and protected MCP/JSON-RPC spellings.
MCP standard members remain exact; an MCP request or session is never renamed into a gateway or host
identity merely because the adapter maps it.
