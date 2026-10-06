# Control and connected sync contracts

Production source is pending compilation and final local integration. These
examples are not historical receipts or certification.

## Selected host

Headless and windowed Companion share the same-user Unix socket and local
signed KBD authority. Local control is installed before optional P2P startup.
`sovereign-sync --mode daemon` is an alternative using the same production
supervisor/consumer path. Choose one socket owner. `--mode server` disables P2P.
`--mode mcp` serves stdio tools; its sync tools connect to an existing host using
`--socket` or `SOVEREIGN_SYNC_SOCKET`.

The MCP bridge has a bounded Unix HTTP client with proxies/redirects disabled.
It does not fall back to TCP, create another P2P identity or report its own empty
transport as the host's status. Unreachable host means unavailable. Explicit
loopback `--tcp` daemon/server mode requires a private bearer-token file; MCP
sync does not support that alternate transport. The default API is not public.

| Endpoint | Meaning |
|---|---|
| `GET /health` | Static listener/process liveness, distinct from authority/peers |
| `GET /ready` | Local authority startup readiness |
| `GET /openapi.json` | Signed v2 sync schema and examples |
| `GET /api/v1/sync/status` | Host transport, peer IDs and domain/privacy metadata |
| `GET /api/v1/sync/peers` | Host peers and transport snapshot |
| `GET /api/v1/services` | Manifest-supervisor reports, or `[]` without one |
| `GET /api/v1/kbd/projects` | Registered project routes |
| `/api/v1/kbd/projects/{project_id}/...` | Status, replicas, events, signed commands, claims, conflicts and audit/control routes |
| `POST /api/v2/sync/pushes` | Signed, receipt-backed push |
| `GET /api/v2/sync/pushes/{request_id}` | Durable receipt on the original host |
| `GET /api/v2/sync/pushes/{request_id}/events` | Durable event sequence and resume cursor |

The source router defines the full route inventory. Project identity, authorized
replica and expected frontier remain canonical checks. Registration, discovery
and merge status cannot replace acceptance. `/api/v1/stream` AG-UI events are
not the sync-push receipt stream.

## Status and privacy

MCP `sync-status` and `sync-peers` return the selected host's JSON. Optional
status `domain` annotates the request; it is not a convergence check.
`transport.state` is `disabled`, `initializing`, `bootstrapping`, `ready`,
`degraded`, `failed` or `stopping`. Read `attempt`, `lastError`, `nextRetryMs` and
peer IDs. Legacy `node_state` is a rollup; ready does not prove peer application.

Domain metadata labels skill-index public, learner-model and kbd-control trusted,
and surreal-memory local-only with a never-synced adapter. These describe policy
and wiring, not convergence. Local-only memory payloads must never be pushed.
Metadata is not authority to share arbitrary knowledge, credentials or project
data.

## Signed push and uncertain outcomes

Call MCP `sync-push` with `{ "domain": "kbd-control:<project-id>" }` or another
explicit syncable domain. A new request uses that project runtime's enrolled
device signer. Generic domains require exactly one registered project to choose
the signer; otherwise resolve ambiguity. The MCP process needs signing access.

The tool submits once and returns `requestId`, `signedRequest`, endpoint and
HTTP status/body, retaining structured rejections. A connection, timeout or
decoding failure is **uncertain**. Retain the exact request and endpoint; do not
create a new ID to settle response loss. Call `sync-push-receipt` with
`{ "request_id": "<original-id>" }` on the same host first. If an authorized
replay is appropriate, supply the unchanged returned request as `signed_request`
alongside its original `domain`. Do not change time, targets, expected frontier,
signature or ID.

Before POST, the MCP bridge stores a private, fsynced exact request/endpoint at
`<data-root>/prometheus/companion/mcp-push-outbox/<request-id>.json` and returns
`intentPath`. It requires an existing absolute selected data root and socket.
If recording fails, it reports `not-submitted` and sends nothing. Interrupted
recording leaves the original intent for inspection; malformed or conflicting
intent is not overwritten. After a process exit, inspect this outbox to recover
the original ID/body and query the original host. It records intent only, never
fabricated acceptance or peer delivery. Retain it with the matching receipts.
SDK callers using `sync_push_signed` must persist their own request before send.

Tool prefix aliases (`--prefix-tools`) and `UAR_SKILL_SERVICE_URL` passthrough
are not implemented contracts here. MCP mode rejects them rather than claiming
aliases or silently changing the declared tool set.

REST `SignedSyncPushRequest` uses schema `1.7`: `schemaVersion`, `requestId`,
`domain`, optional `targetEndpointIds`/`expectedFrontier`, `issuedAtMs`,
`signerKeyId` and `signature`. Use the real runtime signer and OpenAPI; never
fabricate a signature. Same-ID/same-canonical-payload replay returns the persisted
receipt; changed content under that ID conflicts. Signer, freshness and frontier
checks remain enforced. A receipt 404 does not prove an outstanding concurrent
request was never received; resolve outstanding work before electing to replay.

Receipt `localState` is `accepted`, `prepared`, `applied_locally`, `broadcast` or
`failed`. Broadcast records a local send, not peer application. Read per-peer
states as recorded. Local preparation can precede failed broadcast. Replaying
an existing failed receipt does not restart it; inspect actual receipt/frontier
before authorizing new work. Resume events with `after` or `Last-Event-ID`, using
the last real sequence.

Deprecated unsigned `POST /api/v1/sync/push` is same-user Unix-only and lacks the
v2 request/receipt contract. The SDK retains its legacy helper for compatibility;
connected skills use v2. TCP must not revive unsigned mutation. A successful API
response, gossip traffic, source merge or MCP registration each needs separate
real collaborator evidence before cross-device delivery is claimed.
