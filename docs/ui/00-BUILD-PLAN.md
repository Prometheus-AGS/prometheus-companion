# Build Plan

> Six phases. Each has a gate. A phase is not done because it compiles — `working` is a
> runtime claim (`hybrid-runtime-verification`). Do not start phase N+1 with phase N's
> gate open.

## Phase 0 · Contracts before code

Produce the types, produce nothing that renders.

**Do**
- Define the A2UI envelope and event union in Rust (`gen_ui_core/src/protocol/a2ui.rs`),
  matching `schemas/agui-event.schema.json` exactly.
- Define the capability manifest type, matching `schemas/capability-manifest.schema.json`.
- Define the catalog component registry type and the 12 component prop types, matching
  `schemas/a2ui-catalog.schema.json`.
- Regenerate the TypeScript mirror (`src/bridge/a2ui/types.ts`) from the Rust source.

**Do not**
- Do not add a `ContentBlock` variant. See ADR-UI-1.
- Do not hand-edit a generated mirror.

**Gate 0** — `cargo test -p gen_ui_core` green; the TS mirror regenerates byte-identically
from a clean checkout; every schema in `schemas/` validates its example in `reference/`;
`node --test` on the command-contract parity test passes.

## Phase 1 · The catalog, rendered

Twelve components, each with a story of its own before any agent drives them.

**Do**
- One React component per catalog entry in `@prometheus-ags/gen-ui-react`, exhaustive
  dispatch with a `never` guard in `default`.
- A catalog registry module that maps component id → component + prop validator. Renderer
  resolves through the registry only; no `if (id === …)` chains anywhere.
- The unknown-component path: render `UnknownSurface` with the raw payload inspectable,
  and mark the run neither failed nor complete.

**Do not**
- Do not fetch, subscribe, or `invoke()` from a catalog component. They are pure props-in.
- Do not style with ad-hoc values — every value comes from the token set
  (`hybrid-design-tokens`).

**Gate 1** — every component has a screenshot at 320/768/1024/1440 in both themes
(`tauri-ui-review`); the a11y checklist passes (`a11y-gate`); prop-validation rejection is
covered by a test per component; `UnknownSurface` renders for an unregistered id.

## Phase 2 · The AG-UI stream

**Do**
- SSE client with resume-from-last-committed-event-id, idempotent application, sequence
  enforcement, and out-of-order buffering.
- Zustand store holding the run projection. Components read through a `useRun` hook and
  never touch the transport.
- Explicit UI states for approval, denial, cancellation, timeout, and failure.

**Do not**
- Do not infer completion from prose. Only `run.completed` completes a run.
- Do not treat an unknown event type as success.

**Gate 2** — disconnect/reconnect mid-stream resumes without duplicate blocks; duplicate
and out-of-order deliveries produce an identical projection; a restart recovers the
projection through `uar_recover`; slow-consumer backpressure does not drop events.

## Phase 3 · The resolver

**Do**
- Implement the pipeline in `30-task-resolution-spec.md` in Rust: normalize → retrieve →
  score → verdict → plan.
- Emit `resolution.required` for the ambiguous, underspecified, and low-confidence
  verdicts. Persist the pending resolution keyed by `resolutionId`.
- Wire the three disambiguation components to answer it.

**Do not**
- Do not resolve in the UI, in TypeScript, or in a prompt. The resolver is Rust and is
  deterministic given the same manifest and thresholds.
- Do not execute anything. The resolver's output is a `RunPlan`.

**Gate 3** — the fixture set in `60-acceptance-tests.md` produces the expected verdict for
every row, including the deliberate near-ties; a resolution survives an app restart and
resumes from the persisted `resolutionId`; answering with a stale `resolutionId` is
rejected, not silently re-run.

## Phase 4 · Surfaces

**Do**
- The assistant slideout and the settings surfaces per `11-react-surfaces-spec.md`.
- Cross-links: a surface row opens the corresponding operator view; the model chip opens
  the model settings section.

**Do not**
- Do not add a second copy of state that the board already owns. The thread projects; it
  does not cache a parallel service list.

**Gate 4** — the slideout pushes rather than overlays and the board reflows without
horizontal scroll at every breakpoint; approving a plan in the thread changes the board's
own health count; the reference oracle comparison passes (`reference-ui-fidelity`).

## Phase 5 · A2A boundary

**Do**
- Publish the Agent Card from the capability manifest — generated, never hand-written.
- Gateway, not endpoint: verified session, tenant derivation, policy check, bounded
  budgets, idempotent submit, redacted persisted outcome.

**Do not**
- Do not accept a caller-selected tenant, role, provider route, or raw tool permission.
- Do not expose a privileged-class capability to a peer by default.

**Gate 5** — the forged / expired / wrong-audience credential tests fail closed;
cross-tenant access is denied; a peer's cancel propagates to UAR; the Agent Card
regenerates identically from the manifest.

## Phase 6 · Review policy

**Do**
- Implement the producer/critic/judge resolver in `40-adversarial-review-config.md`,
  including the harness exclusion set and the substitution trace.

**Gate 6** — every harness value produces a legal assignment; a deliberate collision
produces a substitution with a recorded trace; the no-alternative case degrades and says
so rather than skipping the pass.

## Order of operations for the agent

1. Read `AGENT_BASE_RULES.md` in the target project. It wins over anything here.
2. Read the phase you are on and the gate that closes it. State the success criteria
   before writing code (Rule 4).
3. Invoke the skills listed for that phase in `50-skills-manifest.md` **before** writing
   UI code, not after.
4. Make the smallest change that closes the gate. No speculative abstraction (Rule 2).
5. Close the gate with evidence — a screenshot, a passing test, a device run. Not a claim.
