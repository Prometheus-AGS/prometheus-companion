# Prompt 04 · The Task Resolver

> Phase 3. The hard one. Read `docs/ui/30-task-resolution-spec.md` in full before starting.

---

Invoke before writing code: `agent-runtime-security`, `domain-glossary-service`,
`client-rag`, `persona-scoped-agent`.

Implement prompt-to-task resolution: a sentence becomes exactly one governed task, or it
becomes a question. Nothing in between.

## What you are building

Five stages, all in Rust, all deterministic given the same manifest and thresholds:
normalize → retrieve → score → verdict → plan.

The resolver produces a `RunPlan`. **It does not execute.** It does not route providers or
decide policy. Its output goes to the gateway, which authorizes it, and UAR runs it — the
same path a button press takes.

## The five verdicts

Evaluated in order, first match wins. Thresholds are named constants in
`resolver/policy.rs`: `TAU_FLOOR` 0.20, `TAU_LOW` 0.45, `TAU_HIGH` 0.72, `DELTA` 0.15.

| Verdict | Condition | Surface |
|---|---|---|
| `no_task` | top1 < `TAU_FLOOR` | prose, no capability asserted |
| `low_confidence` | top1 < `TAU_LOW` | `ClarifyPrompt` + nearest 3, unranked |
| `ambiguous` | top1 ≥ `TAU_LOW`, margin < `DELTA` | `CandidateChoice` |
| `underspecified` | top1 ≥ `TAU_HIGH`, required params unbound | `ParamForm`, unbound params only |
| `resolved` | top1 ≥ `TAU_HIGH`, margin ≥ `DELTA`, params bound | `RunPlan` |

A lone mid-confidence candidate — above `TAU_LOW`, below `TAU_HIGH`, healthy margin — is
`ambiguous` with one candidate. It renders as a confirm-this-one card. It does not run
silently. A confident resolver does not earn a skipped gate: `resolved` with
`effectClass: privileged` still passes the approval gate.

## Disambiguation is a state machine, not a message

This is the part that is usually built wrong: as a chat message the model writes and then
re-parses. Build it as state.

1. Persist a `PendingResolution` — `resolutionId`, prompt, glossary version, candidate set
   with scores, threshold set, verdict, `createdAt`.
2. Emit `resolution.required`. The A2A task state becomes `input-required`. The run is
   **not** failed and **not** complete.
3. The client renders the given order. It does not re-score or re-sort — the ordering is
   part of the audit record.
4. The answer is a **typed** event carrying `candidateId`, `params`, or (clarify only)
   `text`. Never prose to be re-understood.
5. Load by id, apply, continue from the verdict stage.

Non-negotiables:

- Persisted, not in-memory. Survives restart; `uar_recover` brings the surface back.
- Single-use and expiring. A stale or answered id → `resolution.stale`, never a re-run.
  15-minute TTL, `resolution.expired` on elapse.
- Answering `CandidateChoice` must **not** re-run retrieval. Re-running can produce a
  different set, which would make the operator's choice a lie. Assert this in a test.
- One open resolution per run. Two means you are interrogating — collapse into one
  `ParamForm`.
- Two rounds maximum, then `no_task` with an honest explanation.

## Scoring

The formula and weights are in the spec. Implement them literally. Two things to internalize
rather than transcribe:

- The `blastPenalty` (0.00 read / 0.05 mutating / 0.15 privileged) is deliberate asymmetry.
  Asking about a privileged action costs seconds; guessing one costs an incident.
- `priorAcceptRate` may reorder candidates. It may **never** move a threshold. A system that
  lowers its own bar after a run of successes is how an agent that asked politely for a
  month suddenly restarts the wrong daemon. No code path writes a threshold — I will grep
  for one.

## Fixtures

`docs/ui/60-acceptance-tests.md` Gate 3 has the fixture table. Every row is a test. Note
`reboot` → never `service.restart`; `negativeTriggers` are removed from the candidate set,
not down-weighted. And `restart everything` is `ambiguous` — a fan-out plan is never
implicit.

Determinism test: same prompt, manifest, and thresholds produce an identical candidate set
and order across 100 runs.

## Start here

Before writing code, show me: the capability manifest schema you will resolve against, where
the manifest comes from today, and your candidate `resolver/policy.rs` with every constant
named. If the manifest does not exist yet, say so — inventing one silently is worse than
blocking on it.
