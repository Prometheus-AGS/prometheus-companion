<!-- GENERATED — DO NOT EDIT — source: templates/rules/agents.md.tmpl
     Regenerate with: bash scripts/gen-rule-files.sh
     Verify with:     bash scripts/audit-generated.sh -->

# AGENTS.md — Prometheus Companion

> **Read this first.** Every AI agent that opens this repository
> (Claude Code, OpenCode, Codex, Kimi, MiniMax, Cursor, Aider,
> Cline, Roo, Windsurf, Zed, Gemini CLI) must read and follow
> this file. The companion **`CLAUDE.md`** inherits every rule
> here and adds Claude-Code-specific behavior.
>
> **Tiered on purpose.** Instruction-following degrades as rule
> count rises. The first sections are the ones to keep resident;
> the appendices are load-on-demand.

---

## §0 · The Constitution (inviolable; survives compaction)

These rules govern every turn. If context is compacted, **this is
the section to re-read first**.

- **§0.1 — Think before coding.** State assumptions. Surface
  tradeoffs before implementing. If uncertain, if interpretations
  differ, or if a simpler approach exists — say so and, when it
  blocks correctness, stop and ask.
- **§0.2 — Observed Problems Only.** Write code only for an
  *observed* problem. A problem is observed iff it comes from:
  (1) an operator report this session, (2) an error/log/stack
  trace visible this session, (3) a failing test this session,
  or (4) an explicit requirement. **Defensive code** — validation,
  guards, error handling, fallbacks, retries, timeouts — requires
  a named failure scenario from an observed problem. No scenario,
  no code. The ask-valve: an unobserved concern gets one sentence
  and a question, never speculative code. Silence means no.
- **§0.3 — Security traces to a real boundary.** Hardening at an
  *actual* trust boundary in the code — untrusted input,
  authn/authz, secrets, tenant isolation, prompt-injection surface,
  tool-execution boundary — is a standing requirement, not
  speculation. It must trace to a boundary present in the code
  and be named in the completion summary. Never log secrets,
  tokens, keys, or sensitive user data.
- **§0.4 — Simplicity and surgical scope.** Minimum code that
  solves the problem. Touch only what is necessary. Do not
  refactor, reformat, or "improve" adjacent working code — treat
  its current state as intentional. Match existing conventions.
- **§0.5 — Truth over fluency.** Never prefer a confident answer
  to a correct one. State uncertainty plainly. Do not invent
  APIs, files, packages, commands, or behavior. If unknown, say so.
- **§0.6 — Verified vs. self-reported.** Report what was
  *actually run* and at which tier. An unverified claim reported
  as verified is worse than no test.
- **§0.7 — Preserve intent; preserve behavior.** Do not silently
  expand or reduce scope. Do not break existing behavior unless
  the task requires it; when you do, identify current vs.
  desired behavior, update tests/docs, and call out the breaking
  change.
- **§0.8 — Architecture before code.** Before implementing,
  identify affected subsystems, data flow, interface contracts,
  persistence/UI/security/runtime impact, and the testing
  strategy. Do not start coding until the architecture is
  understood.

**Source:** `prometheus-skill-pack/AGENT_BASE_RULES.md §A`.

---

## §1 · Local-only validation (MANDATORY)

All builds, tests, linting, formatting, type checks, documentation
checks, API contract checks, doctor runs, health checks, diagnosis,
and release certification **must run on the local development
machine.**

- **Never** use GitHub Actions or any hosted CI/CD runner for
  testing or validation.
- Do not start, rerun, watch, poll, debug, or cite a GitHub
  Actions test workflow.
- Push only after the applicable local gates pass, and record
  the exact local commands and results.
- GitHub may be used for source hosting, review, and an
  explicitly authorized deployment such as GitHub Pages.
- If a legacy test workflow starts automatically, cancel it when
  authorized and continue locally. Its result is not validation
  evidence.

**Source:** `prometheus-skill-pack/AGENTS.md` (Local-Only
Validation MANDATORY).

**Enforcement:** every `pnpm audit:*` script runs locally; the
local pre-push hook refuses to push if any audit fails.

---

