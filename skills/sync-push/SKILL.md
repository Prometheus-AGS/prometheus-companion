---
name: sync-push
description: Push a sync domain to connected P2P peers via the Companion. Triggers CRDT delta exchange for skill indexes, learner models, or other enabled domains.
version: '1.0.0'
license: MIT
metadata:
  author: prometheus-companion
  category: sync
  tags: [sovereign-sync, p2p, sync, push, crdt, companion, loro]
---

# /sync-push

Submit one explicitly requested syncable domain through the selected Companion
host's signed v2 push contract. This optional connected skill is not needed for
the pack to operate. Source, registration and a network send are separate from
successful peer application.

## Procedure

1. Confirm the requested domain, project/signer and selected host. Use
   `sync-status`/`sync-peers` for actual transport observations. Never push
   local-only surreal-memory, arbitrary credentials or unspecified project data.
2. For project history, call MCP `sync-push` with `{ "domain": "kbd-control:<project-id>", "target_endpoint_ids": ["<paired-endpoint-id>"] }`. The MCP client can select a single explicitly configured bootstrap endpoint when targets are omitted. With multiple possible destinations, select the intended peers. A fresh signed project push over HTTP must already name its recipients when P2P is enabled.
   Generic domains require exactly one registered
   project for signer selection. Ambiguity or unavailable key access is an
   error, not permission to create another signer.
3. Retain `requestId`, exact `signedRequest`, endpoint, `intentPath`, HTTP status
   and the actual response. MCP durably stores private exact request/endpoint
   intent before POST under `<data-root>/prometheus/companion/mcp-push-outbox`.
   If that cannot be recorded, `not-submitted` means it sent no push. Intent is
   not a fabricated successful receipt.
4. Read the real receipt's `localState`, canonical payload hash, targets,
   per-peer states, failure and event sequence. Accepted/prepared are not peer
   delivery. Applied-locally means local-only application. Broadcast means a
   local send; do not claim every peer applied it. Failed broadcast can follow
   local preparation.
5. On connection/timeout/decoding failure, report **uncertain** and preserve the
   same ID/body/endpoint. After process interruption, recover them from the
   private intent file. Query `sync-push-receipt` with
   `{ "request_id": "<original-id>" }` on the original host before any replay.
   A receipt 404 is not proof a concurrent request was never received. If replay
   is authorized, send the unchanged request as `signed_request` alongside its
   original `domain`; do not change its time, targets, frontier, signature or ID.

Same-ID/same-canonical-payload replay returns the persisted receipt; altered
content under the same ID conflicts. Replaying a failed receipt does not restart
that operation. Inspect the real receipt/frontier before authorizing new work.
Never invent a second ID just to resolve response loss. Resume durable receipt
events using the last observed sequence (`after`/`Last-Event-ID`); AG-UI stream
output is not a sync receipt.

Connected skills use `POST /api/v2/sync/pushes`. Deprecated unsigned v1 push is
Unix-only compatibility behavior, not this delivery contract. Consult
[control APIs](../../docs/control-api.md) for the schema/SDK and
[maintenance](../../docs/maintenance.md) for rollback and acceptance. Preserve
keys/tickets privately; publish only redacted operation evidence.

## Progress signals

```
Starting task 1 out of 1: Submit and reconcile the requested sovereign-sync push
Completed task 1 out of 1: Submit and reconcile the requested sovereign-sync push
```
