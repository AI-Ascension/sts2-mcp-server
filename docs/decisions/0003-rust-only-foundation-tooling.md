# ADR 0003: Rust-only foundation tooling

- Status: Accepted for preparation
- Date: 2026-09-02

## Context

The target is intended to become an external Rust process. Introducing Python or a copied governance
tool would add an unowned language/runtime surface during the foundation phase and would conflict with
the project-wide Rust-first policy.

## Decision

The target-local policy checker is a small Rust workspace tool using only the Rust standard library.
Product behavior, MCP behavior, repository checks, fixtures-as-code, and tests remain Rust-first. The
foundation keeps `Directory.Build.props` only for parity; no managed project is part of this target.

## Consequences

The strict policy command is executable from this root without Python or a product crate. The checker is
deliberately limited to objective repository rules and does not pretend to prove external runtime
behavior. Adding another language or a managed boundary requires a superseding decision.
