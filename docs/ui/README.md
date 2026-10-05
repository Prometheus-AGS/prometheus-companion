> Current source status (2026-10-05): retained authored UI specifications,
> schemas, examples and prompts. They are not an implemented application or
> release acceptance evidence. Current Tauri window remains the starter UI;
> the service-health tray hosts the operational node separately. See
> [current scope](../../README.md) and [maintenance](../maintenance.md).
> Historical CI/unit-first instructions below do not override canonical
> implementation-first, local-only, final-integration policy.

# Companion UI Package

> **Unzip into `docs/ui/` of `prometheus-companion`.**
> Authoring context: TJ-ARCH-MOB-001 · `hybrid-mobile-architecture` skill pack ·
> Companion Concept B (Operations Board) as the visual acceptance oracle.

This package is the buildable specification for the Companion's **agentic UI layer**: the
A2UI component catalog, the React 19 surfaces that render it, the AG-UI agent that drives
it, the A2A boundary that exposes it, and the prompt-to-task resolver — including the
disambiguation surface that appears when a prompt does not resolve cleanly to one task.

It contains no application code. It contains the specs, schemas, and harness prompts a
coding agent needs to write that code correctly on the first pass.

## Read in this order

| # | File | What it settles |
|---|---|---|
| 1 | `00-BUILD-PLAN.md` | Six phases, their acceptance gates, and what each phase may not touch |
| 2 | `10-a2ui-component-spec.md` | The 12-component client catalog and the frozen-union rule (**ADR-UI-1**) |
| 3 | `20-agui-agent-spec.md` | AG-UI envelope, run lifecycle, gateway sequence, A2A Agent Card |
| 4 | `30-task-resolution-spec.md` | Prompt → capability resolution, confidence maths, the five verdicts, disambiguation |
| 5 | `11-react-surfaces-spec.md` | The React surfaces: assistant slideout, settings, board integration |
| 6 | `40-adversarial-review-config.md` | Producer / critic / judge resolution under a harness |
| 7 | `50-skills-manifest.md` | Which skills from the pack are mandatory for each phase |
| 8 | `60-acceptance-tests.md` | The test matrix that closes each gate |

`prompts/` holds the harness-facing prompts. `schemas/` holds the JSON Schemas that CI
validates against. `reference/` holds worked examples.

## The three invariants

Everything in this package follows from these. If a decision in the code contradicts one,
the code is wrong.

1. **The client owns the catalog.** A2UI is a projection format. The agent names a
   registered component and supplies validated props; it cannot ship markup, code, or a
   new component. An unknown component id renders as an inspectable artifact, never as a
   guess and never as success.
2. **The resolver plans; UAR executes.** Prompt resolution produces a `RunPlan`. It never
   invokes anything. Every effect passes the same governance whether the caller is the
   board, the thread, an A2A peer, or a paired phone — one registry, four callers.
3. **Ambiguity is a task state, not a chat message.** A prompt that does not resolve puts
   the run into `input-required` with a persisted `resolutionId`. The answer arrives as a
   typed event carrying a candidate id — never as prose to be re-parsed.

## Versions

Mirrored from `hybrid-mobile-architecture-src/versions.toml`, which is the only source.
Do not edit these here — bump `versions.toml`, then re-run `bash scripts/audit.sh
doc-consistency`.

| Component | Pin |
|---|---|
| rust | 1.97.1 |
| node | 26.5.0 |
| typescript | 7.0.2 |
| react | 19.2.7 |
| zustand | 5.0.14 |
| vite | 8.1.5 |
| tauri_cli | 2.11.4 |
| pnpm | 11.15.0 |
| surrealdb | 3.2.1 |
| sqlite_vec | 0.1 |
| embedding_dim | 384 |

Pins are exact, never floors. The rationale comment next to a pin is the real artifact —
see `dependency-pin-discipline`.

## What is already decided and must not be re-litigated

- The `ContentBlock` union is **frozen at 11 variants**. Catalog components ride inside
  `artifact`; see ADR-UI-1 in `10-a2ui-component-spec.md`.
- The Tauri command surface is **exactly three commands** — `uar_run`, `uar_cancel`,
  `uar_recover` — generated from `assets/templates/command-contract/commands.json`.
  Adding a fourth means editing that manifest and regenerating, never hand-writing.
- `shell:default` is forbidden in Tauri permissions. The generator asserts it.
- Confidence thresholds are constants in one policy file. A model never chooses a
  threshold, and a threshold is never lowered to make a run succeed.
