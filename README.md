# Prometheus Companion

Prometheus Companion is an optional host for the Prometheus skill pack's control
plane, peer synchronization and service supervision. The pack retains its local
signed KBD authority, services and skills and operates without Companion.
Dependencies flow from Companion to the pack.

This checkout is a source candidate. Compilation, local integration, approved
release pins, installation and functional acceptance remain separate gates. Its
initial `origin/main` is a private local staging mirror; no published remote or
release artifact is implied.

## Current scope

| Surface | Implemented source | Limit |
|---|---|---|
| `prometheus-companion-headless` | Same-user Unix control socket, signed KBD routes, optional configured P2P, service supervision | Requires an existing unattended signing key; macOS launchd adoption |
| Tauri shell | Hosts the same node; five service-health tray states | Window still contains the starter greeting screen; no dashboard, pairing screen or tray action menus |
| `sovereign-sync` | Stdio MCP bridge, standalone daemon/server, status, pairing CLI, OpenAPI | One selected control host per socket; server mode has no P2P |
| `sovereign-client` | REST/SSE SDK, Unix client, signed v2 push and receipt lookup | Caller must retain request identity and distinguish receipt states |
| Connected skills | `sync-status`, `sync-peers`, `sync-push` through the selected host | Require a running host; MCP registration alone does not start one |
| `kbd-mobile`, `iroh-docs-adapter` | Relocated embedded replica and P2P storage source | No supported mobile application or newly certified storage deployment |

The node installs local authority before starting its P2P supervisor. Disabled,
failed or retrying transport remains visible separately from local readiness.
Incoming authenticated frames enter the same production control and CRDT
handlers as the standalone daemon. Service reports come from the selected
pack's `shared/services.manifest.json`; an empty list cannot establish health.

## Build and install

Use the repository's Rust workspace and pinned Node/pnpm frontend. The headless
build targets are:

```bash
cargo build -p prometheus-substrate --no-default-features --features headless --bin prometheus-companion-headless
cargo build -p sovereign-sync --bin sovereign-sync
```

These are operator instructions, not successful build evidence. During an active
delivery phase, production work must finish before consolidated local gates run;
only one Cargo/rustc build may run on the machine. Desktop packaging uses the
Tauri frontend separately and is not required for headless deployment.

The four pack consumers in `Cargo.toml` select exactly
`ba5c4516fd72e60c21ab3bb1d09a63105aeb3c61` of
`Prometheus-AGS/prometheus-skill-system`. `versions.toml` remains operator
protected; the lockfile must be reconciled and compiled at the final gate before
a reproducible release is claimed. Crate relocation commits are provenance,
not the current dependency selection.

[Installation](docs/installation.md) covers loading the headless LaunchAgent
and registering the connected skills through Claude Code's user-scoped MCP CLI.
Both installers offer a read-only `--dry-run`; neither builds a binary or grants
identity enrollment.

## Use and contracts

Choose one host: headless, Tauri, or `sovereign-sync --mode daemon`. They share
the socket contract and must not compete for it. `--mode server` serves local
control with disabled P2P. Stdio MCP connects its sync tools to that selected
host; it does not substitute an empty local transport for the host's status.

```bash
sovereign-sync --mode status --format json
```

This reports liveness and latency, not peer convergence. `/ready`, transport
status and signed command/push receipts answer different questions. See
[control APIs](docs/control-api.md) for REST, MCP and signed v2 push recovery.
[Installation](docs/installation.md) explains explicit private peer enrollment.
Discovery supplies addresses for known endpoints; it grants no access.

## Support, maintenance and history

[Maintenance](docs/maintenance.md) defines the finite platform/harness scope,
update/rollback, identity/journal recovery, dependency maintenance and remaining
acceptance requirements. Release ownership stays with the Prometheus repository
owner. Report regressions with redacted artifact/source identities and the exact
failed contract; never include keys or pairing tickets.

The [architecture draft](docs/00-architecture-and-implementation-plan.md) and
[UI package](docs/ui/README.md) preserve authored design history. Their dashboard,
agentic UI and mobile roadmap do not describe shipped capabilities.

MIT: see [LICENSE](LICENSE). Dependency notices, approved source inventory and
private publication remain release-owner work; this candidate is not a published
release merely because its license file is present.
