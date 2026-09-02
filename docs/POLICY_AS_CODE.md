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

The command is read-only. It returns nonzero when required files are missing, a mandatory rule fails, or
strict mode promotes a preferred-budget warning to an error.

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

The checker skips ignored build/editor/vendor directories and symlinks. It checks the initialized MCP
crate as ordinary Rust source but does not prove runtime
behavior, MCP conformance, gateway authorization, dependency graph ownership, host compatibility,
secrets absence outside scanned files, or release readiness. Those require future target-owned tests and
review.

## Changes to policy

A policy change must explain the rule, enforcement effect, migration impact, and exact validation. Do not
weaken a threshold or add an exemption just to make an unrelated change pass. Keep checker output bounded,
repository-relative, deterministic, and free of credentials or private payloads.
