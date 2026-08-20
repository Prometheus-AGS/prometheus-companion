#!/usr/bin/env bash
# audit-progress.sh — AGENTS.md §2: every process skill authored in THIS repo
# declares Progress Signals.
#
# Scope: skills authored here only. Vendored skill copies (installed by
# `openspec init` into .claude/.agents/.kimi-code/.opencode) are third-party
# output; linting them reports on someone else's file, not our compliance.
set -uo pipefail
cd "$(dirname "$0")/.."

VENDORED='^\./(\.claude|\.agents|\.kimi-code|\.opencode)/skills/'

mapfile -t authored < <(
  find . -name 'SKILL.md' -not -path './node_modules/*' 2>/dev/null \
    | grep -vE "$VENDORED" | sort
)

if [[ ${#authored[@]} -eq 0 ]]; then
  echo "audit-progress: PASS (no repo-authored skills)"
  echo "  note: $(find . -name 'SKILL.md' -not -path './node_modules/*' 2>/dev/null | wc -l | tr -d ' ') vendored SKILL.md files skipped — installed by \`openspec init\`, not authored here."
  exit 0
fi

violations=0
for f in "${authored[@]}"; do
  if ! grep -qiE '^#+.*progress signals?' "$f"; then
    echo "✗ $f — §2 requires a Progress Signals section"
    violations=$((violations + 1))
    continue
  fi
  grep -qE 'Starting (phase|task)' "$f" || { echo "✗ $f — Progress Signals section lacks a 'Starting phase/task' example"; violations=$((violations + 1)); }
  grep -qE 'Completed (phase|task)' "$f" || { echo "✗ $f — Progress Signals section lacks a 'Completed phase/task' example"; violations=$((violations + 1)); }
done

if [[ "$violations" -gt 0 ]]; then
  echo "audit-progress: FAIL ($violations violations)"
  exit 1
fi
echo "audit-progress: PASS (${#authored[@]} repo-authored skill(s))"
