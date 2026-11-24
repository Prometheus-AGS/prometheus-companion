# sovereign-client

The Rust client connects to the selected host. Its `sync_push_signed` and `sync_push_receipt` methods let callers persist an exact signed request, submit it, and reconcile uncertain replies. [Transport and receipt semantics](../../docs/control-api.md)

The current workspace consumes Full v1.17.0 at `acd029107533c07a661a7e3ad90eb28834688afa`. The table below records the original relocation.

Relocated from `prometheus-skill-pack` by `change-cpc-004-relocate-sovereign-sync`
(phase `control-plane-to-companion`).

| | |
|---|---|
| **Source** | `substrate/sovereign-client` |
| **Source commit** | `cfbc262b47ce794cb2ea845c64dc33e70b10e890` |
| **Approx. size at move** | 355 LOC |
| **Moved** | 2026-09-02 |

Rust SDK for the node's REST API and AG-UI SSE stream.

## Why it moved

Operator decision D-02 (option D) draws the product boundary: the skill pack is
open source and complete on its own, and the Companion is the control and
extension framework. This crate is **pure control** — sync, replication, and
peer transport that provide no function the pack's own skills need. It is
therefore Companion software, not pack software.

What stayed in the pack: the `prometheus` CLI, `kbd-runtime` (the local signed
KBD authority), `skill-index`, `learner-model`, the `StorageProvider` trait
with its local adapters, and every functional service. The pack retains its own local signed authority, and can read a registered project with Companion stopped.

## Dependency direction

This crate may depend on pack crates **by git rev only**. The pack must never
depend on anything here. See the pack's `docs/integration-contract.md`
(contract v1.0.0) for the four seams that keep that boundary honest.
