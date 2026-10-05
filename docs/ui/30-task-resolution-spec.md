# Task Resolution Specification

> How a sentence becomes exactly one governed task — and what happens when it doesn't.
> Backing skills: `agent-runtime-security`, `domain-glossary-service`, `client-rag`,
> `agui-event-contract`, `persona-scoped-agent`.

The resolver's job is to turn a prompt into a **`RunPlan`**: one capability id, bound
parameters, an effect class, and a gate. It does not execute. It does not route providers.
It does not decide policy. It produces a plan that UAR then governs exactly as it governs
a plan that came from a button.

## Why this exists

"The chat can do anything the UI can" is only true if both enumerate the same registry.
The board renders a button per capability; the thread must reach those same capabilities
from natural language. The hard part is not the happy path — it is the two thirds of real
prompts that are ambiguous, underspecified, or simply not a task at all. A resolver that
guesses on those is worse than one that asks, because a wrong `service.disable` is not
recoverable by an apology.

---

## The capability manifest

The resolver's entire search space. Generated from the same source that generates the
board's buttons and the A2A Agent Card — never a second list.

```json
{
  "id": "service.restart",
  "summary": "Restart a supervised service and wait for readiness",
  "effectClass": "mutating",
  "gate": "confirm",
  "callers": ["ui", "chat", "mobile"],
  "params": {
    "type": "object",
    "required": ["serviceId"],
    "properties": {
      "serviceId": { "type": "string", "resolver": "service" },
      "waitForReady": { "type": "boolean", "default": true }
    }
  },
  "triggers": ["restart", "bounce", "kick", "reload the service", "bring it back"],
  "negativeTriggers": ["restart my mac", "reboot"],
  "examples": [
    { "prompt": "bounce surreal-memory", "params": { "serviceId": "ai.prometheus.surreal-memory-native" } }
  ],
  "blastRadius": "single-unit"
}
```

`negativeTriggers` matter more than they look. "reboot" resolving to `service.restart` is
the kind of near-miss that erodes trust faster than a missed hit.

`params[].resolver` names an **entity resolver** — a typed lookup that turns "surreal
memory", "the memory one", or "23001" into a service id. Entity resolution is a separate
pass with its own confidence, and it is the most common source of the `underspecified`
verdict.

---

## Pipeline

Five stages, all in Rust, all deterministic given the same manifest and thresholds.

### 1 · Normalize

- Lowercase, strip punctuation that is not part of an identifier, keep port numbers and
  dotted ids intact.
- Expand synonyms through the versioned domain glossary
  (`domain-glossary-service`). The glossary is versioned because a synonym change silently
  changes every future resolution; record which glossary version resolved a prompt.
- Extract candidate entity mentions and their spans. Keep the spans — the disambiguation
  UI needs to show what matched where.

### 2 · Retrieve

Hybrid search over the manifest, per `client-rag`. Entirely on-device.

- **Lexical:** BM25 over `id`, `summary`, `triggers`, and `examples[].prompt`.
- **Semantic:** 384-dim embeddings (one dimension everywhere, so vectors survive
  cross-device sync) over `summary` + `triggers` + `examples`.
- Fuse with reciprocal rank fusion. Take the top 8 as the candidate set.
- Any candidate matching a `negativeTrigger` is removed, not down-weighted.

### 3 · Score

```
confidence(c) =
    0.45 * fusedRetrievalScore(c)          // normalized 0..1
  + 0.25 * paramCompleteness(c)            // bound required params / required params
  + 0.20 * priorAcceptRate(c)              // smoothed, from resolution feedback
  + 0.10 * personaAffinity(c)              // 1.0 if in the verified persona's set, else 0
  - blastPenalty(c)                        // 0.00 read · 0.05 mutating · 0.15 privileged
```

`margin = confidence(top1) - confidence(top2)`.

Every weight and threshold is a named constant in `resolver/policy.rs`. A model never
chooses one. They are versioned and logged with each resolution, because a threshold change
invalidates prior feedback data.

The `blastPenalty` is deliberate asymmetry: the cost of asking about a privileged action
is a few seconds; the cost of guessing one is an incident. Bias toward asking as the effect
class rises.

### 4 · Verdict

Evaluated in order. First match wins.

| Verdict | Condition | Emits | Terminal? |
|---|---|---|---|
| `no_task` | top1 < `TAU_FLOOR` (0.20) | plain `text` answer, no capability asserted | yes |
| `low_confidence` | top1 < `TAU_LOW` (0.45) | `ClarifyPrompt` + nearest 3, unranked | no |
| `ambiguous` | top1 ≥ `TAU_LOW` and `margin` < `DELTA` (0.15) | `CandidateChoice` over candidates within `DELTA` | no |
| `underspecified` | top1 ≥ `TAU_HIGH` (0.72), required params unbound | `ParamForm` for the unbound params only | no |
| `resolved` | top1 ≥ `TAU_HIGH`, `margin` ≥ `DELTA`, params bound | `RunPlan` | yes |

