# Policy as code

## Purpose

Written guidance is not enforcement by itself. This target keeps objective foundation rules in
`policy.toml` and checks them with the Rust-only target-local `repo-policy` tool. The tool is governance
infrastructure, not MCP product behavior.

## Local entrypoint

Run from the target root:

```bash
cargo run --locked --package repo-policy -- --strict
```

The command is read-only. Version 2 returns nonzero when required files or mandatory rules fail; the
preferred `SIZE001` warning stays advisory under strict mode. Version 1 remains supported for legacy
callers and its strict mode promotes every warning, so changing policy versions is an explicit
migration.

## Enforced rule families

| Rule | Meaning |
| --- | --- |
| `CFG001` | policy exists, parses, and uses the supported version |
| `DOC001` | required foundation files exist |
| `DOC002` | local Markdown links resolve |
| `SIZE001` | source, workflow, and Markdown budgets are respected |
| `EXC001` | exemptions are exact paths with durable reasons |
| `WF001-005` | permissions, trust triggers, immutable action pins, and visible failures |
| `RUST001` | workspace lockfile, toolchain, package metadata, and lint policy agree |
| `LANG001` | Python source and package metadata are prohibited |
| `LIC001-003` | MIT root/license declarations and source SPDX headers exist |

The checker skips ignored build/editor/vendor directories and symlinks. The generated-output `bin`
ignore does not exclude Rust `src/bin` entrypoints or their nested support modules. Files named
`*_test.rs` or `*_tests.rs` use test budgets even within `src/bin`; ordinary binary modules use the
production budgets. Disposable-tree regressions verify collection and SPDX, language, and size checks
for these executable sources while generated-output exclusions remain effective.
It checks the initialized MCP
crate as ordinary Rust source but does not prove runtime
behavior, MCP conformance, gateway authorization, dependency graph ownership, host compatibility,
secrets absence outside scanned files, or release readiness. Those require future target-owned tests and
review.

## Changes to policy

A policy change must explain the rule, enforcement effect, migration impact, and exact validation. Do not
weaken a threshold or add an exemption just to make an unrelated change pass. Keep checker output bounded,
repository-relative, deterministic, and free of credentials or private payloads.

## Production lint scope

The production Clippy lane selects workspace libraries and binaries and forbids
unwrap, expect, panic, todo and unimplemented on the compiler command line.
A source-level allowance cannot override that lane. The existing all-target lane
still checks tests with their scoped allowances. `production_lints` runs real
compiler fixtures for forbidden constructs, an attempted blanket allowance,
and valid comments/test-only code. Missing Clippy or an unrelated compiler
failure cannot satisfy a negative case: its diagnostic must identify the rule.
