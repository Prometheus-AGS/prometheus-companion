## Why

This repository is a `pnpm create tauri-app` scaffold plus governance: 14 lines of Rust in `src-tauri/src/lib.rs`, no `crates/` directory, no Cargo workspace, and an empty `versions.toml` `[pins]` table. Every later change in the `control-plane-to-companion` phase needs a workspace and a substrate crate to build into.

The phase is governed by operator decision D-02 (option D): the skill pack stays open source and complete on its own; this Companion is the paid control and extension framework. The pack never depends on the Companion. Consequently this repository consumes the pack's open crates **by git rev**, and that direction is one-way.

The architecture plan's Appendix A assumed a skill-pack superworkspace that does not exist, and pinned `iroh = "0.30"` and `rmcp = "0.1"` — both stale by a major version against the pack's resolved `iroh 1.0.3` and `rmcp 1.8`.

## What Changes

- Add a root `Cargo.toml` workspace with members `crates/prometheus-substrate` and `src-tauri` (the HMA `scaffold-tauri-tray.sh` shape; graph-explorer precedent).
- Scaffold `crates/prometheus-substrate` with `desktop` and `headless` presets and git-rev dependencies on `kbd-runtime`, `storage-provider`, `learner-model`, and `skill-index`.
- Stage the required `versions.toml` pins in `docs/decisions/versions-pins-proposal.md`. **Agents are denied `Edit(versions.toml)`; this change stays BLOCKED until the operator's hand edit lands.**
- Rewrite Appendix A for git-rev consumption and correct the stale pins.
- Initialize OpenSpec here, mirror this change, and reconcile the `docs-review-for-build-assessment` runtime stage with its on-disk assess artifacts.

## Impact

Adds a Cargo workspace to a repository that had none. No behavioural change to the existing Tauri scaffold. The `versions.toml` edit is an operator action, not an agent action.

Canonical record: `.kbd-orchestrator/changes/change-cpc-003-companion-workspace/` in `prometheus-skill-pack`, which owns this phase's KBD run. Evidence for the tasks executed here is recorded in that change's `verification.md`.