## §2 · Progress signaling (MANDATORY)

Every agent must emit a progress signal at the start and
completion of each phase and each task. These signals make long
conversations scannable and let any participant orient
immediately.

```
Starting phase 2 out of 6:  SP-014 fallback SubagentStop matcher verification
Starting task 1 out of 4:   Locate all fallback hook scripts
Completed task 1 out of 4:  Locate all fallback hook scripts
Completed phase 2 out of 6: SP-014 fallback SubagentStop matcher verification
```

**Rules:**

- Signal **before** any work begins, not mid-task.
- Signal **immediately** after completion, before starting the
  next item.
- Read `progress.json` or the active plan to get accurate totals.
  Never guess.
- Use the canonical name from `plan.md` or `progress.json`.
- Signals are plain text in the response — no tool call required.

**Source:** `prometheus-skill-pack/AGENTS.md` (Progress Signaling
MANDATORY).

---

## §3 · File naming — kebab-case (this project's rule)

**Every TypeScript file and folder under `src/` is kebab-case.**
React component *symbols* remain PascalCase (`Titlebar`,
`ServiceCard`, `MacOSTrafficLights`), but the *files* that
contain them are `titlebar.tsx`, `service-card.tsx`,
`macos-traffic-lights.tsx`. Folder names are also kebab-case.

**Allowed exceptions** (framework-mandated, do not change):

- `__root.tsx` and `__root.ts` (TanStack Router convention)
- `index.ts` and `index.tsx` (platform convention)

**Enforcement:** `scripts/audit-naming.sh` (fails on PascalCase
or camelCase file names under `src/`). Run via `pnpm audit:naming`.

**Rationale:** the PMP codebase uses kebab-case for tooling paths
and slash-separated module names; the Companion follows. Pinned
to this project; the prometheus-skill-pack plugin convention may
differ.

---

## §4 · Layer rule — CLEAN architecture

All TypeScript source dependencies point **inward**. Domain has
no imports from any other layer. Presentation imports only from
Application and Domain. Infrastructure implements the ports
declared by Application.

```
┌──────────────────────────────────────────────────────────────┐
│  Presentation (P)                                            │
│  React components, routes, Zustand UI stores                 │
│  imports from: Application (hooks), Domain (types)           │
├──────────────────────────────────────────────────────────────┤
│  Application (A)                                             │
│  Use cases, application hooks, transient stores, mappers     │
│  imports from: Domain (entities, errors, ports)              │
├──────────────────────────────────────────────────────────────┤
│  Infrastructure (I)                                          │
│  Tauri commands, PEM transports, PGlite, Stronghold,         │
│  Iroh, env readers, logger                                   │
│  imports from: Domain (ports)                                │
├──────────────────────────────────────────────────────────────┤
│  Domain (D)                                                  │
│  Entity types, value objects, domain errors, port interfaces │
│  imports from: NOTHING (no React, no Tauri, no PEM)          │
└──────────────────────────────────────────────────────────────┘
```

**Forbidden imports:**

| From layer | Cannot import |
|---|---|
| Domain | anything except other Domain |
| Application | Infrastructure, Presentation |
| Infrastructure | Presentation |
| Presentation | Infrastructure (direct) — go through Application |

**Other presentation rules:**

- No `invoke(` or `listen(` in `src/presentation/components/**`
  (use the application hook instead)
- No `useEffect` data fetching (use PEM hooks)
- No `@tanstack/react-query` anywhere
- No `@radix-ui/*` imports (use `@base-ui-components/react` via
  shadcn)

**Enforcement:** `scripts/audit-layer.sh` (fails on cross-layer
imports). Run via `pnpm audit:layer`.

**Source:** `clean-architecture/SKILL.md` (PMP) +
`entity-graph-web-shell/SKILL.md` (HMA) +
`pem-local-first/SKILL.md` (HMA).

---

## §5 · Tauri surface gating

Every `@tauri-apps/*` import is wrapped in `isTauri()` (or in a
**synchronous-gated module body** that is never imported on the
web). Web builds must not crash with "no `__TAURI_INTERNALS__`
bridge."

**Pattern:**

