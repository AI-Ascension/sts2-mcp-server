# Security policy

## Scope

The security boundary is the external MCP adapter and its authenticated connection to the gateway. The
gateway owns instance lifecycle, leases, registry state, routing authority, and downstream credentials.
The game-mod/host owns game state and mutations. This repository must not bypass either boundary.

The adapter must fail closed for missing or mismatched credentials, unknown profiles/tools, invalid
session or instance bindings, arbitrary routes, oversized input/output, malformed downstream responses,
and ambiguous timeout or cancellation state. Localhost is not an authorization decision. Logs and tool
content must not expose tokens, saves, prompts, personal paths, multiplayer identities, or raw untrusted
payloads without sanitization.

## Reporting

Report a suspected vulnerability privately to the repository maintainers through the approved security
contact. Include a concise reproduction, affected boundary, versions or revisions, impact, and safe
mitigation. Do not disclose secrets or proprietary game material in an issue or pull request.

Do not probe a live gateway, game, provider, valued profile, or third-party service without explicit
authorization. Use deterministic fakes or disposable environments for security tests. A missing runtime
probe is `unverified`, not a security pass.

## Dependency and release hygiene

Keep dependencies minimal, locked, reviewed, and sourced from declared registries. Do not vendor
unreviewed code. Release review must inspect the exact package contents and dependency notices before
publication.
