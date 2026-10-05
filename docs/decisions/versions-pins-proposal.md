# Proposed `versions.toml` pins — Companion substrate workspace

Status: **awaiting operator edit**
Raised by: `change-cpc-003-companion-workspace` (phase `control-plane-to-companion`)
Date: 2026-09-02

## Why this file exists

`versions.toml` is authoritative for dependency pins in this repository, and
`.claude/settings.json` denies `Edit(versions.toml)` to agents. That is
deliberate: pins are a human decision. This file stages the proposal so the
edit is a review, not a research task.

`change-cpc-003` and every later Companion change that adds a crate stays
**BLOCKED** until the corresponding pins land here as a hand edit.

## Proposed `[pins]` entries

Copy into `versions.toml` under `[pins]`:

```toml
[pins]
# --- Rust substrate: relocated from prometheus-skill-pack (change-cpc-004) ---
iroh              = "1.0.3"   # resolved in the pack's sovereign-sync Cargo.lock; manifest floor is 1.0.2 (relay DoS security floor)
iroh_gossip       = "0.101"   # membership; already in use by sovereign-sync
loro              = "1.13"    # CRDT; HMA ADR-LFS-5 permits no alternative
redb              = "2"       # sovereign-sync's own persistence
axum              = "0.8"     # the node's HTTP surface
rmcp              = "1.8"     # MCP server (the spec's Appendix A said 0.1 — stale by a major version)

# --- Discovery (change-cpc-006) ---
iroh_mdns_address_lookup     = "0.5.0"  # LAN discovery; depends on iroh ^1.0.0
iroh_mainline_address_lookup = "0.5.0"  # Mainline DHT lookup; depends on iroh ^1.0.0; publication OFF by default

# --- Inherited from the pack's kbd-runtime (change-cpc-003) ---
keyring           = "=3.6.3"  # EXACT. kbd-runtime pins it exactly and holds the device key
                              # as credential-store item `prometheus-kbd-device`. A second
                              # major version of keyring in one graph is a defect, not a choice.
dirs_next         = "2"       # shared-state path resolution; never Tauri's path resolver

# --- Node hosting (change-cpc-005) ---
tauri_plugin_stronghold = "2.3.2"  # operator/provider credentials only, per spec §17
```

## Notes for the reviewer

- **`keyring` must be exact (`=3.6.3`).** It is not a preference. `kbd-runtime`
  pins that exact version and stores the canonical device key through it; a
  caret range would allow a second major version into the same dependency graph.
- **`iroh` is 1.0.3, not 0.30.** The architecture plan's Appendix A pins 0.30,
  which is stale by a major version. `rmcp` is likewise 1.8, not 0.1. Both are
  corrected by task 4 of this change.
- **Mainline DHT publication is off by default.** The crate is pinned so the
  capability exists; enabling publication is a per-operator opt-in because it
  publishes this node's reachability record to a public DHT
  (`change-cpc-006`, decision D-13).
- These are the pins known at the time of writing. Later Companion changes that
  introduce a crate append to this file and block on the same gate.

## Suggested `[decisions]` entries

```toml
[decisions]
substrate-consumption = "Companion consumes prometheus-skill-pack crates as git dependencies pinned by rev; never a path dependency into the pack, and no committed [patch] to a local checkout. The pack never depends on the Companion (operator decision D-02, option D)."
device-key-store = "The device key stays in the OS credential store under kbd-runtime's keyring =3.6.3 as item `prometheus-kbd-device`. Stronghold is scoped to operator and provider credentials only."
dht-publication = "iroh Mainline address lookup is compiled in but publication is off by default; enabling it is a per-operator opt-in because it publishes node reachability to a public DHT."
```

## Operator record

### Pins edit (gate 1)

- `keyring = "=3.6.3"` landed 2026-09-02, exact as required.
- **Still outstanding:** the other ten entries above (`iroh`, `iroh_gossip`,
  the two `iroh_*_address_lookup` crates, `loro`, `redb`, `axum`, `rmcp`,
  `dirs_next`, `tauri_plugin_stronghold`) plus the three `[decisions]` entries.
  `change-cpc-003` tasks 1, 2, and 6 stay blocked until they land, because the
  workspace manifests they write must carry exact pins (Companion AGENTS.md
  §15, enforced by `scripts/audit-deps.sh`).

### Pack commit to pin (gate 2) — CLEARED

The skill-pack commit containing `change-cpc-002` now exists:

```
rev = "cfbc262b47ce794cb2ea845c64dc33e70b10e890"
```

`cfbc262` on branch `feat/cpc-001-002-integration-contract` in
`Prometheus-AGS/prometheus-skill-system`. It carries the integration contract
and the skill-ffi cut, so a Companion pinned to it consumes a `skill-ffi` with
no `kbd-mobile` dependency.

**Not yet pushed.** A `git` dependency resolves from the remote, so this rev
must be pushed (or the branch merged) before any machine other than this one
can build the Companion against it.
