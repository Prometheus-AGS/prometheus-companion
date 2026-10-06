#!/usr/bin/env bash
# Register Companion-owned skills and user-scope MCP through Claude Code's CLI.
# No direct writes to obsolete mcp.json/mcp-servers.json. --dry-run is read-only.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec node "$SCRIPT_DIR/install-skill-package.mjs" "$@"
