# Installation and ownership

These procedures describe the source candidate. Supported release procedures
require final local integration, approved artifact/pin freeze and publication.
They have not been executed as acceptance here.

## Prerequisites and paths

Use separately built and approved `prometheus-companion-headless` and
`sovereign-sync` binaries. Record source revisions, binary hashes, lockfile and
selected pack revision. Neither installer builds them or installs pack services.
The substrate and `skill-package.json` require pack contract `1.0.0`; version
fields alone do not prove installed interoperability.

| Selection | Resolution |
|---|---|
| Journal/registry data root | `PROMETHEUS_DATA_DIR`, otherwise platform data-local directory |
| Control socket | `SOVEREIGN_SYNC_SOCKET`, otherwise `<data-local-dir>/prometheus/run/sovereign-sync.sock`; standalone `--socket` takes precedence |
| Headless/windowed skills | `PROMETHEUS_SKILLS_DIR`, then `CLAUDE_CONFIG_DIR/skills`, then `~/.claude/skills` |
| Sync configuration | `SOVEREIGN_SYNC_CONFIG`, then `XDG_CONFIG_HOME/sovereign-sync/config.toml`, then `~/.config/sovereign-sync/config.toml`; standalone `--config` takes precedence |
| Device signing key | `PROMETHEUS_DEVICE_KEY_FILE`, then managed XDG-style `sovereign-sync/device-key.json`, then credential store in interactive contexts |
| P2P identity | Configuration `node.p2p_identity_file`; default XDG-style `sovereign-sync/p2p-identity.json` |
| Pack manifest | Explicit `PROMETHEUS_PACK_ROOT/shared/services.manifest.json` |

`PROMETHEUS_DATA_DIR` does **not** relocate the socket or identity files. Set
each override consistently on the host and clients. Headless/windowed hosts
use the environment skill resolution; standalone modes read `node.skills_dir`
from configuration. Missing explicit configuration is an error; an absent
default configuration uses defaults. No invalid explicit identity is rescued
by choosing a different signer.

Headless startup requires a readable, regular, non-symlink key file excluding
group/other access (normally `0600`). Preflight checks access, not cryptographic
validity or project enrollment; authority must still open. Authorizing a
windowed binary's credential-store prompt does not authorize the headless binary.
Restore an existing project's enrolled key or select it explicitly.
`sovereign-sync --mode init` provisions local files for a new installation or
approved recovery; creating a new key cannot recover another signer's history.

## macOS headless service

Preview from this source directory using absolute paths:

```bash
bash scripts/install-companion-service.sh --dry-run \
  --bin /absolute/approved/prometheus-companion-headless \
  --pack-root /absolute/installed/pack \
  --device-key-file /absolute/enrolled/device-key.json
```

Preview reads the template and renders escaped XML to stdout. It creates no
directories, temporary files, identities, plists or services and invokes neither
`plutil` nor `launchctl`. The binary may be absent during preview. For an approved
installation, apply the same arguments without `--dry-run`; the executable must
already exist.

The installer owns `~/Library/LaunchAgents/ai.prometheus.companion.plist` and
`gui/<uid>/ai.prometheus.companion`. It validates a staged plist, retains an exact
private sibling backup of the previous plist, replaces only that service label
and requests startup. It preserves explicit key/data/socket/skills/config scope
overrides. Other flags are `--log-dir`, `--data-dir`, `--skills-dir`, `--socket`,
`--sync-config`, `--xdg-config-home` and `--claude-config-dir`; all paths are
absolute. This script does not claim ownership of unrelated service labels.

The template uses `RunAtLoad`, conditional `KeepAlive`, `ProcessType=Interactive`,
`ThrottleInterval=15` and separate stdout/stderr logs. Loading it does not prove
crash recovery or signing access. A missing/unreadable pack manifest leaves
local control independent, with no attached service supervision or tray health
subscription. No guessed pack path is used. Linux has no supplied service
installer; native Windows is outside this Unix/launchd contract.

## Claude Code connected skills

Preview this repository's own optional skill package:

```bash
bash scripts/install-skill-package.sh --dry-run \
  --harness claude-code \
  --bin /absolute/approved/sovereign-sync \
  --claude-bin /absolute/installed/claude
```

Apply without `--dry-run` after reviewing ownership. The installer uses
`claude mcp add-json ... --scope user` and removes a same-name entry only when
its ownership receipt matches. It does not write obsolete `~/.claude/mcp.json`.
Current Claude user configuration is `~/.claude.json`, or `.claude.json` inside
`CLAUDE_CONFIG_DIR` when overridden. This follows the [official MCP docs](https://code.claude.com/docs/en/mcp)
and [configuration environment docs](https://code.claude.com/docs/en/settings).

The named `sovereign-sync` registration launches the explicit installed binary
with `--mode mcp`. Symlinks into `<Claude-config-dir>/skills` reference this
repository: retain that source path or register an approved replacement before
removing it. The registration records the selected skill root and explicit
connection/config path environment overrides, including `SOVEREIGN_SYNC_SOCKET`.
Set the same selected endpoint and runtime/key scope used by the host before
registration. Reload the harness afterward. Registration does not start a host.

Private ownership and backups live under
`<Claude-config-dir>/.prometheus-companion/skill-package-receipt.json`. Matching
receipt-owned entries may be replaced; matching unowned entries remain unowned.
Non-symlink skill content, unknown/edited registrations, malformed JSON and
symlink ancestors are refused. Other MCP names are checked for preservation.
Same-name local/project registrations can override user scope; reconcile them
separately rather than removing another owner's configuration.

Stop concurrent configuration writers. The private lock serializes this
installer, not unrelated Claude processes. Failure retains a pending receipt
and exact private backup, and attempts to restore only its removed entry if
still absent. It never restores a whole config over unrelated edits. Retry the
same artifact/configuration or reconcile ownership first. A crashed installer's
lock or orphaned temporary symlink needs owner inspection after confirming no
installer remains alive; there is no automatic stale-lock takeover. Dry-run
performs no CLI calls, locks, backups, links or directory writes. Other harness
installers and native Windows registration are not implemented.

## Optional peers

Disable network transport explicitly in the selected configuration:

```toml
[node]
p2p_enabled = false
```

Enabled transport uses the selected identity, bootstrap endpoint IDs and
discovery settings on every supervisor retry. Default mDNS finds addresses on
the local link; it neither invents endpoint IDs nor admits unknown peers. DHT
is off by default. `[discovery] dht = true` publishes a signed endpoint/relay/
address record to the public DHT. iroh relay behavior is separate; disabling
DHT does not promise no relay traffic.

To enroll A and B: A exports with `sovereign-sync --config /absolute/config.toml
--mode pair-export`; transfer its secret ticket privately to B. B imports using
`--mode pair-import --ticket <private-ticket>`, then exports its updated ticket
back to A for import. This order keeps the group secret shared. Import replaces
the selected identity's group secret and updates its allowlist; back up existing
membership first. Tickets passed as CLI arguments may appear in history/process
listings: protect that session, and never paste them into issues or shared logs.

Add each authorized endpoint ID to the other node's `[peers] bootstrap` list,
then restart the one selected host to read the updated identity/configuration.
mDNS supplies changing addresses for those known IDs. The source offers a
pairing URI payload, not a shipped QR screen, short-code protocol or group UI.
Peers use Ed25519 endpoint authentication over iroh QUIC/TLS, without the
architecture draft's additional Noise handshake. KBD project/replica signing
authorization is separate; network pairing never grants arbitrary mutation.
