#!/usr/bin/env bash
# audit-all.sh — AGENTS.md §19 umbrella. Runs every audit, reports each, and
# exits non-zero if any FAILED.
#
# SKIP is never counted as PASS. A check that could not run is unverified, and
# reporting it as passing is how a gate becomes decorative.
set -uo pipefail
cd "$(dirname "$0")/.."

AUDITS=(layer naming generated tokens a11y deps secret-leak skill-desc progress ui)

pass=0; fail=0; skip=0
failed_names=()

for a in "${AUDITS[@]}"; do
  script="scripts/audit-$a.sh"
  if [[ ! -x "$script" ]]; then
    echo "── audit:$a"; echo "   ✗ missing or not executable: $script"
    fail=$((fail+1)); failed_names+=("$a"); continue
  fi
  echo "── audit:$a"
  out=$(bash "$script" 2>&1); code=$?
  sed 's/^/   /' <<< "$out"
  if [[ $code -ne 0 ]]; then
    fail=$((fail+1)); failed_names+=("$a")
  elif grep -q ': SKIP' <<< "$out"; then
    skip=$((skip+1))
  else
    pass=$((pass+1))
  fi
done

echo
echo "════════════════════════════════════════"
echo "PASS $pass   FAIL $fail   SKIP $skip   (of ${#AUDITS[@]})"
if [[ $skip -gt 0 ]]; then
  echo "SKIP is not PASS — those checks did not run. See each script's header."
fi
if [[ $fail -gt 0 ]]; then
  echo "FAILED: ${failed_names[*]}"
  exit 1
fi
echo "All runnable audits passed."
