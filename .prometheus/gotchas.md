# gotchas

Append-only. Dated entries. Mark superseded entries; do not delete them.

## 2026-08-20
- Initialized by prometheus-context-bootstrap.

## tier-guard.sh reads `.status`, not `.phase`

`bootstrap.sh` writes a waypoint containing only `.phase`, with a note saying
tier-guard reads `.phase` and to "set it to milestone before Tier 3."

Both halves are wrong. `.claude/hooks/tier-guard.sh:39` reads `.status`. The
bootstrap waypoint had no `.status` key at all, so `jq -r '.status // ""'`
returned empty, and Tier 3 was blocked unconditionally with no reachable unblock
path — the exact dead-gate the hook's own header comment (lines 9-14) says an
earlier version had.

Fixed at kbd-init by adding `"status": "running"` and correcting the note.
Verified both directions: status=running blocks `cargo build --release` (exit 2);
`PROMETHEUS_TIER3=1` allows it (exit 0).

**If you re-run `bootstrap.sh`, it will overwrite this file and reintroduce the
bug.** Re-apply the `.status` key afterward.

## AGENTS.md §19 advertises 9 audit scripts that do not exist

§19 lists a `pnpm audit` umbrella of ten `audit:*` scripts and says a pre-push
hook refuses to push if any fails. Only `scripts/audit-generated.sh` exists, and
`package.json` has no `audit` key at all — so `pnpm audit` does not resolve to
them. §3, §4, §5, §11, §12, §13, §15 each cite one of the missing scripts as
their enforcement.

Consequence: those rules are prose only. `.kbd-orchestrator/constraints.md`
carries inline `check:` greps as substitutes, but a grep is a heuristic — §4's
layer rule in particular cannot be reduced to one.

## src/App.tsx violates the repo's own §3 kebab-case rule

Tauri scaffold default. Should be `src/app.tsx`. Recorded as a warning rather
than a blocking constraint, because a gate that fails on a clean checkout is
broken, not strict. Promote to blocking after the rename.