```ts
import { isTauri } from '@tauri-apps/api/core'
const appWindow = isTauri() ? getCurrentWindow() : null
// appWindow is nullable; UI handles the null case
```

**Source:** `tauri-custom-titlebar/SKILL.md` (HMA).

**Enforcement:** `scripts/audit-layer.sh` flags `invoke(` and
`listen(` in `src/presentation/components/**`.

---

## §6 · Custom title bar

`tauri.conf.json` settings for the main window:

```jsonc
{
  "decorations": false,
  "titleBarStyle": "Overlay",
  "hiddenTitle": true
}
```

**Platform-correct window controls:**

- **macOS** — close / minimize / maximize, **left side**,
  in that order
- **Windows / Linux** — minimize / maximize / close,
  **right side**, in that order

**Drag pattern** (use **both**, not just one):

```tsx
<header
  data-tauri-drag-region
  onMouseDown={(e) => { if (e.button === 0) appWindow?.startDragging() }}
>
```

**Accessibility** (required, since we removed the OS chrome):

- `aria-label` on every control
- Real `<button>` elements
- `select-none` on the bar
- Visible hover / focus / active states
- Contrast ≥ AA

**Source:** `tauri-custom-titlebar/SKILL.md` (HMA) +
`a11y-gate/SKILL.md` (HMA).

**Enforcement:** `scripts/audit-a11y.sh` flags icon-only
buttons without `aria-label`.

---

## §7 · Mobile navigation

**Top-level destinations go in a bottom bar. On every platform.
No platform check.** Switch to a **rail by window width** (≥
600px), never by OS.

| Width | Chrome |
|---|---|
| < 600px | bottom bar |
| ≥ 600px | rail (left side) |

**One list, never two.** Labels, icons, and paths are declared
once; the chrome is chosen by `useBreakpoint`. The selected
index is **derived from the router**, never stored.

**Forbidden patterns:**

- `Platform.isIOS` / `Platform.isAndroid` checks in nav code
- `navigator.userAgent` for nav
- A top tab bar for app-level destinations
- Nav state mirrored into a store/provider
- Two destination lists

**Source:** `mobile-navigation/SKILL.md` (HMA).

---

## §8 · Component library — shadcn/ui with Base UI

shadcn/ui now uses **Base UI** primitives
(`@base-ui-components/react`) by default, not Radix. The
`components.json` key `base: "base-ui"` is the source of truth.

**Initialize:**

```bash
npx shadcn@latest init --base-ui --yes
```

**API differences** (Radix → Base UI):

| Radix | Base UI |
|---|---|
| `asChild` | `render={<button>…</button>}` |
| `Overlay` | `Backdrop` |
| `forceMount` | `keepMounted` (different timing) |
| `onOpenChange(open)` | `onOpenChange(open, eventDetails)` |

**Coexistence:** Base UI and Radix may temporarily coexist in
the same project. Migrate **leaf first** (button, label, field
before the components that import them), one commit per
component. Strangler-fig pattern.

**Source:** `shadcn-deck.com/blog/radix-vs-base-ui`,
`ui.beste.co/blog/base-ui-the-new-default`.

**Enforcement:** `scripts/audit-layer.sh` fails on
`@radix-ui/*` imports.

---

## §9 · Entity management — PEM 3.x only

PEM 3.x (`@prometheus-ags/prometheus-entity-management`) is the
client-side data layer. **No TanStack Query.** Components
consume entity data through application hooks, which in turn use
PEM.

**Layering reminder (from §4):**

```
Component (presentation)
  → Hook (application)
    → PEM hooks + Zustand stores (application)
      → Transport (infrastructure)
        → Tauri command (infrastructure) or local store
```

**Optimistic mutations** are enabled by default. **One queue,
not two** — never maintain a separate replay queue in JS; the
PEM `_operation_queue` is the truth.

**Source:** `pem-local-first/SKILL.md` (HMA) +
`entity-graph-web-shell/SKILL.md` (HMA) +
`prometheus-entity-skills/SKILLS.md` (PMP).

**Enforcement:** `scripts/audit-layer.sh` fails on
`@tanstack/react-query` imports.

---

## §10 · Sync doctrine

