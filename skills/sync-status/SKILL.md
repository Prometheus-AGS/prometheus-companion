---
name: sync-status
description: Show the current P2P sync status of the Companion's sovereign-sync node — connection state, peer count, and domain health.
version: '1.0.0'
license: MIT
metadata:
  author: prometheus-companion
  category: sync
  tags: [sovereign-sync, p2p, sync, status, companion, iroh]
---

# /sync-status

Read the selected Companion host's live transport and domain metadata. This
connected skill belongs to Companion; the pack remains usable without it.
Installation/source presence and listener liveness do not prove peer delivery.

## Procedure

1. Use the registered `sync-status` MCP tool. It connects to the selected
   same-user Unix socket, including `SOVEREIGN_SYNC_SOCKET` or the MCP process's
   explicit `--socket`. Its optional `domain` annotates the request; it does not
   verify convergence. Never substitute a local disabled transport for the host.
2. If the tool reports unavailable, name the actual endpoint/error. Confirm the
   selected host and client scope, and follow [installation](../../docs/installation.md).
   Do not silently install, pair, restart or create another identity/host.
3. Read `transport.state`, `nodeId`, `peers`, `attempt`, `lastError`, `nextRetryMs`
   and the domain/privacy metadata. States are disabled, initializing,
   bootstrapping, ready, degraded, failed or stopping. Legacy `node_state` is a
   rollup; report the returned values without inventing Connected/Syncing states.
4. State which evidence exists: local readiness, transport state and peer IDs.
   Ready with no peers is not proof of peer convergence. An empty service report
   is not proof all pack services are healthy. Surreal-memory is local-only and
   must never be offered as a syncable payload.

The operator status CLI is `sovereign-sync --mode status --format json`. It
reports liveness/latency separately from `/ready` and sync state. A healthy Unix
listener need not bind `127.0.0.1:7892`; TCP exists only when explicitly selected
with private bearer authentication. Connected MCP sync uses Unix, without an
implicit TCP/proxy fallback.

For API shape and the meaning of each boundary, use [control APIs](../../docs/control-api.md).
The finite supported matrix and remaining acceptance gates are in
[maintenance](../../docs/maintenance.md). Do not report source compilation,
installation or final acceptance from a status response alone.

## Progress signals

```
Starting task 1 out of 1: Check sovereign-sync status
Completed task 1 out of 1: Check sovereign-sync status
```
