#!/usr/bin/env bash
# audit-naming.sh — AGENTS.md §3: every TypeScript file and folder under src/
# is kebab-case. Component *symbols* stay PascalCase; the *files* do not.
#
# Allowed exceptions (framework-mandated): __root.tsx/.ts, index.ts/.tsx
set -uo pipefail
cd "$(dirname "$0")/.."

violations=0

# Files: flag any basename containing an uppercase letter.
while IFS= read -r f; do
  base="${f##*/}"
  case "$base" in
    __root.ts|__root.tsx|index.ts|index.tsx) continue ;;
  esac
  if [[ "$base" =~ [A-Z] ]]; then
    echo "✗ $f — file name must be kebab-case (suggest: $(echo "$base" | sed -E 's/([a-z0-9])([A-Z])/\1-\2/g' | tr '[:upper:]' '[:lower:]'))"
    violations=$((violations + 1))
  fi
done < <(find src -type f \( -name '*.ts' -o -name '*.tsx' \) 2>/dev/null | sort)

# Folders: flag any directory component containing an uppercase letter.
while IFS= read -r d; do
  base="${d##*/}"
  if [[ "$base" =~ [A-Z] ]]; then
    echo "✗ $d/ — folder name must be kebab-case"
    violations=$((violations + 1))
  fi
done < <(find src -type d 2>/dev/null | sort)

if [[ "$violations" -gt 0 ]]; then
  echo "audit-naming: FAIL ($violations violations)"
  exit 1
fi
echo "audit-naming: PASS"