**Three lanes, one privacy class per entity:**

| Data shape | Lane | Merge |
|---|---|---|
| Shared app data (entities, settings rows, lookups) | **Relational** | Server-authoritative LWW + upload queue |
| User-owned collaborative (profile vault, scratchpads) | **CRDT (Loro)** | CRDT merge |
| Append-only history (runs, telemetry, sessions) | **Event log** | none |

**Privacy classes:** `public` | `trusted` (both server-synced,
tenant-bound) | `local` (never server-synced, fail closed at the
enqueue boundary). Unknown class = `local`. Secrets are always
`local`.

**The seven invariants:**

1. Server is authority for shared relational data.
2. Client-generated UUID PKs; every write idempotent / replayable.
3. Additive-only local migrations (`deprecated_at`, never
   destructive).
4. Secrets and `local`-class data never enqueue for server sync.
5. Boot order: migrations → seed/lookup → pre-onboarding load →
   [onboarding] → post-onboarding load → sync attach.
6. One abstraction, many transports: features talk to
   `SyncTransport` / repositories — never a specific gateway or
   socket.
7. Assume per-event authz re-check server-side; never widen
   subscriptions past the user's scopes; fail closed when authz
   is unreachable.

**Source:** `sync-doctrine/SKILL.md` (HMA).

---

## §11 · Accessibility (WCAG 2.2 AA — non-negotiable)

- **Contrast ≥ AA** — 4.5:1 body text, 3:1 large text / UI
  components / focus indicators, in **both** themes
- **Every actionable element has an accessible name** and a
  visible label or tooltip
- **Keyboard reachable and operable** — full flow with Tab /
  Shift-Tab / Enter / Escape; logical focus order; no
  keyboard trap; visible focus ring
- **Images / media have alt / label** — decorative marked as
  such; informative described
- **Reduced motion honored** — `prefers-reduced-motion`
- **Live regions announce streaming** — aria-live for
  streaming text / thinking / status
- **Target size ≥ 24×24 CSS px** (AA 2.5.8), ≥ 44px recommended
  for primary touch
- **Status not by color alone** — pair color with icon/text

**Source:** `a11y-gate/SKILL.md` (HMA).

**Enforcement:** `scripts/audit-a11y.sh` (per-file heuristic:
`aria-label` on buttons, no `tabindex>0`, no `outline: none`
without replacement).

---

## §12 · UI review — screenshot-driven

After every UI change, capture and review screenshots at
**320 / 768 / 1024 / 1440 px** wide, in **both light and dark
themes** — **8 shots per themed surface**.

**Per-shot checklist** (the highlights):

- No horizontal overflow
- No layout shift from streaming / async data
- Complete product coverage (no placeholder routes)
- Hierarchy via scale contrast
- Intentional rhythm (spacing varies with meaning)
- Designed hover / focus / active states
- Both themes look intentional (dark ≠ inverted light)
- Flat 2.0: no visible borders / divider lines / layout
  shadows; adjacent regions differ by **background color**
- Touch targets ≥ 44px at 320 / 768 widths

**Source:** `tauri-ui-review/SKILL.md` (HMA).

**Enforcement:** `scripts/audit-ui.sh` (the screenshot loop).

---

## §13 · Secrets — OS secure store only

API keys, OAuth tokens, and any other secret material:

- Go in the OS secure store (Stronghold on Tauri, Keychain on
  macOS, Credential Manager on Windows, Secret Service on Linux)
- **Never** in PEM, PGlite, Zustand, logs, URLs, ordinary
  database columns, images, Compose files, or ConfigMaps
- APIs return **metadata, never key values**

**The Rust side** is the only place that touches the secure
store. TypeScript only knows whether a key *exists*, never its
value.

**Source:** `deploy-hybrid-agentic-stack/SKILL.md` (HMA).

**Enforcement:** `scripts/audit-secret-leak.sh` (no literal
strings that look like API keys; no committed `.env`).

---

## §14 · Build, sign, and release

- `pnpm tauri build --target <target>` for each OS target
- macOS: `codesign --force --sign "Developer ID Application:
  …"` + `xcrun notarytool submit … --wait`
