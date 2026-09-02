# Contributing

## Before proposing a change

Read [`AGENTS.md`](AGENTS.md), [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md), and the relevant testing,
compatibility, security, licensing, and workflow documents. State the observable problem, the owner of
the affected contract, the evidence level, and the smallest safe validation plan.

This target currently contains foundation tooling only. Do not add product behavior, a placeholder
product crate, a direct game connection, a gateway registry, model/provider logic, or copied reference
source. New shared contracts belong in the accepted protocol target only when its owner, consumers,
version, language neutrality, provenance, and conformance oracle are explicit.

## Implementation expectations

- Keep MCP framing and content mapping separate from gateway transport details and domain meaning.
- Use an explicit, versioned gateway route/method/header/body allowlist; never implement an open proxy.
- Bind each request to the authorized MCP session and gateway target without conflating identifier
  namespaces.
- Bound input, output, logs, retries, polling, and diagnostics. Redact credentials and untrusted content.
- Preserve accepted work semantics and distinguish acknowledgement from downstream completion.
- Add focused serialization, mapping, malformed-input, error, timeout, cancellation, and fake-gateway
  tests when product behavior exists.

## Validation

Run from the target root:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
cargo run --locked --package repo-policy -- --strict
```

If an external runtime, gateway, host, provider, credential, or proprietary artifact is unavailable,
report the exact safe probe and mark that evidence `unverified`; do not replace it with a simulated green
result. Keep temporary outputs outside the source tree or in ignored paths.

## Pull requests

Describe ownership, contract effects, compatibility classification, provenance, security/data impact,
exact commands and results, and remaining unverified lanes. Keep changes focused. A green check does not
authorize merge, release, deployment, or game mutation.
