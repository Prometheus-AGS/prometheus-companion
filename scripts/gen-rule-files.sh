#!/usr/bin/env bash
# gen-rule-files.sh — render templates/rules/*.md.tmpl into
# AGENTS.md and CLAUDE.md at the project root.
#
# Both output files carry a `<!-- GENERATED — DO NOT EDIT —>` banner.
# scripts/audit-generated.sh verifies the output matches the
# template; if it does not, the next commit will fail in CI.
#
# Usage:
#   bash scripts/gen-rule-files.sh
#
# Idempotent: re-running produces the same output as long as the
# templates are unchanged.

set -euo pipefail
cd "$(dirname "$0")/.."

TEMPLATE_DIR="templates/rules"
AGENTS_TEMPLATE="${TEMPLATE_DIR}/agents.md.tmpl"
CLAUDE_TEMPLATE="${TEMPLATE_DIR}/claude.md.tmpl"

AGENTS_OUT="AGENTS.md"
CLAUDE_OUT="CLAUDE.md"

[[ -f "$AGENTS_TEMPLATE" ]] || { echo "missing template: $AGENTS_TEMPLATE" >&2; exit 1; }
[[ -f "$CLAUDE_TEMPLATE" ]] || { echo "missing template: $CLAUDE_TEMPLATE" >&2; exit 1; }

# Render — for now, the templates are the source of truth and the
# generator is a copy + banner guard. Future versions may splice
# the version, the date, the projectId, etc.

{
  cat "$AGENTS_TEMPLATE"
} > "$AGENTS_OUT.new"
mv "$AGENTS_OUT.new" "$AGENTS_OUT"
echo "✓ wrote $AGENTS_OUT"

{
  cat "$CLAUDE_TEMPLATE"
} > "$CLAUDE_OUT.new"
mv "$CLAUDE_OUT.new" "$CLAUDE_OUT"
echo "✓ wrote $CLAUDE_OUT"