- Windows: `signtool sign /fd sha256 /tr … /td sha256 …`
- Auto-updater signing key generated with `npx
  @tauri-apps/cli signer generate -w ~/.tauri/companion.key`;
  public key in `tauri.conf.json` `plugins.updater.pubkey`;
  private key **only** in CI secret
- Release cadence: weekly patch, monthly minor, quarterly major
- Release notes generated from `CHANGELOG.md`

**Source:** `tauri-react-vite/SKILL.md` (PMP) + §21 of the
architecture doc.

---

## §15 · Dependencies — pinned versions

- No `^` for breaking-deps in `package.json`
- `pnpm install --frozen-lockfile` in CI
- No `*` in `Cargo.toml` version specs
- `cargo update -p <crate>` requires a commit explaining the
  bump
- The dependency-pin-discipline skill is the source of truth

**Source:** `dependency-pin-discipline/SKILL.md` (HMA).

**Enforcement:** `scripts/audit-deps.sh`.

---

## §16 · Hooks (Claude Code)

- The bundled hook runtime is **immutable** (sha256-pinned)
- Every hook command is a `bash -c '…'` with a `--bundle` arg
  and an `EXEC_BUNDLE` env var
- The `UserPromptSubmit` `prompt-karpathy-learning` hook is
  the only one of its kind; do not add another
- The `SubagentStop` hooks use matchers (`assessor`,
  `analyst`, `planner`, `executor`, `reflector`) and a
  `subagent-fallback` for any unknown role

**Source:** `hooks/hooks.json` + `bootstrap-hook-runtime.sh` in
the PMP.

---

## §17 · Anti-patterns (do not do these)

- ❌ `unwrap()` in Rust (use `?` and typed errors)
- ❌ `new ConcreteThing()` in business code (use composition
  root)
- ❌ `@radix-ui/*` imports (use `@base-ui-components/react`)
- ❌ `@tanstack/react-query` (use PEM)
- ❌ `Platform.isIOS` for nav (use `useBreakpoint`)
- ❌ Hardcoded colors / spacing / radii in components (use
  tokens from `theme.css`)
- ❌ `cargo clean` without committing the change to the lockfile
- ❌ Speculative error handling with no observed failure
- ❌ Silent fallback paths (a missing dependency should fail
  closed, not fail open)
- ❌ Hand-edits to generated files (AGENTS.md, CLAUDE.md,
  `theme.css`)

---

## §18 · Skill invocation map

For task → skill, see §24 of `docs/00-architecture-and-implementation-plan.md`.
The most common:

| Task | Skill |
|---|---|
| Add a new service card | `pem-local-first` → `entity-graph-web-shell` |
| Add a new theme color | `hybrid-design-tokens` |
| Add a new shadcn component | `shadcn` MCP (or skill) |
| Add a new top-level route | `mobile-navigation` |
| Add a new A2UI component | `a2ui-surface-contract` |
| Add a new AG-UI event | `agui-event-contract` |
| Add a new P2P message | `axum-agent-gateway` + `peer-profile-sync` |
| Add a new provider | `deploy-hybrid-agentic-stack` (BYOK rules) |
| Add a new hook | `tauri-react-vite` (typed IPC) |
| Add a new Tauri command | `tauri-react-vite` + `rust/error-handling` |
| Add a new Tauri window | `tauri-custom-titlebar` |
| Touch any UI file | `tauri-ui-review` + `a11y-gate` |
| Touch any cross-surface data | `sync-doctrine` |
| Touch any entity | `pem-local-first` |
| Touch any agent | `axum-agent-gateway` |
| Cut a release | `hybrid-runtime-verification` |
| Plan a multi-phase change | `orchestrate-prometheus-application` |
| Generate a new skill | `skill-creator` |
| Refine a skill | `skill-refiner` |

---

## §19 · The pnpm audit umbrella

