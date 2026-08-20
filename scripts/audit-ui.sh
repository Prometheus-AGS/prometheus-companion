#!/usr/bin/env bash
# audit-ui.sh — AGENTS.md §12: screenshot-driven UI review.
#
# STATUS: STUB. No screenshot harness exists.
#
# §12 requires 8 shots per themed surface — 320/768/1024/1440 px wide, in both
# light and dark. Capturing those needs a driver. This repo has no Playwright
# config and no BrowserClaw dependency, so no script here can produce them.
#
# A script that checked only "does a screenshots/ directory exist" would pass
# whenever someone created an empty folder. That is worse than no check: it
# reports green for work that never happened.
#
# TO ACTIVATE:
#   pnpm add -D @playwright/test && pnpm exec playwright install chromium
#   then capture 320/768/1024/1440 x {light,dark} per surface and replace this
#   stub with a check that asserts the expected files exist AND are newer than
#   the newest file under src/presentation.
set -uo pipefail
cd "$(dirname "$0")/.."

harness=""
[[ -f playwright.config.ts || -f playwright.config.js ]] && harness="playwright"
grep -qi 'browserclaw' package.json 2>/dev/null && harness="${harness:+$harness+}browserclaw"

if [[ -z "$harness" ]]; then
  echo "audit-ui: SKIP — no screenshot harness installed"
  echo "  §12 requires 8 shots/surface (320/768/1024/1440 x light|dark)."
  echo "  Neither Playwright nor BrowserClaw is present, so none can be captured."
  echo "  Install a harness, then replace this stub (see header)."
  echo "  exit 0 — this is a SKIP, not a PASS. UI review is currently manual."
  exit 0
fi

echo "audit-ui: SKIP — harness detected ($harness) but the capture loop is still a stub."
echo "  Implement the capture + freshness assertion described in this file's header."
exit 0
