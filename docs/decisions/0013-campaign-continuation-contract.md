# Campaign continuation contract

- Status: Proposed; coordinated consumer migration
- Date: 2026-09-06
- Owner: MCP adapter

## Decision

Consume protocol PR #14 revision `a81ec64d7d14bdb3079b8c7dc3c75e5c88693dfd`.
Its schema digest is `8e99cea36b7ede97532348fd8efe302ca79260895265a7bf14ddf7e006d8ff63`.
The complete MIT artifact, source schema and conformance companions are copied verbatim.
The revision adds `proceed`, `confirm_selection` and `cancel_selection`.
Producer and consumers migrate together; earlier digests remain rejected.

The MCP catalog advertises three argument-free payload alternatives. Request mapping and response projection admit the same closed variants. The adapter forwards a validated action once and retains operation identity; it does not infer host legality or settlement.

## Validation and limits

The continuation mapping regression exercises all three producer request vectors through the recording gateway and proves extra arguments do not cause another forward.

Workspace tests, formatting, Clippy and strict policy pass locally. These are component results.
They do not establish available native controls, live host effects or full campaign completion.
The game-mod owns those separate host evidence requirements. Merge only with the reviewed
producer revision and coordinated consumers.