```bash
pnpm audit:layer        # CLEAN layer rule (§4)
pnpm audit:naming       # kebab-case rule (§3)
pnpm audit:generated    # AGENTS.md / CLAUDE.md are generated (§26)
pnpm audit:tokens       # no hardcoded values in components (§8)
pnpm audit:a11y         # WCAG 2.2 AA per-file heuristic (§11)
pnpm audit:deps         # pinned versions (§15)
pnpm audit:secret-leak  # no API key literals (§13)
pnpm audit:skill-desc   # description rules (§7.1)
pnpm audit:progress     # every process skill has Progress Signals (§2)
pnpm audit:ui           # tauri-ui-review loop (§12)
```

**Run them all:** `pnpm audit` (or `bash scripts/audit-all.sh`).

**Pre-push hook** refuses to push if any audit fails.

---

## §20 · What's NOT in this file

To keep this file resident and high-signal, the following live
elsewhere:

- **Long-form documentation** → `docs/`
- **Architecture** → `docs/00-architecture-and-implementation-plan.md`
- **Dependency table** → `versions.toml`
- **Release notes** → `CHANGELOG.md`
- **ADRs** → `docs/adr/`
- **Skill index** → the marketplace

`AGENTS.md` is the **rule book**, not the **manual**. The manual
is the doc you're reading alongside.

---

*This file is generated from `templates/rules/agents.md.tmpl`.
To change a rule, edit the template, then run
`bash scripts/gen-rule-files.sh`. The CI will fail if a
hand-edit appears in this file.*

<!-- prometheus-base:start v1 -->
# Agent Operating Rules

This region is the standing contract. It holds only invariants that must
survive compaction. Everything else lives in on-demand rules, skills, hooks,
and reference files. Where a hook enforces a rule stated here, the hook wins.

Managed by `prometheus-context-bootstrap`. Edits inside these markers are
overwritten on re-run. Write project prose outside them.

## Position and authority

- `.kbd-orchestrator/current-waypoint.json` is authoritative for position.
- `versions.toml` is authoritative for architecture decisions and dependency pins.
- READMEs go stale. Do not trust one over the two files above.
- Read the waypoint at session start. State the current phase before executing.

## Capability inversion

Agent kernels do not write. Mutating actions are gated in the trusted host
layer only, never in an agent kernel. Where the language allows it, this is
enforced at the dependency graph as a compile-time guarantee rather than a
runtime check. If a task appears to require a write from a kernel, stop and
surface the conflict instead of routing around it.

## Phase order

Task loop: Spec, Plan, Execute, Reflect.
Evolution loop: Compile, Evaluate, Optimize, Promote.

Running a phase out of order is a quality failure, not a shortcut. Name the
phase you are in. Do not execute before a plan exists.

## Verification tiers

Tier 0 every edit. Tier 1 unit complete. Tier 2 phase completion. Tier 3
milestone or release only. Running a tier before its point is a violation, not
diligence. Per-stack commands live in `.claude/rules/`, loaded when a matching
file is read.

- `.claude/rules/typescript.md` — typescript tiers and hard rules

## Evidentiary standard

Address observed problems. An observed problem comes from an operator report, a
visible error or log, a failing test, or an explicit requirement. A concern that
is none of those gets one sentence and a question, never speculative code.

Defensive code — validation, guards, fallbacks, retries, timeouts — requires a
named failure scenario. Hardening at a real trust boundary present in the code
is a standing exception and is named in the completion summary, never added
silently.

## Evidence over assertion

Show the command and its output, the test result, or the artifact. "Looks done"
is not done. Report what was actually run and at which tier. If a check could
not run, say which claims are therefore unverified. An unverified claim reported
as verified is worse than no check at all.

## Anti-sycophancy

Critics never see generation history. Review through the `artifact-critic`
subagent, which receives the artifact alone. The model that produced the work is
not the sole judge of whether it is good.

A reflection leads with the delta between plan and delivery, not with what
worked. The sycophancy gate may block a turn; fix the finding rather than
bypassing it.

## Learning and memory

Learning is append-only under `.prometheus/`: `session-log.md`, `decisions.md`,
`gotchas.md`, `postmortems/`, `knowledge/`. Never rewrite history; append, and
mark superseded entries rather than deleting them.

Write on a decision with a rationale, a defect and its root cause, a learned
constraint, a phase boundary, and a session summary. Read `gotchas.md` before
touching a subsystem.

