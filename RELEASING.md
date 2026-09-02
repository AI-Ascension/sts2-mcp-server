# Releasing

No release is authorized or defined for this foundation-only target. The current `0.0.0` workspace
version identifies preparation tooling, not a distributable MCP server.

When product behavior is approved, a release owner must first define the MCP profile, gateway contract
revision, supported Rust/runtime/platform combinations, package allowlist, and compatibility evidence.
The release must be built from an approved immutable revision with a committed lockfile and checksums.

Before any publication, verify:

- repository policy, formatting, lint, tests, protocol/gateway conformance, and security checks pass;
- every advertised MCP tool and gateway route has an owner-local fixture and versioned mapping;
- no proprietary host files, saves, credentials, private paths, or generated workspace state is packaged;
- compatibility evidence distinguishes build, transport, gateway, host, and end-to-end runtime levels;
- the changelog, compatibility record, notices, and package manifest describe the exact bytes; and
- maintainer approval and any protected release environment requirements are satisfied.

Preparation, tagging, publication, deployment, and post-release verification are separate authority
events. Never treat a passing local check as permission to perform them.
