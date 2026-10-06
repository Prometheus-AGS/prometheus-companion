---
name: sync-peers
description: List peers in the Companion's sovereign-sync P2P network, and pair a new device with a ticket. Manage which nodes participate in CRDT state synchronization.
version: '1.0.0'
license: MIT
metadata:
  author: prometheus-companion
  category: sync
  tags: [sovereign-sync, p2p, peers, iroh, companion, pairing, network]
---

# /sync-peers

Read the selected Companion host's real peer list. Explicit peer enrollment
uses the existing pairing CLI. This optional connected skill is Companion-owned;
it is not required for normal pack operation.

## Read peers

Call the registered `sync-peers` MCP tool and report its `peers` and `transport`
fields as returned. The bridge uses the selected Unix host, not an independent
empty in-process transport. Unavailable, disabled, retrying and ready are
separate outcomes. Peer presence or discovery is not signed-command acceptance
or proven CRDT convergence.

Do not invent `/sync-peers add-ticket`, peer CRUD HTTP routes, QR screens or
short-code workflows. There is no shipped pairing UI. Listing peers does not
implicitly authorize enrollment or changes to a running service.

## Explicit enrollment

For operator-authorized enrollment, follow [installation](../../docs/installation.md)
with the selected configuration/identity paths. Back up current group membership.
A exports using `sovereign-sync --config /absolute/config.toml --mode pair-export`.
Transfer that secret ticket privately to B. B imports with `--mode pair-import
--ticket <private-ticket>`, then exports its updated ticket back to A for import.
Import replaces the group's secret and adds the endpoint/fingerprint binding;
this sequence preserves the shared group secret. Protect the enrollment session
from CLI history/process-list exposure; never paste a real ticket in chat,
issues, published evidence or logs.

Put each authorized endpoint ID in the other node's `[peers] bootstrap` list,
then restart only the selected host so it reloads identity/configuration. Do not
start a second owner on the same socket. The network uses authenticated Ed25519
endpoints over iroh QUIC/TLS. Network pairing does not grant KBD replica/project
mutation authority; those signed control checks remain separate.

Default mDNS supplies addresses for known endpoints on the local link, without
automatic admission or invented peer IDs. DHT lookup/publication is off by
default and must be explicitly selected; it publishes endpoint/relay/address
metadata publicly. Relay traffic is a separate policy concern.

Report enrollment changes, the actual new peer observation and pending
cross-device integration separately. A paired ticket alone never proves delivery.
See [control APIs](../../docs/control-api.md) and
[maintenance](../../docs/maintenance.md) for contract and recovery limits.

## Progress signals

```
Starting task 1 out of 1: Inspect or explicitly enroll sovereign-sync peers
Completed task 1 out of 1: Inspect or explicitly enroll sovereign-sync peers
```