Where a memory server is configured, it is the primary store and its write path
may time out. On failure, log to the markdown files above and continue. Never
block a task on the memory server.

## Architecture

- Single-writer build discipline within one build or target directory.
- Feature-based organization by capability, not by technical layer.
- Strict layering: UI, then hooks or view models, then stores, then services,
  then external. Reverse flow only through reactive state or events.
- Business state lives in explicit, inspectable systems, never in UI components
  or agent-only memory.
- Open standards first. Avoid lock-in unless explicitly required.
- Verify dependency versions against official sources before introducing them.
  Do not rely on training-era version knowledge.

## Scope

Minimum change that solves the problem. Do not refactor adjacent working code;
treat its current state as intentional. Mention unrelated issues, do not fix
them unasked. Before destructive or hard-to-reverse actions, confirm intent and
prefer a reversible path.

## Skills may be absent

Harnesses drop skill descriptions past a context budget, so a skill you expect
may not be listed. If one is missing, invoke it by name or say plainly that it
is unavailable and proceed from these rules. Never invent what an absent skill
would have done.

## Communication

Direct and execution-first. Structure claims as statement, mechanism, stakes.
Short declarative sentences. No marketing language.

Avoid: leverage as a verb, utilize, synergy, roadmap as a verb, journey,
harness as a verb, delve, revolutionary.

Every significant document names the uncomfortable thing — the scenario that
hurts the author's own position.

## Done

A task is done when its stated exit criteria pass at the current tier, not when
the output looks plausible. Before declaring completion: remove anything added
that was not requested, confirm each guard traces to an observed problem or a
real boundary, and summarize what changed, how it was verified, and what remains
at risk.

## Execution scaffold

This section exists because the fleet is mixed. Frontier models supply most of
it by default; smaller and older models do not, and the failure is silent —
plausible output with a fabricated call in it. Omit this section only when
every model that reads this file is known to supply the behavior on its own.

### Before executing

Restate the task in one sentence, and name the phase. If the restatement does
not match what was asked, stop and ask rather than proceeding on the closer
reading. Name the files you intend to touch before touching them.

### Do not fabricate

Never invent an API, a file path, a package name, a command flag, or a
configuration key. If you have not read it in this session or it is not pinned
in `versions.toml`, verify it before using it. "I could not confirm this
exists" is a correct answer. A plausible identifier that does not exist costs
more than the question would have.

Do not guess at a tool's parameters. Read its schema. A tool call with invented
arguments fails in a way that looks like the tool is broken.

### Verification is explicit

Run the check. Paste the command and its actual output. Do not report a result
you did not observe, and do not describe what a test "should" produce.

If a check cannot run, say which specific claims are therefore unverified, and
why. Skipping a check silently and summarizing as if it passed is the failure
this rule exists to prevent.

### Code output

Never elide code with `...`, `// rest unchanged`, or a similar placeholder in a
file you are writing. Emit the complete content of every file you write.

When editing, change the minimum span. Do not reformat, reorder imports, or
rename adjacent symbols while making an unrelated change.

Match the file's existing conventions over your own defaults.

### One thing at a time

Complete one edit and its cheap check before starting the next. Do not batch
several unrelated changes into one pass and verify at the end — when it fails
you will not know which change caused it.

Do not start a second subsystem while the first is unverified.

### Stop conditions

Stop and ask when: the requirement is ambiguous in a way that changes the
design, two readings of the task lead to different files, the change would
break an existing behavior, or you are about to do something hard to reverse.

Stop when the goal is met. Do not continue into adjacent improvements.

### Format contracts

When a specific output format is requested — JSON, a table, a diff, a schema —
emit exactly that format with no preamble, no trailing commentary, and no
markdown fence unless the fence was asked for. A parser is often reading it.

### Self-check before reporting completion

State each of these explicitly, not as a claim that you did them:

1. What changed, file by file.
2. What was run to verify it, and the observed output.
3. What was added that was not requested — remove it, or list it and ask.
4. Which guards trace to an observed failure, and which do not.
5. What remains unverified, and why.

<!-- profile: mixed — see references/MODEL-PROFILES.md before changing -->
<!-- prometheus-base:end -->
