# Acceptance Tests

> Each gate from `00-BUILD-PLAN.md`, expressed as something that passes or fails. Compiling
> is not evidence (`hybrid-runtime-verification`).

## Gate 0 · Contracts

| Test | Pass condition |
|---|---|
| Schema round-trip | Every file in `reference/` validates against its schema in `schemas/` |
| Mirror determinism | `src/bridge/a2ui/types.ts` regenerates byte-identically from a clean checkout |
| Command parity | `node --test` on the generated command-contract test passes; `shell:default` absent |
| Union frozen | `ContentBlock` has exactly 11 variants; a 12th fails the audit |

## Gate 1 · Catalog

| Test | Pass condition |
|---|---|
| Exhaustiveness | Removing a case from the `ContentBlock` switch is a **compile error**, not a runtime fallthrough |
| Registry dispatch | No `if`/`switch` on `artifactType` outside `catalog.ts` (grep gate in CI) |
| Unknown component | An unregistered id renders `UnknownSurface` with inspectable props; run state is neither failed nor complete |
| Malformed props | Each component rejects a malformed fixture, renders `MalformedSurface`, emits `surface.malformed`, does not throw |
| Six states | Every component renders pending-approval, denied, cancelled, failed, resumed, completed |
| Screenshots | 320 / 768 / 1024 / 1440 × light + dark, per component |
| Contrast | Every text/background pair ≥ 4.5:1 (body) or 3:1 (≥24 px). **Audit dark surfaces explicitly** |
| Token purity | No hex literal and no raw px in component source; only token references |

## Gate 2 · Stream

| Test | Pass condition |
|---|---|
| Reconnect | Disconnect mid-stream, reconnect: projection identical, no duplicate blocks |
| Duplicates | Replaying the same 200 events produces an identical projection |
| Out of order | Shuffled delivery within the buffer window produces an identical projection |
| Gap timeout | A permanent gap triggers a replay request, not an out-of-order apply |
| Restart | Kill and relaunch: `uar_recover` restores the projection and any open resolution surface |
| Slow consumer | A 5 s stalled reader drops no events |
| Unknown event | Preserved and displayed; not counted as success |
| Unknown minor version | Accepted, unknown fields preserved. Unknown **major** rejected |
| No prose completion | A run with text saying "done" but no `run.completed` stays running |

## Gate 3 · Resolver

Fixture table — each row is a test. Thresholds: `TAU_FLOOR` 0.20, `TAU_LOW` 0.45,
`TAU_HIGH` 0.72, `DELTA` 0.15.

| Prompt | Expected verdict | Expected surface |
|---|---|---|
| `restart surreal-memory` | `resolved` | `FixPlan` / direct plan + confirm gate |
| `bounce the memory one` | `resolved` via glossary + entity resolver | plan |
| `restart it` (no prior context) | `underspecified` | `ParamForm` with a service picker |
| `reindex` | `ambiguous` (`skills.reindex` vs `client-rag` reindex) | `CandidateChoice`, 2 candidates, evidence shown |
| `fix the tray` | `low_confidence` | `ClarifyPrompt` + nearest 3, unranked |
| `what is a LaunchAgent` | `no_task` | prose only, no capability asserted |
| `reboot` | `no_task` or `low_confidence` — **never** `service.restart` | negative trigger holds |
| `disable sovereign-sync` | `resolved`, `effectClass: privileged` | plan + approval gate, gate not skipped |
| `restart the thing on 23001` | `resolved` via port entity resolution | plan |
| `restart everything` | `ambiguous` | `CandidateChoice`; a fan-out plan is never implicit |

| Behavioural test | Pass condition |
|---|---|
| Determinism | Same prompt + manifest + thresholds → identical candidate set and order, 100 runs |
| Restart mid-resolution | Pending resolution survives; the surface returns; the answer still applies |
| Stale answer | Answering an answered or expired `resolutionId` → `resolution.stale`, no execution |
| No re-retrieval | Answering `CandidateChoice` does not re-run retrieval (assert the candidate set is unchanged) |
| Two rounds max | A third clarification becomes `no_task` with an explanation |
| Concurrent duplicates | The same prompt twice concurrently produces one plan and one effect |
| Injection | An instruction embedded in a service name, description, or log line is treated as data |
| Persona scoping | A capability outside the persona's set is excluded at retrieval and visible in `ResolutionTrace` |
| Threshold immutability | No code path writes a threshold; feedback cannot move one |

## Gate 4 · Surfaces

| Test | Pass condition |
|---|---|
| Push, not overlay | Above 1100 px the slideout pushes; the board reflows 3→2 queue and 4→3 columns; no horizontal scroll |
| Overlay crossover | Below 1100 px it overlays with a scrim |
| Width preference | 400–620 persists across restart |
| Cross-surface effect | Approving in the thread changes the board's health count and queue |
| Ask about this | Opens the assistant seeded with the service; closes the drawer |
| No layout jump | Streaming text does not shift subsequent blocks (measure `getBoundingClientRect`) |
| Scroll discipline | Auto-pin releases on manual scroll up; no `scrollIntoView` anywhere |
| Oracle parity | Screen and state inventory from Concept B fully covered |

## Gate 5 · A2A

| Test | Pass condition |
|---|---|
| Forged credential | Rejected |
| Expired credential | Rejected |
| Wrong audience | Rejected |
| Revoked key | Rejected |
| Cross-tenant | Denied |
| Caller-selected tenant/role/provider | Ignored; derived from the verified session |
| Idempotent retry | One effect |
| Peer cancel | Propagates to UAR |
| Peer disconnect | Task cancelled, not left `working` |
| Privileged default | Not exposed to a peer without an explicit pre-pairing grant |
| Agent Card | Regenerates identically from the manifest; JWS verifies over the JCS-canonicalized card |
| Receipt provenance | `caller: a2a` plus peer node id present |
| Output flood | Bounded by the output limit |
| Approval race | Two clients deciding one `approvalId` → one decision |

## Gate 6 · Review

| Test | Pass condition |
|---|---|
| Every harness | `codex`, `claude`, `local`, `none` each produce a legal assignment |
| Collision | harness `codex` + critic `openai/gpt-5-codex` → substitution + trace + visible warning |
| No alternative | Degrades to same-provider, records it in the verdict, does not skip |
| Unhealthy provider | Never selected for any role |
| Distinctness | Judge provider ≠ critic provider ≠ producer provider |
| Blocked approval | Approve control `aria-disabled` with the critic's reason, still in the DOM |

## CI gates

Run on every commit: `bash scripts/audit.sh doc-consistency` (version drift),
`node scripts/check-skill-contracts.mjs`, `node scripts/check-runtime-security.mjs`,
`node scripts/check-prometheus-boundary.mjs`, the schema round-trip, the contrast audit,
and the grep gate for `artifactType` dispatch outside the registry.

Add one more, learned the hard way: **the contrast audit must scan value maps in `.ts`/`.js`
data files, not only style attributes.** A banned colour survived two eliminations by living
in a shared level-colour map rather than in any component's markup.
