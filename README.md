# Prometheus Companion

Two KBD sessions can work against the same local project using the Full skill pack alone. Run Companion when a session needs a control socket, a separately paired copy of the project's signed history, or a supervisor for the services in an installed pack. Each Companion process owns its own local authority. A signed push delivers work to a recipient whose endpoint and project membership have both been authorized.

## Choose how to work

| What you need | Entry point |
|---|---|
| One local KBD authority shared by sessions | Use the Full pack CLI in the registered checkout. The pack can keep working while Companion is stopped. |
| One process serving local clients | Run `prometheus-companion-headless` with an enrolled device key. Clients select its Unix socket. The Tauri app and `sovereign-sync --mode daemon` can also own a socket. Start one owner per socket. |
| Work shared between two separately running authorities | Pair the two endpoints and enroll the device identities in the project. Send an addressed push and read the event back from the receiver. [Signed transfer walkthrough](docs/control-api.md#send-project-history-to-one-paired-peer) |
| Connected sync tools in Claude Code | Register the three connected skills with the skill-package installer and point the MCP process at the selected host. [Installer previews and ownership](docs/installation.md) |
| Troubleshooting, upgrades or rollback | Compare the receiver's signed audit and frontier with the sender's, retain the original request ID and preserve the enrolled keys. [Operations and recovery](docs/maintenance.md) |

## Read a local project without Companion

From an already registered project, set `PROJECT_ROOT` to its absolute checkout path. With the Companion socket absent, the Full pack CLI can read local signed state. A second KBD session using the same local authority can observe the same writes.

```bash
prometheus kbd --path "$PROJECT_ROOT" status --json
```

Use a second, independently enrolled authority when the receiver needs its own persisted copy or separate checkout. The walkthrough below uses that setup.

## Build and connect

From the Companion checkout, build the locked versions of the headless host and sync CLI.

```bash
cargo build --locked -p prometheus-substrate --no-default-features --features headless --bin prometheus-companion-headless
cargo build --locked -p sovereign-sync --bin sovereign-sync
```

Set `SOVEREIGN_SYNC_SOCKET` to the Unix socket selected for your host. Once the host is running, these commands show whether the listener responds and whether its local authority is ready.

```bash
target/debug/sovereign-sync --mode status --format json --socket "$SOVEREIGN_SYNC_SOCKET"
curl --fail --silent --show-error --unix-socket "$SOVEREIGN_SYNC_SOCKET" http://localhost/ready
```

Local readiness comes first. Peer discovery and service supervision start separately. Use the receiver's audit to check a transfer and the selected pack's service manifest to interpret a service report. [Control API](docs/control-api.md)

## Version and platform coverage

Companion 0.2.0 consumes four crates from Full v1.17.0 at commit `acd029107533c07a661a7e3ad90eb28834688afa`. The local macOS ARM64 verification exercised the headless host, the CLI, linked MCP, the standalone fallback and two-process signed replication. Clean-machine installation remains deferred. [Local verification receipt](docs/intersession-0.2.0-verification.json) · [Support matrix](docs/maintenance.md#platform-and-harness-coverage)

The Tauri shell hosts the node and shows service health in its tray. The main window contains the starter UI. The mobile replica and iroh storage adapter are libraries. Design materials describe potential desktop and mobile surfaces. [Architecture history](docs/00-architecture-and-implementation-plan.md) · [UI design materials](docs/ui/README.md)

The project uses the MIT license. [License](LICENSE)
