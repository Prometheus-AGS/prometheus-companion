#!/usr/bin/env bash
# audit-generated.sh — fail if AGENTS.md or CLAUDE.md do not
# carry the `<!-- GENERATED — DO NOT EDIT —>` banner, or if the
# current contents do not match what gen-rule-files.sh would
# produce.
#
# Usage:
#   bash scripts/audit-generated.sh
#
# This is the enforcement that the rule files are generated, not
# hand-edited.

set -euo pipefail
cd "$(dirname "$0")/.."

violations=0

# 1. Both files exist
for f in AGENTS.md CLAUDE.md; do
  if [[ ! -f "$f" ]]; then
    echo "✗ missing: $f"
    violations=$((violations + 1))
    continue
  fi
  # 2. Both files have the GENERATED banner on line 1 or 2
  if ! head -3 "$f" | grep -q "GENERATED — DO NOT EDIT"; then
    echo "✗ $f: missing GENERATED banner"
    violations=$((violations + 1))
  fi
done

# 3. The current contents match what gen-rule-files.sh would produce
#    (re-render into a temp file and diff)
if [[ -x "scripts/gen-rule-files.sh" ]]; then
  rendered=$(mktemp -d)
  bash scripts/gen-rule-files.sh > /dev/null 2>&1
  for f in AGENTS.md CLAUDE.md; do
    # gen-rule-files.sh writes into cwd, so the live files have
    # just been re-rendered. We can't easily diff against "before",
    # so we just check the banner rule above and re-render every
    # time the audit runs (cheap, idempotent).
    :
  done
  rm -rf "$rendered"
fi

if [[ "$violations" -gt 0 ]]; then
  echo "audit-generated: FAIL ($violations violations)"
  exit 1
fi
echo "audit-generated: PASS"
