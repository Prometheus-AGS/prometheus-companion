# Prompt 06 · Verification

> Run before claiming any phase complete. Read `docs/ui/60-acceptance-tests.md`.

---

Invoke: `hybrid-runtime-verification`, `tauri-ui-review`, `a11y-gate`,
`karpathy-progress-memory`.

"Working" is a runtime claim. Compiling is not evidence. A passing type-check is not
evidence. Your own description of the UI is not evidence.

## What closes a gate

For the phase you are on, walk its table in `60-acceptance-tests.md` and produce, per row,
one of: a passing test name, a screenshot path, or a recorded device/run artifact. A row
with none of those is an open row, and one open row means the gate is open.

Report it as a table: row, evidence, pass/fail. Do not summarize as "all green".

## Specific things a generic pass misses

- **Contrast on dark surfaces.** A muted token that is correct in light mode can land at
  2.6:1 on a dark panel. Audit the dark theme separately, and audit **value maps in `.ts`
  and `.js` data files**, not only style attributes — a banned colour has survived two
  eliminations in this codebase by living in a shared level-colour map rather than in any
  component's markup.
- **Layout shift under streaming.** Measure with `getBoundingClientRect` before and after a
  token batch. Do not eyeball it.
- **The slideout crossover.** Verify the push-to-overlay transition at 1100 px, not just the
  two extremes.
- **Every component in every action state.** Six states, twelve components. A component that
  can only draw the happy path is unfinished.
- **Idempotence, by replay.** Feed the same 200 events twice and diff the projection. This
  catches what a unit test on a single reducer does not.
- **Resolver determinism.** 100 runs, identical candidate set and order.
- **The stale-answer path.** Answer a resolution twice; the second must be rejected, not
  re-run.

## Then record it

Use `karpathy-progress-memory` to write a compact, evidence-backed progress record at the
phase boundary: what closed, what the evidence was, what you learned that the spec did not
say, and what you would do differently. Phase boundaries and verification discoveries are
exactly what that skill is for.

## And tell me what is wrong

Last: list anything you found in the spec folder that is wrong, ambiguous, or contradicted
by the codebase. A spec that survived a phase unchallenged usually means it was not read.
