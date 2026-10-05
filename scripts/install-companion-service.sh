#!/usr/bin/env bash
# Install only ai.prometheus.companion. Does not build binaries or install pack services.
# --dry-run renders XML only: no mkdir, temporary files, validation or launchctl.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TEMPLATE="$REPO_ROOT/shared/launchd/ai.prometheus.companion.plist.tmpl"
LAUNCH_AGENTS_DIR="$HOME/Library/LaunchAgents"
PLIST_OUT="$LAUNCH_AGENTS_DIR/ai.prometheus.companion.plist"
LABEL="ai.prometheus.companion"
BIN="$HOME/.local/bin/prometheus-companion-headless"
PACK_ROOT="${PROMETHEUS_PACK_ROOT:-}"
LOG_DIR="$HOME/.prometheus/logs"
DEVICE_KEY="${PROMETHEUS_DEVICE_KEY_FILE:-}"
DATA_DIR="${PROMETHEUS_DATA_DIR:-}"
SKILLS_DIR="${PROMETHEUS_SKILLS_DIR:-}"
SOCKET="${SOVEREIGN_SYNC_SOCKET:-}"
SYNC_CONFIG="${SOVEREIGN_SYNC_CONFIG:-}"
CONFIG_HOME="${XDG_CONFIG_HOME:-}"
CLAUDE_DIR="${CLAUDE_CONFIG_DIR:-}"
DRY_RUN=0
usage() {
  cat <<'USAGE'
Usage: bash scripts/install-companion-service.sh [--dry-run] [--bin PATH]
  [--pack-root PATH] [--log-dir PATH] [--device-key-file PATH] [--data-dir PATH]
  [--skills-dir PATH] [--socket PATH] [--sync-config PATH]
  [--xdg-config-home PATH] [--claude-config-dir PATH]
All paths are absolute. Optional overrides otherwise use the named environment
variables. Preview is read-only. Apply requires a separately built/approved binary
and existing enrolled signing key; no identity is created by this installer.
USAGE
}
while [ $# -gt 0 ]; do
  case "$1" in
    --bin) BIN="${2:?--bin needs a value}"; shift 2 ;;
    --pack-root) PACK_ROOT="${2:?--pack-root needs a value}"; shift 2 ;;
    --log-dir) LOG_DIR="${2:?--log-dir needs a value}"; shift 2 ;;
    --device-key-file) DEVICE_KEY="${2:?--device-key-file needs a value}"; shift 2 ;;
    --data-dir) DATA_DIR="${2:?--data-dir needs a value}"; shift 2 ;;
    --skills-dir) SKILLS_DIR="${2:?--skills-dir needs a value}"; shift 2 ;;
    --socket) SOCKET="${2:?--socket needs a value}"; shift 2 ;;
    --sync-config) SYNC_CONFIG="${2:?--sync-config needs a value}"; shift 2 ;;
    --xdg-config-home) CONFIG_HOME="${2:?--xdg-config-home needs a value}"; shift 2 ;;
    --claude-config-dir) CLAUDE_DIR="${2:?--claude-config-dir needs a value}"; shift 2 ;;
    --dry-run) DRY_RUN=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "install-companion-service: unknown option: $1" >&2; usage >&2; exit 1 ;;
  esac
done
if [ "$(uname -s)" != "Darwin" ]; then
  echo "install-companion-service: launchd is macOS-only; no Linux/Windows service installer is supplied" >&2
  exit 1
fi
[ -f "$TEMPLATE" ] || { echo "install-companion-service: missing template" >&2; exit 1; }
ENVIRONMENT_JSON=$(node - "$DEVICE_KEY" "$DATA_DIR" "$SKILLS_DIR" "$SOCKET" "$SYNC_CONFIG" "$CONFIG_HOME" "$CLAUDE_DIR" <<'JS'
const keys = ['PROMETHEUS_DEVICE_KEY_FILE', 'PROMETHEUS_DATA_DIR', 'PROMETHEUS_SKILLS_DIR', 'SOVEREIGN_SYNC_SOCKET', 'SOVEREIGN_SYNC_CONFIG', 'XDG_CONFIG_HOME', 'CLAUDE_CONFIG_DIR'];
console.log(JSON.stringify(Object.fromEntries(keys.map((key, index) => [key, process.argv[index + 2]]))));
JS
)
render() {
  node "$SCRIPT_DIR/render-companion-plist.mjs" "$TEMPLATE" "$BIN" "$LOG_DIR" "$HOME" "$PACK_ROOT" "$ENVIRONMENT_JSON"
}
if [ "$DRY_RUN" -eq 1 ]; then
  render
  exit 0
fi
[ -x "$BIN" ] && [ -f "$BIN" ] || { echo "install-companion-service: approved executable required: $BIN" >&2; exit 1; }
if [ -z "$PACK_ROOT" ]; then
  echo "install-companion-service: no pack root selected; service adoption is unavailable, local control remains separate" >&2
fi
# Render before creating any destination; escaping/path failures preserve it.
RENDERED=$(render)
if [ -L "$PLIST_OUT" ] || { [ -e "$PLIST_OUT" ] && [ ! -f "$PLIST_OUT" ]; }; then
  echo "install-companion-service: refusing non-regular plist destination; original preserved" >&2
  exit 1
fi
mkdir -p "$LAUNCH_AGENTS_DIR" "$LOG_DIR"
TEMPORARY=$(mktemp "$LAUNCH_AGENTS_DIR/.ai.prometheus.companion.XXXXXX")
trap 'rm -f "$TEMPORARY"' EXIT
printf '%s\n' "$RENDERED" > "$TEMPORARY"
plutil -lint "$TEMPORARY" >/dev/null
if [ -f "$PLIST_OUT" ]; then
  BACKUP=$(mktemp "$LAUNCH_AGENTS_DIR/.ai.prometheus.companion.backup.XXXXXX")
  cat "$PLIST_OUT" > "$BACKUP"
  echo "install-companion-service: previous plist retained at $BACKUP"
fi
mv -f "$TEMPORARY" "$PLIST_OUT"
launchctl bootout "gui/$(id -u)/$LABEL" >/dev/null 2>&1 || true
launchctl bootstrap "gui/$(id -u)" "$PLIST_OUT"
launchctl kickstart -k "gui/$(id -u)/$LABEL"
echo "install-companion-service: loaded $LABEL; functional acceptance remains separate"
echo "  plist: $PLIST_OUT"
echo "  binary: $BIN"
echo "  logs: $LOG_DIR/companion.stdout.log"
