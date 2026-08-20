#!/usr/bin/env bash
# install-git-hooks.sh — install repo git hooks into .git/hooks.
# .git/hooks is not version-controlled, so each clone must run this once.
set -euo pipefail
cd "$(dirname "$0")/.."
[[ -d .git ]] || { echo "not a git checkout" >&2; exit 1; }

cat > .git/hooks/pre-push <<'HOOK'
#!/usr/bin/env bash
set -uo pipefail
cd "$(git rev-parse --show-toplevel)" || exit 1
echo "pre-push: running local audit gate (AGENTS.md §19)…"
bash scripts/audit-all.sh || {
  echo; echo "pre-push: BLOCKED — audits failed. Bypass: git push --no-verify"; exit 1; }
HOOK
chmod +x .git/hooks/pre-push
echo "✓ installed .git/hooks/pre-push"
