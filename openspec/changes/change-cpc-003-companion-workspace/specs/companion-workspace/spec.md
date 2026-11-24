## ADDED Requirements

### Requirement: Workspace builds the substrate crate in both presets
`cargo check -p prometheus-substrate --features headless` and `--features desktop` SHALL each succeed as a narrowly targeted diagnostic; `bash scripts/audit-all.sh` SHALL exit 0.

#### Scenario: Headless preset
- **WHEN** the headless preset is checked
- **THEN** no tauri dependency is selected

#### Scenario: Pins present
- **WHEN** `versions.toml` `[pins]` is populated by the operator
- **THEN** `bash scripts/audit-deps.sh` passes with no wildcard or caret ranges

### Requirement: Pack crates are consumed by git rev only
The Companion SHALL depend on pack crates by `git` + `rev`; no `path` dependency into the pack and no committed `[patch]` to local checkouts.

#### Scenario: Committed manifest
- **WHEN** the workspace manifests are inspected
- **THEN** every pack crate dependency carries a `rev` and no `[patch]` section targets a local path

### Requirement: Companion runtime is reconciled before mirroring
The Companion runtime SHALL record the assess stage for `docs-review-for-build-assessment` in agreement with the phase artifacts.

#### Scenario: Reconciled
- **WHEN** `prometheus kbd --path <companion> status --json` runs
- **THEN** the assess stage for that phase is complete, not `ready`
