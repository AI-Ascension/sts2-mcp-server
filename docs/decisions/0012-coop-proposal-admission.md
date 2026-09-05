# ADR 0012: Separate co-op proposal admission

- Status: Proposed; blocked on shared-contract admission
- Date: 2026-09-05

## Scope and lineage

This proposal restores the co-op schema, read-only library catalog, mapping, projection and tests
removed from the Runtime-v3 merge lane. The source is preserved exactly from commit
198caa3303d2ee82a72728b943b4e4f0eba0de69 on review/mcp-coop-proposal-source-20260905, including the
bound MCP/gateway session correction. The proposal is based on split commit
65449d4da2feee18fec6c4991aa489a2aa0b9b5b. Its exports belong only to this proposal branch; the
Runtime-v3 merge lane has no co-op surface.

## Admission blocker

A shared contract requires at least two named actual serialized-contract consumers. MCP consumes
only synchronization responses: metadata, peers and synchronization, with action, vote, shared-effect
and ally-target fields null. Its schema validation test demonstrates that subset, not a second
consumer for each exported protocol type. Source-only status does not waive the admission gate.
Do not merge this proposal until protocol ownership records the required consumer evidence or
narrows the schema to the admitted contract and all consumer copies/tests follow that decision.

## Evidence boundaries

The copied schema digest is 85e0028c1ae20e49542791da165eeabaaea0cc2023626b5094b6660ebcc0cc81.
The five co-op tests cover schema-valid synthetic synchronization, read-only mapping, invalid peer
counts, unknown input, and separate bound MCP/gateway sessions. There is no co-op executable selector,
multiplayer mutation barrier, host observation, or live multiplayer compatibility claim.

ADR 0009's accepted status applies only to its Runtime-v3 six-tool profile. This separate proposal
has no accepted admission decision.

The proposal was rebased onto Exo commit f31928c947503a613b04a290516b9419f05477aa, preserving
its standalone session defaults, typed scope errors, and full frozen artifact inventories.
