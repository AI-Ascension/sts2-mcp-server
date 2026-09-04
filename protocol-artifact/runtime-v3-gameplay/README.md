# Earlier Runtime-v3 gameplay artifact

The schema, manifest, checksums, eight goldens, source schema and conformance inventory are copied
byte-for-byte from `AI-Ascension/sts2-protocol` PR #7 commit
`11a7979f7368c78c10924337228991d16c9ec92a`. This is the earlier bounded `play_card` proposal,
not the broader Runtime-v3 proposal in protocol PR #8 and MCP PR #8. Their matching profile names
do not imply compatible envelopes: the exact schema digest is required.

`runtime_v3_gameplay_artifact` checks every checksum entry (including source and conformance),
validates all eight canonical goldens against the copied schema, and projects the producer's
targeted reconciliation receipt through the MCP consumer. Source/fake tests are not live-host
or executable gateway interoperability evidence.
