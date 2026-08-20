#!/usr/bin/env bash
# audit-skill-desc.sh — AGENTS.md §19 lists this as "description rules (§7.1)".
#
# STATUS: STUB. Two blockers.
#
# 1. DANGLING REFERENCE. §19 cites §7.1, but §7 is "Mobile navigation" and has
#    no subsection 7.1. There is no description rule in AGENTS.md to enforce.
#    Either add the rule or drop the script from §19.
#
# 2. NOTHING IN SCOPE. Every SKILL.md here (40 files) was installed by
#    `openspec init` into .claude/, .agents/, .kimi-code/, .opencode/. They are
#    vendored third-party output. Linting them reports on OpenSpec's authoring
#    choices, not this repo's compliance, and would fail on style we do not own.
#
# TO ACTIVATE: write the description rule as a real numbered section in
# AGENTS.md, author at least one skill in this repo, then implement the check
# over repo-authored skills only (see audit-progress.sh for the vendored-path
# exclusion pattern).
set -uo pipefail
cd "$(dirname "$0")/.."

VENDORED='^\./(\.claude|\.agents|\.kimi-code|\.opencode)/skills/'
authored=$(find . -name 'SKILL.md' -not -path './node_modules/*' 2>/dev/null | grep -vcE "$VENDORED" || true)
vendored=$(find . -name 'SKILL.md' -not -path './node_modules/*' 2>/dev/null | grep -cE "$VENDORED" || true)

echo "audit-skill-desc: SKIP — no rule to enforce and nothing in scope"
echo "  §19 cites §7.1, which does not exist (§7 is Mobile navigation)."
echo "  repo-authored skills: ${authored:-0}   vendored (openspec init): ${vendored:-0}"
echo "  exit 0 — this is a SKIP, not a PASS."
exit 0
