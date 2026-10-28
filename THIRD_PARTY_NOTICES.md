# Third-party notices

This repository contains no copied third-party implementation source, proprietary game file,
host assembly, save, or credential. The policy tool uses only the Rust standard library.
The MCP package directly depends at runtime on `jsonschema` 0.60.1, with default
features disabled, and `serde_json` 1.0.154 from crates.io. These exact versions are
pinned in `crates/mcp-server/Cargo.toml` and `Cargo.lock`. They are used in embedded
Draft 2020-12 schema validation and JSON conversion, including the exact-restore
boundary; they are not only development dependencies.

The complete dependency graph, registry sources, and checksums are recorded in
[Cargo.lock](Cargo.lock). This correction is not a complete license inventory or
release approval. Before distribution, generate and review notices for the exact
lockfile, target, and distributed bytes, including transitive dependencies and
applicable upstream license, copyright, and attribution requirements.

When dependencies or fixtures are added, record their exact version, source, license, redistribution
status, and review owner here or in a generated notice tied to the committed lockfile. A dependency with
unknown provenance or incompatible terms blocks release until resolved.
