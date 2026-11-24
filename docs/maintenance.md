# Maintenance, rollback and recovery

## Platform and harness coverage

| Surface | Current operating evidence |
|---|---|
| macOS ARM64 headless | Companion 0.2.0 built and ran locally. Paired processes with separate keys and sockets transferred signed history, caught up after a restart and recovered from a stale checkout projection. The clean-machine LaunchAgent lifecycle remains deferred. |
| Full pack without a host | The local process check stopped its isolated host and successfully read project status using the pack CLI. |
| Connected Claude Code skills | The selected-host stdio MCP integration exercised status, signed push, receipt lookup and replay. Installation into a clean Claude user profile remains a separate check. |
| macOS Tauri | The desktop source hosts the node and reports five tray health states. Desktop packaging and screens have separate acceptance. |
| Linux, Windows and mobile | Source includes Unix headless components and an embedded replica library. The Companion package supplies its macOS LaunchAgent and Claude POSIX installers. Platform-specific service installation elsewhere requires separate provisioning and verification. |

The verification record identifies the source base, dependency lock and SHA-256 hashes of the observed programs. Use these to compare a specific installation with the tested build. [Companion 0.2.0 receipt](intersession-0.2.0-verification.json)

## Compatibility and installation

Companion 0.2.0 consumes Full v1.17.0 at `acd029107533c07a661a7e3ad90eb28834688afa`. The dependency selection in `Cargo.toml` and `Cargo.lock` is independent of the minimum pack extension contract (`1.0.0`) and signed HTTP push schema (`1.7`). Check each at its own boundary. Use the selected pack’s actual service manifest for supervision.

The Companion source lives in a private GitHub repository. Build using the locked workspace, record the resulting executable hashes, and review both installer previews. The pack release archives and container catalogs have their own product scope. [Installation](installation.md)

## Update and rollback

Record approved old/new binaries, selected configuration and runtime roots.
Stop the one socket-owning host and wait for shutdown before replacement. Keep
the previous executable at a known separate path, exact plist backup, installer
receipt/private config backups and a consistent signed-state copy. Install the
approved artifact without changing keys/project IDs. Review previews first.

Receipt ownership permits replacing only matching owned entries/links, never
edited or unowned content. Reconcile failed/pending receipts and keep unrelated
settings. An installer lock cannot serialize external config writers. Stale
locks require owner inspection, not age-based deletion. Only remove a recorded
owned symlink if its target still matches; preserve real directories or edits.

Stop only `ai.prometheus.companion` for binary/plist rollback, restore the old
approved executable/plist/environment, then restart that host. Retain failed
artifacts/logs/operation identities. Code rollback is not canonical-state
rollback: never replace newer signed history or successful receipts with an old
backup to make an earlier binary appear healthy. If wire/storage compatibility
prevents safe rollback, stop and use owner-reviewed compatible recovery.

## Data and identity recovery

Back up project `.prometheus/project.json`, signed journal, `project.loro`,
canonical revision/frontier, receipts/checkpoints/run history and selected
registry/data root. Keep enrolled device/P2P keys, group/allowlist and tokens
private, separate from public receipts. Record paths/hashes without exposing
secret contents. Stop writers for a consistent copy. Status/screenshots cannot
be converted into missing signed receipts.

For a project-level recovery copy, first stop the selected writer and any process that can change this project. Set `PROJECT_ROOT` to its checkout, `PROJECT_ID` to the registered UUID and `DATA_ROOT` to that host’s resolved data root. Set `BACKUP_PARENT` to an existing private directory outside those roots. These archives contain signed project history and should receive the same access controls as the originals.

```bash
(
  set -eu
  umask 077
  BACKUP_DIR=$(mktemp -d "${BACKUP_PARENT:?}/kbd-recovery.XXXXXXXX")
  tar -C "${DATA_ROOT:?}/prometheus/kbd/projects" -cf "$BACKUP_DIR/authority.tar" "${PROJECT_ID:?}"
  tar -C "${PROJECT_ROOT:?}" -cf "$BACKUP_DIR/checkout.tar" .prometheus/project.json .kbd-orchestrator/current-waypoint.json
  (cd "$BACKUP_DIR" && shasum -a 256 authority.tar checkout.tar > SHA256SUMS && shasum -a 256 -c SHA256SUMS)
  printf '%s\n' "$BACKUP_DIR"
)
```

The two archives capture this project's authority and checkout identification. Preserve the registry, original receipts, outbox, sync configuration and enrolled key material through the host's private backup procedure as well. During a restore rehearsal, verify the archive checksums, extract into empty staging directories, compare project ID and replica membership, then read the signed audit and frontier through an isolated socket before switching any active writer. This project-copy example has no automatic full-machine restore step.

Preserve the private `prometheus/companion/mcp-push-outbox` beneath the selected
data root too. It contains exact signed request/endpoint intent created before
MCP sends; recover the original ID from it after a process interruption. Intent
is not a server receipt, successful replay or peer-delivery proof.

Restore the enrolled identities and matching state first. Changing data/XDG/
socket paths can start an empty environment; empty status is not recovery.
Headless preflight checks key-file access only. Authority, replica authorization
and signed continuity still need real gates. Generating a new signer cannot
repair another identity's history.

If B already has the signed event but its checkout appears stale, compare the project's status with B’s `.kbd-orchestrator/current-waypoint.json`. The waypoint's `sourceRevision` and `frontier` should describe B's current authority. Resolve directory permissions or an obstructing file first. On that same receiver, request a projection refresh and read the waypoint again.

```bash
curl --fail --silent --show-error --unix-socket "$B_SOCKET" --request POST \
  "http://localhost/api/v1/kbd/projects/$PROJECT_ID/projections/refresh"
```

A projection refresh reconstructs the checkout view from the signed authority. Recheck the audit and original request receipt. A missing event requires delivery or an authority recovery, while a stale waypoint can be rebuilt without sending the command again. For an older binary that cannot read newer signed history, preserve the originals and return to a compatible reader.

Headless shutdown handles SIGTERM/interrupt, stops HTTP acceptance and P2P,
drains the incoming authority consumer and joins the listener with bounded
waits. Failure/timeout is an error, not a clean-drain receipt. Drop requests
cancellation but cannot await thread completion; desktop currently uses that
fallback rather than an accepted awaited UI exit procedure. After interruption,
preserve the signed request ID/body, original host, receipts and frontier. Use
[receipt lookup](control-api.md) before an uncertain push replay; never resolve
response loss by inventing a new operation ID.

## Security, triage and ownership

Keep keys/tokens private and restrict the socket to its intended user. Expose
TCP deliberately with authentication. Ticket import replaces the group secret;
back up membership and enroll only intended peers. Discovery/network pairing
does not authorize canonical mutation. DHT publication is opt-in. Surreal memory
is local-only.

The release owner approves dependency/compatibility updates. Review security
advisories for the actual frozen graph, prepare coherent source, then run local
production-boundary integration before shipping. Hosted testing, isolated unit/
mock results and a merged source branch do not establish acceptance.

Triage separates listener liveness, authority readiness, peer state, supervisor
health and installed provenance. Retain source/binary identities, platform/
harness, contract, original request ID and redacted expected/observed behavior.
Escalate identity, journal-integrity, data-loss or privacy faults before further
mutation. UI/mobile roadmap remains separate; advertised control/install/peer/
rollback behavior with a missing mandatory gate remains release-blocking.
