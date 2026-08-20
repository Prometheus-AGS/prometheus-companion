#!/usr/bin/env bash
# audit-layer.sh — AGENTS.md §4 CLEAN layer rule + §5 Tauri surface gating.
#
# Dependencies point inward:
#   Domain         imports NOTHING outside Domain
#   Application    must not import Infrastructure or Presentation
#   Infrastructure must not import Presentation
#   Presentation   must not import Infrastructure directly
#
# Plus the §17 banned-import list and the §5 presentation restrictions.
set -uo pipefail
cd "$(dirname "$0")/.."

violations=0
report() { echo "✗ $1"; violations=$((violations + 1)); }

# --- Layer rule. Only runs where the layer actually exists. ---
# Matches import/export/require/dynamic-import referencing another layer.
layer_re='(from|import|require)\s*\(?\s*['"'"'"][^'"'"'"]*(\.\./)*'

check_layer() {           # $1 = layer dir, $2 = forbidden layer, $3 = human message
  [[ -d "src/$1" ]] || return 0
  local hits
  hits=$(grep -rnE "['\"][^'\"]*\b$2/" "src/$1" --include='*.ts' --include='*.tsx' 2>/dev/null || true)
  if [[ -n "$hits" ]]; then
    while IFS= read -r line; do report "$3 — $line"; done <<< "$hits"
  fi
}

# Domain imports nothing from other layers, and no framework.
if [[ -d src/domain ]]; then
  for forbidden in application infrastructure presentation; do
    check_layer domain "$forbidden" "§4 Domain must import NOTHING outside Domain"
  done
  hits=$(grep -rnE "from ['\"](react|@tauri-apps|@prometheus-ags|zustand)" src/domain --include='*.ts' --include='*.tsx' 2>/dev/null || true)
  [[ -n "$hits" ]] && while IFS= read -r l; do report "§4 Domain must not import a framework — $l"; done <<< "$hits"
fi

check_layer application  infrastructure "§4 Application must not import Infrastructure"
check_layer application  presentation   "§4 Application must not import Presentation"
check_layer infrastructure presentation "§4 Infrastructure must not import Presentation"
check_layer presentation infrastructure "§4 Presentation must not import Infrastructure directly — go through Application"

# --- §5 / §17: presentation components must not call Tauri directly. ---
if [[ -d src/presentation/components ]]; then
  hits=$(grep -rnE '\b(invoke|listen)\s*\(' src/presentation/components --include='*.ts' --include='*.tsx' 2>/dev/null || true)
  [[ -n "$hits" ]] && while IFS= read -r l; do report "§5 no invoke()/listen() in presentation/components — use an application hook — $l"; done <<< "$hits"
fi

# --- §17 banned imports, repo-wide under src/. ---
ban() {                   # $1 = pattern, $2 = message
  local hits
  hits=$(grep -rn "$1" src --include='*.ts' --include='*.tsx' 2>/dev/null || true)
  [[ -n "$hits" ]] && while IFS= read -r l; do report "$2 — $l"; done <<< "$hits"
  return 0
}
ban '@radix-ui/'            "§8/§17 no @radix-ui/* — use @base-ui-components/react"
ban '@tanstack/react-query' "§9/§17 no @tanstack/react-query — use PEM 3.x"

if [[ "$violations" -gt 0 ]]; then
  echo "audit-layer: FAIL ($violations violations)"
  exit 1
fi
echo "audit-layer: PASS"