Between `TAU_LOW` and `TAU_HIGH` with a healthy margin, the verdict is `ambiguous` with a
single candidate — which renders as a confirm-this-one card, not a silent run. A lone
mid-confidence candidate is still a guess; make the operator nod.

`resolved` with `effectClass: privileged` still requires the approval gate. Resolution
confidence and execution authority are different questions, and a confident resolver does
not earn a skipped gate.

### 5 · Plan

```json
{
  "planId": "pl-8823",
  "capability": "service.restart",
  "params": { "serviceId": "ai.prometheus.surreal-memory-native", "waitForReady": true },
  "effectClass": "mutating",
  "gate": "confirm",
  "confidence": 0.88,
  "resolvedVia": ["glossary@4", "bm25", "vector", "entity:service"],
  "idempotencyKey": "…"
}
```

The plan goes to the gateway. The gateway authorizes it. UAR runs it. The resolver is done.

---

## Disambiguation as a task state

This is the part that is usually built wrong — as a chat message the model writes and then
re-parses. It is a **state machine**, and it maps 1:1 onto the A2A task lifecycle.

```
submitted ──► working ──┬──► completed
                        ├──► input-required ──► working ──► …
                        ├──► canceled
                        └──► failed
```

On a non-terminal verdict:

1. The resolver persists a `PendingResolution`: `resolutionId`, prompt, glossary version,
   candidate set with scores, threshold set, verdict, and `createdAt`.
2. It emits `resolution.required` carrying the component id and props. The run's A2A task
   state becomes `input-required`. **The run is not failed and not complete.**
3. The client renders the component. The client does not re-score, re-rank, or re-order —
   it renders the order it was given, because the ordering is part of the audit record.
4. The operator answers. The client emits `resolution.answered` with the `resolutionId` and
   a **typed** answer: `candidateId`, `params`, or `text`. Never prose that has to be
   re-understood, except in the `ClarifyPrompt` free-text case, which re-enters the
   pipeline at stage 1 as a new prompt correlated to the same run.
5. The resolver loads the pending resolution by id, applies the answer, and continues from
   stage 4. A resolved answer skips straight to `plan`.

### Rules that make it survive contact with reality

- **Persisted, not in-memory.** A `PendingResolution` outlives an app restart, a stream
  reconnect, and a window close. `uar_recover` restores it and the surface comes back.
- **Single-use and expiring.** An answer for an already-answered or expired `resolutionId`
  is rejected with `resolution.stale`, never re-run. Default TTL 15 minutes; expiry emits
  `resolution.expired` and the surface goes to a spent state with the reason.
- **The answer is not a new prompt** (except free-text clarify). Answering a
  `CandidateChoice` must not re-run retrieval — the candidate set is already fixed, and
  re-running it can produce a different set, which would make the operator's choice a lie.
- **Max one open resolution per run.** A second one means the resolver is interrogating;
  collapse the questions into one `ParamForm` instead.
- **At most two rounds.** If a run needs a third clarification, the verdict becomes
  `no_task` with an honest explanation. Three questions is a design failure, not a
  conversation.
- **Cancellation is always available.** Cancelling a resolution cancels the run and says
  so. It does not leave an orphan pending row.

---

## Feedback, carefully

Each answered resolution writes a record: prompt hash, glossary version, threshold
version, candidate set, chosen candidate, rejected candidates, verdict, latency.

That record feeds `priorAcceptRate` — smoothed, capped in its contribution by its 0.20
weight, and **never** allowed to move a threshold. Learning may reorder candidates; it may
not make the resolver bolder. A system that lowers its own bar after a few successes is how
an agent that asked politely for a month suddenly restarts the wrong daemon.

Records are scoped to the verified persona and tenant. A prompt hash is stored, not the
prompt, unless the operator opts into keeping the text for their own review.

## Failure modes to test explicitly

| Failure | Expected behaviour |
|---|---|
| Two capabilities with near-identical summaries | `ambiguous`, both shown with distinguishing evidence |
| A prompt naming a service that does not exist | `underspecified`, `ParamForm` with a resolver-backed picker, not a free text box |
| A prompt that is a question, not a task | `no_task`, answered as prose, no capability asserted |
| An injected instruction inside a service name or log line | Tool names and descriptions are untrusted data; never treated as an instruction |
| Manifest changed between resolution and answer | Answer rejected as stale; re-resolve from the new manifest |
| Same prompt sent twice concurrently | One plan, one effect, by idempotency key |
| A capability the persona may not call | Excluded at retrieval, not at execution, and the exclusion is visible in `ResolutionTrace` |
