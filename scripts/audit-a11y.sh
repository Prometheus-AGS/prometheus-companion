#!/usr/bin/env bash
# audit-a11y.sh — AGENTS.md §11 WCAG 2.2 AA, per-file heuristic.
#
# Scope note: this is a static heuristic, not an a11y audit. Contrast,
# focus order, and live-region correctness need a running app (§12).
set -uo pipefail
cd "$(dirname "$0")/.."

violations=0
report() { echo "✗ $1"; violations=$((violations + 1)); }

if [[ ! -d src ]]; then echo "audit-a11y: PASS (no src/)"; exit 0; fi

# 1. Positive tabindex breaks focus order.
hits=$(grep -rnE 'tabIndex=\{?["'"'"']?[1-9]' src --include='*.tsx' 2>/dev/null || true)
[[ -n "$hits" ]] && while IFS= read -r l; do report "§11 positive tabIndex breaks focus order — $l"; done <<< "$hits"

# 2. outline:none without a replacement indicator in the same file.
while IFS= read -r f; do
  [[ -z "$f" ]] && continue
  if ! grep -qE 'outline-offset|:focus-visible|box-shadow|ring-' "$f" 2>/dev/null; then
    report "§11 outline removed with no visible focus replacement — $f"
  fi
done < <(grep -rlE 'outline:\s*(none|0)' src --include='*.css' --include='*.tsx' 2>/dev/null || true)

# 3. <img> without alt. Match every <img ...> tag, then filter.
while IFS= read -r l; do
  [[ -z "$l" ]] && continue
  tag="${l#*:}"; tag="${tag#*:}"
  grep -qE '\balt=' <<< "$tag" || report "§11 <img> without alt — $l"
done < <(grep -rnoE '<img\b[^>]*>' src --include='*.tsx' 2>/dev/null || true)

# 4. Icon-only <button> with no accessible name: a button whose body holds
#    only an element/whitespace and which lacks aria-label/aria-labelledby/title.
while IFS= read -r l; do
  [[ -z "$l" ]] && continue
  body="${l#*:}"; body="${body#*:}"
  # Body between > and </button> holds only element(s)/whitespace — no text node.
  inner=$(sed -E 's/^<button[^>]*>//; s|</button>$||' <<< "$body")
  stripped=$(sed -E 's/<[^>]*>//g; s/[[:space:]]//g' <<< "$inner")
  if [[ -z "$stripped" ]] && ! grep -qE 'aria-label|aria-labelledby|title=' <<< "$body"; then
    report "§11 icon-only button without accessible name — $l"
  fi
done < <(grep -rnoE '<button\b[^>]*>[^<]*<[^>]*>[[:space:]]*</button>' src --include='*.tsx' 2>/dev/null || true)

# 5. onClick on a non-interactive element without role+tabIndex.
hits=$(grep -rnE '<(div|span)\b[^>]*onClick' src --include='*.tsx' 2>/dev/null || true)
while IFS= read -r l; do
  [[ -z "$l" ]] && continue
  grep -qE 'role=' <<< "$l" || report "§11 clickable <div>/<span> without role — use <button> — $l"
done <<< "$hits"

if [[ "$violations" -gt 0 ]]; then
  echo "audit-a11y: FAIL ($violations violations)"
  echo "  Heuristic only — contrast, focus order, and live regions need §12 review."
  exit 1
fi
echo "audit-a11y: PASS (heuristic; contrast + focus order still need §12 review)"
