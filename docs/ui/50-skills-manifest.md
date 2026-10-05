# Skills Manifest

> The skill collection at `hybrid-mobile-architecture-src/skills/` — 37 skills. This file
> says which ones are **mandatory** for each phase of this build, and which are context.
>
> UI/UX skills are mandatory, not optional (CLAUDE.md, Development Philosophy). Invoke them
> **before** writing UI code. First-shot quality is the discipline; a review pass afterwards
> recovers a fraction of it.

## Activation mechanics

Skills fire on prompt-word match through the `UserPromptSubmit` hook
(`.claude/hooks/skill-activation.py`), which appends a directive reminder via
`additionalContext`. It is additive and never blocks. Measured activation rises from ~50%
to ~84–100% with the hook in place. A `PostToolUse` hook reminds after a UI file edit to run
`a11y-gate`.

If your harness has no hook mechanism (Codex, Kimi), **name the skills explicitly in the
prompt** — see `prompts/harness-notes.md`. Do not assume they self-activate.

---

## Mandatory per phase

| Phase | Skills to invoke before writing code |
|---|---|
| 0 · Contracts | `a2ui-surface-contract`, `agui-event-contract`, `content-block-ui`, `dependency-pin-discipline` |
| 1 · Catalog | `content-block-ui`, `hybrid-design-tokens`, `a11y-gate`, `tauri-ui-review`, `reference-ui-fidelity` |
| 2 · Stream | `agui-event-contract`, `axum-agent-gateway`, `pem-local-first` |
| 3 · Resolver | `agent-runtime-security`, `domain-glossary-service`, `client-rag`, `persona-scoped-agent` |
| 4 · Surfaces | `reference-ui-fidelity`, `mobile-navigation`, `tauri-custom-titlebar`, `tauri-ui-review`, `a11y-gate`, `hybrid-design-tokens` |
| 5 · A2A | `axum-agent-gateway`, `agent-runtime-security`, `legacy-app-embed` (postMessage/origin patterns) |
| 6 · Review | `orchestrate-prometheus-application`, `realtime-skill-refiner` |
| Every phase | `hybrid-runtime-verification` before claiming anything works |

---

## The collection

### Agentic core — read all four before Phase 0

| Skill | Fires on |
|---|---|
| `a2ui-surface-contract` | A2UI components, action continuation, dynamic forms, generated UI state, approval surfaces, cross-platform rendering parity |
| `agui-event-contract` | Agent event envelopes, SSE resume, run correlation, tool approval state, cancellation, event persistence, unknown-event compatibility |
| `axum-agent-gateway` | Agent HTTP/SSE routes, verified identity, tenant derivation, Cedar authorization, cancellation, idempotency, UAR integration |
| `agent-runtime-security` | MCP tools, approvals, effect classification, policy checks, agent budgets, JWT/JWKS identity, tenant isolation, audit logs |

### UI — mandatory, not optional

| Skill | Fires on |
|---|---|
| `content-block-ui` | Rendering/adding any ContentBlock variant; the 11-variant contract and the exhaustiveness rule |
| `hybrid-design-tokens` | Any colour, spacing, typography, radius, or theme value on any surface |
| `a11y-gate` | Any UI edit; cross-surface WCAG 2.2 AA checklist |
| `tauri-ui-review` | After any React/Tauri surface change; screenshots at 320/768/1024/1440 × both themes |
| `flutter-golden-ui` | After any Flutter widget change; goldens + dart-mcp hot-reload verify |
| `reference-ui-fidelity` | Whenever a spec, prototype, screenshot, or reference app exists — converts it to a screen/state inventory and treats it as a visual oracle |
| `mobile-navigation` | Top-level navigation on any surface; rail-vs-bar is a **width** decision, never a platform check |
| `tauri-custom-titlebar` | Desktop window frame, drag regions, window controls |
| `entity-graph-web-shell` | A React shell whose normalized state is owned by PEM with an Axum boundary owning remote effects |
| `mini-app-module` | Self-contained route modules, feature manifests, capability declarations, lazy loading |
| `legacy-app-embed` | iframe integration, postMessage protocols, origin validation, correlation ids |

### Data, state, retrieval

| Skill | Fires on |
|---|---|
| `pem-local-first` | Any client entity/server state; **TanStack Query / SWR / Apollo cache are prohibited** |
| `client-rag` | Vector search, embeddings, semantic recall; on-device only, 384-dim everywhere |
| `sync-doctrine` | Any sync, replication, realtime, offline, or local-first work; every datum has one lane and one privacy class |
| `peer-profile-sync` | Profile data, PII, agent-learned facts, device pairing; Loro vault, device-to-device only |
| `domain-glossary-service` | Canonical terms, synonyms, entity mappings, policy vocabulary — the resolver's normalize stage |
| `anonymized-replica` | POC datasets, de-identification, production-shape fixtures |
| `client-rag` + `domain-glossary-service` together | The resolver's retrieve stage. Neither alone is enough |

### Runtime, inference, platform

| Skill | Fires on |
|---|---|
| `hybrid-runtime-verification` | Before calling anything working, done, ready, or verified |
| `local-inference-lanes` | Engine selection; per-**device**, never one "mobile" engine |
| `persona-scoped-agent` | Agents constrained by verified persona and project policy |
| `hybrid-mobile-architecture` | The umbrella architecture skill; UAR integration, typed native bridges |
| `dependency-pin-discipline` | Any dependency add, bump, or version literal; `versions.toml` is the only source |
| `launchagent-supervisor` | LaunchAgent/systemd/Scheduled Task authoring, crash loops, watchdogs |
| `tauri-tray-app` | Tray-resident app, health-driven icon, popover, frameless dashboard |
| `deploy-hybrid-agentic-stack` | Full-stack deployment, Compose, Kubernetes, auth |

### Process and packaging

| Skill | Fires on |
|---|---|
| `orchestrate-prometheus-application` | Planning or prompting a multi-phase build; model/harness selection; KBD loops |
| `karpathy-progress-memory` | Task and phase boundaries, verification discoveries, before handoff/commit |
| `realtime-skill-refiner` | A skill produced a wrong answer, fired wrongly, or failed to fire |
| `claude-hooks-reliability` | A hook is not firing, a matcher is too narrow/broad, hook stdout corrupts a decision |
| `connected-skill-packages` | Installing/upgrading/validating a git-based skill marketplace |
| `auto-skill-package-integration` | Auto-detecting new skill packages from a watched remote or directory |
| `build-branded-docusaurus` | A docs portal for the project |
| `a11y-gate` + `tauri-ui-review` + `flutter-golden-ui` | The three visual/accessibility gates. All three are blocking |

---

## Project-local skills

Five of the above ship as project-local templates emitted into `<project>/.claude/skills/`
by `scripts/add-project-skills.sh`: `content-block-ui`, `hybrid-design-tokens`,
`tauri-ui-review`, `flutter-golden-ui`, `a11y-gate`. They travel with the generated code
and activate on prompt-word match. Folder name must equal the frontmatter `name`.

## Priority for a first-shot design

`hybrid-design-tokens` (tokens) → `reference-ui-fidelity` (inventory from the oracle) →
`content-block-ui` (the contract) → composition → `tauri-ui-review` (screenshots) →
`a11y-gate` (the gate). Do not reorder: composing before tokens produces values that then
have to be hunted down, and the hunt is where banned colours survive.
