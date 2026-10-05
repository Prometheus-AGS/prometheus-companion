> Current source selection (2026-10-05): pack consumers select exact full
> revision `ba5c4516fd72e60c21ab3bb1d09a63105aeb3c61` in workspace `Cargo.toml`.
> The relocation hash below remains historical provenance. Compilation,
> contract compatibility and installed acceptance remain final local gates.
> See [current operational scope](../../README.md) and
> [control contracts](../../docs/control-api.md); mobile library/storage source
> does not establish a supported application/deployment.

# iroh-docs-adapter

Relocated from `prometheus-skill-pack` by `change-cpc-004-relocate-sovereign-sync`
(phase `control-plane-to-companion`).

| | |
|---|---|
| **Source** | `substrate/storage-provider/src/iroh_docs.rs` |
| **Source commit** | `cfbc262b47ce794cb2ea845c64dc33e70b10e890` |
| **Approx. size at move** | 637 LOC |
| **Moved** | 2026-09-02 |

The iroh-docs backend for the pack's StorageProvider trait. The trait and its local adapters stay in the pack; only this P2P-backed implementation moves.

## Why it moved

Operator decision D-02 (option D) draws the product boundary: the skill pack is
open source and complete on its own, and the Companion is the control and
extension framework. This crate is **pure control** — sync, replication, and
peer transport that provide no function the pack's own skills need. It is
therefore Companion software, not pack software.

What stayed in the pack: the `prometheus` CLI, `kbd-runtime` (the local signed
KBD authority), `skill-index`, `learner-model`, the `StorageProvider` trait
with its local adapters, and every functional service. The pack certifies fully
with no Companion installed.

## Dependency direction

This crate may depend on pack crates **by git rev only**. The pack must never
depend on anything here. See the pack's `docs/integration-contract.md`
(contract v1.0.0) for the four seams that keep that boundary honest.
