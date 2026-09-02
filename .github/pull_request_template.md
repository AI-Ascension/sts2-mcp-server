## Summary

<!-- State the boundary or foundation change and why it is needed. -->

## Contract and ownership

- [ ] This change stays within external MCP framing and gateway mapping.
- [ ] No game-host, game-rule, gateway-lifecycle, model, provider, or harness authority was added.
- [ ] Any public contract or dependency-direction change has an ADR and focused tests/fixtures.

## Provenance and security

- [ ] New code and documentation are original or have recorded redistribution rights.
- [ ] No proprietary game files, saves, credentials, personal paths, or generated output are included.
- [ ] Inputs and downstream responses remain bounded and sensitive values are redacted.

## Validation

- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
- [ ] `cargo test --workspace --all-targets --all-features --locked`
- [ ] `cargo run --locked --package repo-policy -- --strict`

## Runtime and release status

<!-- Distinguish statically derived/build evidence from runtime or release evidence. -->

- Runtime evidence: <!-- confirmed / unverified / not applicable, with details -->
- Release or deployment impact: <!-- none / describe separately -->
