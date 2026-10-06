# Maintenance, rollback and recovery

Maintenance readiness is pending. Source convergence does not establish build,
reproducible install, approved merge/tag, installed artifact or functional
acceptance. The final local phase gate owns those receipts, including two-node
signed-command convergence and pack-without-Companion operation. Missing
historical receipts remain missing.

## Finite support scope

| Platform / harness | Source contract | Release status |
|---|---|---|
| macOS headless | Unix control/P2P and launchd installation/adoption | Pending build, clean install, SIGTERM/crash recovery and peer gates |
| macOS Tauri | Same node, five service-health tray states | Pending packaging; starter main window, no dashboard acceptance |
| Linux Unix headless | Socket/control and optional P2P source | No service installer; supported artifact requires its own local platform gate |
| Claude Code POSIX | User-scope CLI MCP registration and skill links | Pending scratch-config ownership/dry-run and connected functional acceptance |
| Native Windows, other harness installers, mobile app | No supported packaging contract here | Outside release matrix; retained library/design source is not a shipped app |

Freeze exact OS/architecture, harness version, source commits, package/lockfile
and binary hashes. A pending mandatory gate is not maintenance-only. Supervision
uses the actual selected manifest, not a fixed historical service count.

## Compatibility and installation

Companion is separately optional with one-way full-pack dependencies. Source
selects exactly full revision `ba5c4516fd72e60c21ab3bb1d09a63105aeb3c61`; do not
substitute a memory-server SHA or newer pack commit without owner disposition.
Pack contract `1.0.0` appears in substrate/package metadata. Major incompatibility
blocks installation; compatible labels still require real socket/path/manifest/
signing/extension gates. Signed push schema `1.7` is a separate version domain.

Protected `versions.toml`, tags and real harness configuration require owner
approval. Cargo source changes do not reconcile the protected file or lockfile.
Reconcile resolved dependencies and build at the coherent final local boundary,
record hashes, then publish the approved freeze. Do not use local pack path
patches or mutable remote branches as reproducible dependencies.

Initial origin is a private local mirror. Real publication, approved commits,
artifact signing, tags and dependency notices remain release-owner work; no
release download URL exists. [Installation](installation.md) covers separate
service/Claude installers, receipts, private backups and read-only previews.
A normal pack install must not implicitly install or require this extension.

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

Preserve the private `prometheus/companion/mcp-push-outbox` beneath the selected
data root too. It contains exact signed request/endpoint intent created before
MCP sends; recover the original ID from it after a process interruption. Intent
is not a server receipt, successful replay or peer-delivery proof.

Restore the enrolled identities and matching state first. Changing data/XDG/
socket paths can start an empty environment; empty status is not recovery.
Headless preflight checks key-file access only. Authority, replica authorization
and signed continuity still need real gates. Generating a new signer cannot
repair another identity's history.

Pack projection migration is a separate projection-only path in the selected
phase source. Preview:
`prometheus kbd --path /absolute/project migrate --projections --dry-run`.
This must be genuinely read-only. Approved apply uses `--projections` without
`--dry-run`, retains replica lock/authority semantics and does not change signed
journal, `project.loro`, revision or frontier. Matching unmarked projections
are adopted; changed/malformed originals retain exact bytes, hashes and durable
archive provenance before replacement. Unknown phases are archived without
inventing a canonical phase/replacement. Archives are not historical task
closure. Confirm the approved installed pack actually exposes this command;
source in a dirty candidate does not change the owner-selected dependency pin.

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
