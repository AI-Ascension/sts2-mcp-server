# `coop-native-v1` candidate

This directory is an unadmitted, source-derived candidate. Its fixtures are projections of the
managed producer capture at `root/coop-native-producer-capture-20260909-evidence-r4`; the capture
wrapper records the complete producer request and response, while each golden here records one
envelope. The vectors cover observation, local action settlement/rejection/uncertainty, same
operation reconciliation, shared event voting, and rejoin/recovery.

The corrected producer declares the exact candidate schema digest
`3e555563023804383534d92118c3863aa2aee3d0d24b932f484d8fd97e452ca8` at source commit
`8fa255e9ead3ad7077c26a5980c2f28710fae11f` (tree
`25821adf9160e1bfa89ea578022b63f9e83c8699`). The r4 capture is freshly recaptured from that
source revision and records the matching declaration; this establishes source serialization parity
while the candidate remains unadmitted pending independent consumer and live native evidence.

`producer-capture.json` records the source commit/tree, the eight source capture wrapper hashes,
and the one-to-one projection of their request/response members to the fifteen checked-in
goldens. Each golden is byte-identical to `jq -c` of its listed `/capture/request` or
`/capture/response` member (including the terminating newline). The source-derived vocabulary
keeps all five managed action parser kinds, the four installed native pending-operation effect
kinds plus the two synthetic capture aliases, and the producer's disabled/enabled_unread/enabled/
divergent/matched checksum statuses. A remote peer's checkpoint is nullable until its native
checksum is readable. Recovery requests and settled/rejected recovery responses
carry a `reconcile` value; an unknown response may retain the producer's `rejoin` recovery marker
while a rejoin is pending. Rejoin requests are the only mutation-shaped rejoin messages.

No consumer is admitted. The candidate does not move host authority, peer admission, gateway
leases, MCP framing, harness coordination, transport, or provider behavior into this repository.
Admission requires the exact producer digest to match, at least two named consumers, and
independent cross-boundary conformance evidence. These fixtures do not establish a native STS2
session or live settlement.
