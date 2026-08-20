#!/usr/bin/env bash
# audit-tokens.sh — AGENTS.md §8/§17: no hardcoded colors, spacing, or radii in
# components; use tokens from theme.css.
#
# STATUS: STUB. The precondition does not exist yet.
#
# §17 forbids "hardcoded colors / spacing / radii in components (use tokens from
# theme.css)". There is no theme.css or tokens.css in this repo, and src/App.css
# carries 24 hardcoded values from the Tauri scaffold. Enabling the real check
# now would fail on a clean checkout — a gate that is red on main stops being
# read, which is how enforcement becomes decorative.
#
# TO ACTIVATE:
#   1. Create the token layer (src/styles/theme.css) per the
#      hybrid-design-tokens skill.
#   2. Migrate src/App.css off its literals.
#   3. Replace this file with the check below and promote
#      `kebab`-style token usage to a blocking constraint.
#
# The real check, for when the precondition holds:
#   grep -rnE '#[0-9a-fA-F]{3,8}|[0-9]+px|rgba?\(' src/presentation --include='*.tsx' --include='*.css'
set -uo pipefail
cd "$(dirname "$0")/.."

token_file=""
for c in src/styles/theme.css src/styles/tokens.css src/theme.css; do
  [[ -f "$c" ]] && token_file="$c" && break
done

if [[ -z "$token_file" ]]; then
  n=$(grep -rcE '#[0-9a-fA-F]{3,8}|[0-9]+px' src --include='*.css' 2>/dev/null | awk -F: '{s+=$2} END{print s+0}')
  echo "audit-tokens: SKIP — no token layer exists"
  echo "  §8/§17 requires tokens from theme.css; no theme.css/tokens.css found."
  echo "  src/**.css currently holds ~${n} hardcoded literal(s)."
  echo "  Create src/styles/theme.css (hybrid-design-tokens skill), migrate, then"
  echo "  replace this stub with the real check documented in its header."
  echo "  exit 0 — this is a SKIP, not a PASS."
  exit 0
fi

echo "audit-tokens: SKIP — token layer found at $token_file but the check is still a stub."
echo "  Replace this file with the real grep now that the precondition holds."
exit 0
