## 1. Workspace

- [ ] 1.1 Add root `Cargo.toml` workspace (`crates/prometheus-substrate`, `src-tauri`) and scaffold the substrate crate with `desktop` / `headless` features. **Blocked on the operator's `versions.toml` pins.**
- [ ] 1.2 Add git-rev dependencies on `kbd-runtime`, `storage-provider`, `learner-model`, `skill-index` at the pack commit containing `change-cpc-002`. **Blocked: that commit does not exist yet.**

## 2. Pins and specification

- [x] 2.1 Stage the proposed pins in `docs/decisions/versions-pins-proposal.md` for the operator's hand edit.
- [x] 2.2 Rewrite Appendix A for git-rev consumption; correct `iroh` 0.30 → 1.0.3 and `rmcp` 0.1 → 1.8.

## 3. Runtime and mirror

- [x] 3.1 Initialize OpenSpec, mirror this change, and reconcile the `docs-review-for-build-assessment` runtime stage with its on-disk assess artifacts.
- [ ] 3.2 Record the scaffold audit-gate listing and run `cargo check` (headless, desktop) plus `bash scripts/audit-all.sh`. **Blocked with 1.1/1.2.**
