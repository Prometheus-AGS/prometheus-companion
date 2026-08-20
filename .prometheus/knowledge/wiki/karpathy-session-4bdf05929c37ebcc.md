---
type: SessionRecord
id: karpathy-session-4bdf05929c37ebcc
title: Karpathy session 4bdf05929c37
tags:
- karpathy
- session-learning
sources:
- session:7993178a-b855-4487-9a9a-0143daef704e
timestamp: 2026-08-20T08:19:11.603986+00:00
created_at: 2026-08-20T08:19:11.603986+00:00
updated_at: 2026-08-20T08:19:11.603986+00:00
revision: 0
---

## Delta

Bootstrap applied. `mixed` profile, all assets installed, both audit gates reconciled.

**The one substantive deviation from the default plan:** the managed region went into `templates/rules/*.md.tmpl`, not into `AGENTS.md`/`CLAUDE.md`. Your `gen-rule-files.sh` renders by whole-file copy and `audit-generated.sh` invokes it, so the default SPLICE would have been erased by the next `pnpm audit` — silently, no error. Verified the fix: two consecutive renders, one well-formed marker pair, outputs byte-identical to templates, idempotent, `audit-generated: PASS`.

**Installed:**

| | |
|---|---|
| Hooks | `tier-guard.sh`, `single-writer.sh`, `sycophancy-gate.sh`, `reanchor.sh` — all executable, tier-guard wired |
| Rules | `.claude/rules/typescript.md` (path-scoped, loads on file read) |
| Agent | `.claude/agents/artifact-critic.md` |
| Settings | `.claude/settings.json` — clean CREATE, no merge risk; global settings untouched |
| Memory | `.prometheus/{session-log,decisions,gotchas}.md`, `postmortems/`, `model-fleet.md` |
| Position | `.kbd-orchestrator/current-waypoint.json` (phase=spec) |
| Pins | `versions.toml` stub |

`verify.sh`: **PASS 10, FAIL 2, WARN 2**.

**The 2 FAILs are word-count ceilings, and the number is misleading.** `verify.sh` counts all 4271 words of `AGENTS.md` as managed. The bootstrap-owned region is **1388 words — under the 1500 ceiling**. The overage is your pre-existing §0–§20 prose (2883 words), which the skill deliberately refuses to shrink.

But there's a real finding under the bad metric: you now have **two constitutions in resident context**. The region's "Evidentiary standard" restates your §0.2, "Evidence over assertion" restates §0.6, "Scope" restates §0.4 — same substance, different wording. That's precisely the adherence-degrading overlap the skill's own migration path exits 2 to avoid. It needs a human reconciliation pass, and per the skill's procedure: `/context` before and after, then re-run a fixed task set and compare **pass rate**, not feel. A reduction that lowers tokens and lowers pass rate is a regression. Recorded as open in `.prometheus/decisions.md`.

**The 2 WARNs you can't fix here.** Skill budget is **43.1x over** machine-wide (2436 skills, ~172k tokens vs ~4000 budget) — your repo contributes 1510 chars of that. Raising `skillListingBudgetFraction` won't touch a multiple that size; gating plugins is the lever. Also 9 skills have empty descriptions and can't auto-trigger.

Two things to know going forward:

1. **Don't edit the region in `AGENTS.md`** — it's generated. Edit the template region, or re-run `bootstrap.sh` and re-apply the relocation. A plain bootstrap re-run will splice into the outputs again and get wiped.
2. Bootstrap put `@AGENTS.md` on line 1 of `CLAUDE.md`, above the GENERATED banner. `audit-generated.sh` uses `head -3`, so the banner is still caught at line 3 — it passes, but the margin is one line. Worth knowing if that check ever tightens.

Next per the skill: `/kbd-init` for `project.json` + `constraints.md`, then `/doctor` to confirm no skill descriptions dropped.

## Root Cause

No explicit root-cause section was captured; preserve this as a session record, not an inferred diagnosis.

## Corrective Actions

Review and promote only reusable findings.

## Session Metadata

- Harness: claude-code
- Session: 7993178a-b855-4487-9a9a-0143daef704e
- Captured: 2026-08-20T08:19:03.170940Z
- Project: /Users/gqadonis/Projects/prometheus/prometheus-companion

## Changed Paths

- No changed paths detected.
