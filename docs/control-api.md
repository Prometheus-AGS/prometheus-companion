# Control and connected sync contracts

Companion 0.2.0 provides both a same-user control socket and a paired peer transport. The socket accepts a local signed mutation. A separate signed push can deliver that history to another authority. [Verified scopes and binaries](intersession-0.2.0-verification.json)

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

## Send project history to one paired peer

Suppose A has recorded a signed KBD command and B shares the same enrolled baseline. You want B's running session to see the update and to verify independently that B kept it. Use a terminal with A’s selected sync configuration, registry, data root and enrolled signing key. Set these shell variables to the values for your own nodes.

| Variable | Value |
|---|---|
| `SYNC_CLI` | Absolute path to A’s `sovereign-sync` executable |
| `PROJECT_ID` | Registered project UUID shared by A and B |
| `B_ENDPOINT` | B's paired endpoint ID |
| `A_SOCKET`, `B_SOCKET` | Absolute paths to the two independent Unix sockets on this machine |
| `COMMAND_ID` | ID of the signed command already present on A that you want to verify at B |

Keep the request and receipt in a private directory so a retry can use the exact request bytes.

```bash
umask 077
PUSH_DIR=$(mktemp -d "${TMPDIR:-/tmp}/kbd-push.XXXXXXXX")
"$SYNC_CLI" --mode sign-kbd-push --project-id "$PROJECT_ID" --peer "$B_ENDPOINT" > "$PUSH_DIR/request.json"
jq -e --arg peer "$B_ENDPOINT" '.targetEndpointIds == [$peer]' "$PUSH_DIR/request.json"
curl --silent --show-error --unix-socket "$A_SOCKET" \
  --header 'Content-Type: application/json' --data-binary "@$PUSH_DIR/request.json" \
  --output "$PUSH_DIR/receipt.json" --write-out '%{http_code}\n' \
  http://localhost/api/v2/sync/pushes
jq '{pushId, localState, perPeer, failure}' "$PUSH_DIR/receipt.json"
```

A new completed transfer returns HTTP 201. Look for `perPeer[B_ENDPOINT].state == "applied"`, then check the signed event on B. The `/audit` route returns one JSON event per line. For a quiet project with no further writes, the two project status responses should also show the same revision and frontier.

```bash
(
set -e
for side in A B; do
  socket=$A_SOCKET
  if [ "$side" = B ]; then socket=$B_SOCKET; fi
  curl --fail --silent --show-error --unix-socket "$socket" \
    "http://localhost/api/v1/kbd/projects/$PROJECT_ID/audit" > "$PUSH_DIR/$side.audit"
  jq -cesS --arg command "$COMMAND_ID" \
    'map(select(.commandId == $command)) | if length == 1 and (.[0].signature | length) > 0 then .[0] else error("expected one signed event") end' \
    "$PUSH_DIR/$side.audit" > "$PUSH_DIR/$side.event"
done
cmp "$PUSH_DIR/A.event" "$PUSH_DIR/B.event"
)
```

Both filter commands and `cmp` must succeed. Keep the directory private while reconciling the operation. To watch B live, subscribe on B’s socket to `GET /api/v1/kbd/projects/{project_id}/events/stream` before A writes. A receiver event, or a read from B after restart, answers a different question from the sender's receipt.

## Connected skills and uncertain outcomes

An MCP caller can use this argument shape with the `sync-push` tool. Substitute the project and endpoint IDs before submitting.

```json
{"domain":"kbd-control:<project-uuid>","target_endpoint_ids":["<paired-endpoint-id>"]}
```

With exactly one configured bootstrap endpoint, the MCP client can fill in that endpoint before signing a domain-only call. Select destinations explicitly when several peers are possible. Generic sync domains need a single registered project to select the signer. An external HTTP request for private project history must already name its recipients in the signed `targetEndpointIds` array whenever P2P is enabled. An unaddressed request there returns HTTP 422.

The MCP tool submits once and returns `requestId`, `signedRequest`, endpoint and
HTTP status/body, retaining structured rejections. A connection, timeout or
decoding failure is **uncertain**. Retain the exact request and endpoint; do not
create a new ID to settle response loss. Call `sync-push-receipt` with
`{ "request_id": "<original-id>" }` on the same host first. If an authorized
replay is appropriate, supply the unchanged returned request as `signed_request`
alongside its original `domain`. Do not change time, targets, expected frontier,
signature or ID.

For the HTTP walkthrough, recover the original ID from the saved request. Retrieve the receipt from A's socket and inspect the per-peer entry before deciding whether to retry. An authorized retry uses the same POST command and the unchanged `request.json`. A successful identical replay returns HTTP 200 and the original receipt.

```bash
REQUEST_ID=$(jq -er '.requestId' "$PUSH_DIR/request.json")
curl --fail --silent --show-error --unix-socket "$A_SOCKET" \
  "http://localhost/api/v2/sync/pushes/$REQUEST_ID" > "$PUSH_DIR/recovered.json"
jq -e --arg peer "$B_ENDPOINT" '.perPeer[$peer].state == "applied"' "$PUSH_DIR/recovered.json"
```

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

The schema `1.7` REST request includes `schemaVersion`, `requestId`,
`domain`, `targetEndpointIds`, optional `expectedFrontier`, `issuedAtMs`,
`signerKeyId` and `signature`. Use the real runtime signer and OpenAPI; never
fabricate a signature. Same-ID/same-canonical-payload replay returns the persisted
receipt; changed content under that ID conflicts. Signer, freshness and frontier
checks remain enforced. A receipt 404 does not prove an outstanding concurrent
request was never received; resolve outstanding work before electing to replay.

Receipt `localState` is `accepted`, `prepared`, `applied_locally`, `broadcast` or
`failed`. For an addressed KBD transfer, the top-level value can read `broadcast` even after the recipient acknowledged application. Inspect the `applied_at_targets` event, the per-peer result and B’s own audit. A recorded send alone establishes only the sender’s action. Local preparation can precede failed broadcast. Replaying
an existing failed receipt does not restart it; inspect actual receipt/frontier
before authorizing new work. Resume events with `after` or `Last-Event-ID`, using
the last real sequence.

The deprecated unsigned `POST /api/v1/sync/push` is same-user Unix-only and declines private project-history pushes when peer transport is enabled. Use the signed v2 route for KBD. The SDK retains its legacy helper for compatibility;
connected skills use v2. TCP must not revive unsigned mutation. A successful API
response, gossip traffic, source merge or MCP registration each needs separate
real collaborator evidence before cross-device delivery is claimed.
