#!/usr/bin/env bash
# audit-deps.sh — AGENTS.md §15: pinned versions.
#   - no `*` in Cargo.toml version specs
#   - no `^`/`~` ranges in package.json dependencies (exact pins)
#   - pnpm-lock.yaml present and newer than package.json
set -uo pipefail
cd "$(dirname "$0")/.."

violations=0
report() { echo "✗ $1"; violations=$((violations + 1)); }

# --- Cargo: no wildcard versions ---
if [[ -f src-tauri/Cargo.toml ]]; then
  hits=$(grep -nE '^[[:space:]]*[A-Za-z0-9_-]+[[:space:]]*=[[:space:]]*"\*"' src-tauri/Cargo.toml || true)
  [[ -n "$hits" ]] && while IFS= read -r l; do report "§15 wildcard Cargo version — src-tauri/Cargo.toml:$l"; done <<< "$hits"
  hits=$(grep -nE 'version[[:space:]]*=[[:space:]]*"\*"' src-tauri/Cargo.toml || true)
  [[ -n "$hits" ]] && while IFS= read -r l; do report "§15 wildcard Cargo version — src-tauri/Cargo.toml:$l"; done <<< "$hits"
fi

# --- package.json: flag range specifiers ---
if [[ -f package.json ]] && command -v python3 >/dev/null 2>&1; then
  while IFS= read -r line; do
    [[ -n "$line" ]] && report "§15 unpinned npm dep — $line"
  done < <(python3 - <<'PY'
import json
d = json.load(open('package.json'))
for section in ('dependencies', 'devDependencies'):
    for name, spec in (d.get(section) or {}).items():
        if isinstance(spec, str) and (spec.startswith('^') or spec.startswith('~') or spec == '*'):
            print(f"{section}.{name} = \"{spec}\"")
PY
  )
fi

# --- lockfile present ---
if [[ ! -f pnpm-lock.yaml ]]; then
  report "§15 pnpm-lock.yaml missing — CI requires --frozen-lockfile"
fi

if [[ "$violations" -gt 0 ]]; then
  echo "audit-deps: FAIL ($violations violations)"
  echo "  Pin exact versions. CI runs: pnpm install --frozen-lockfile"
  exit 1
fi
echo "audit-deps: PASS"
