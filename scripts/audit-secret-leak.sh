#!/usr/bin/env bash
# audit-secret-leak.sh — AGENTS.md §13: secrets live in the OS secure store.
# No key literals in source; no committed .env.
set -uo pipefail
cd "$(dirname "$0")/.."

violations=0
report() { echo "✗ $1"; violations=$((violations + 1)); }

SCAN_DIRS=()
[[ -d src ]] && SCAN_DIRS+=(src)
[[ -d src-tauri/src ]] && SCAN_DIRS+=(src-tauri/src)

if [[ ${#SCAN_DIRS[@]} -gt 0 ]]; then
  # High-confidence provider key shapes. Deliberately narrow: a broad
  # /secret.*=/ pattern fires on variable names and trains people to ignore it.
  patterns=(
    'sk-[A-Za-z0-9]{20,}'                   # OpenAI-style
    'sk-ant-[A-Za-z0-9_-]{20,}'             # Anthropic
    'ghp_[A-Za-z0-9]{36}'                   # GitHub PAT
    'gho_[A-Za-z0-9]{36}'
    'github_pat_[A-Za-z0-9_]{50,}'
    'AKIA[0-9A-Z]{16}'                      # AWS access key id
    'xox[baprs]-[A-Za-z0-9-]{10,}'          # Slack
    'AIza[0-9A-Za-z_-]{35}'                 # Google API key
    '-----BEGIN [A-Z ]*PRIVATE KEY-----'    # PEM private key
    'eyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}'  # JWT
  )
  for p in "${patterns[@]}"; do
    hits=$(grep -rnE "$p" "${SCAN_DIRS[@]}" 2>/dev/null || true)
    [[ -n "$hits" ]] && while IFS= read -r l; do report "§13 possible secret literal — $l"; done <<< "$hits"
  done

  # Assignment of a long opaque literal to a key-ish identifier.
  hits=$(grep -rnE '(api[_-]?key|apikey|access[_-]?token|client[_-]?secret|auth[_-]?token)[[:space:]]*[:=][[:space:]]*["'"'"'][A-Za-z0-9_\-]{20,}["'"'"']' "${SCAN_DIRS[@]}" 2>/dev/null || true)
  [[ -n "$hits" ]] && while IFS= read -r l; do report "§13 hardcoded credential — $l"; done <<< "$hits"
fi

# .env must not be tracked (needs git; skip cleanly without it).
if command -v git >/dev/null 2>&1 && git rev-parse --git-dir >/dev/null 2>&1; then
  while IFS= read -r f; do
    [[ -n "$f" ]] && report "§13 committed env file — $f"
  done < <(git ls-files | grep -E '(^|/)\.env(\..*)?$' | grep -v '\.example$' || true)
else
  echo "  note: not a git checkout — skipped the committed-.env check"
fi

if [[ "$violations" -gt 0 ]]; then
  echo "audit-secret-leak: FAIL ($violations violations)"
  echo "  Secrets belong in the OS secure store (Stronghold/Keychain/Credential Manager)."
  echo "  If a real secret was committed, ROTATE IT — removing the line is not enough."
  exit 1
fi
echo "audit-secret-leak: PASS"
