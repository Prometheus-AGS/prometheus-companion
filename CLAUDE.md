@AGENTS.md

<!-- GENERATED — DO NOT EDIT — source: templates/rules/claude.md.tmpl
     Regenerate with: bash scripts/gen-rule-files.sh
     Verify with:     bash scripts/audit-generated.sh -->

# CLAUDE.md — Prometheus Companion (Claude Code)

> **Inherits every rule from `AGENTS.md` at the project root.**
> Read `AGENTS.md` first. This file adds Claude-Code-specific
> behavior on top.
>
> Claude Code-specific rules: the skill triggers, the
> `EnterPlanMode` workflow, the PostToolUse reminder hooks, the
> AG-UI memory format, and the `[compact]` block.

---

## §1 · The `AGENTS.md` inheritance

**Every rule in `AGENTS.md` applies here, in full.** The local
rules in this file are **additive**, never overrides. When in
doubt, the more conservative of the two applies.

In particular, the seven Constitution rules (§0 of AGENTS.md) are
**inviolable**. Compaction, context loss, or any other excuse
does not suspend them.

---

## §2 · Skill triggers

These phrases (and their semantic equivalents) auto-fire a skill.
The list is closed — if you want to add a trigger, edit the
`description:` frontmatter of the target skill, not this file.

| Phrase (and equivalents) | Fires |
|---|---|
| "title bar", "window chrome", "decorations false", "drag region", "traffic lights", "window controls", "frameless" | `tauri-custom-titlebar` |
| "design token", "theme", "palette", "color", "spacing", "typography", "shadcn theme", "CSS variable", "dark mode" | `hybrid-design-tokens` |
| "accessibility", "a11y", "WCAG", "screen reader", "keyboard navigation", "focus", "aria", "contrast", "alt text", "tab order" | `a11y-gate` |
| "Tauri UI", "desktop UI", "React component", "screen", "page", "layout", "responsive", "breakpoint", "screenshot review", "visual check", "does it look right" | `tauri-ui-review` |
| "skill", "skills", "select skill", "skill selection", "skill hit rate", "skill description", "skill authoring" | `clean-skill` |
| "clean architecture", "CLEAN", "4-layer", "domain", "application", "infrastructure", "presentation", "layer", "use case" | `clean-architecture` |
| "navigation", "nav bar", "navigation bar", "bottom nav", "tab bar", "tabs", "NavigationBar", "NavigationRail", "BottomNavigationBar", "TabBar", "rail", "sidebar", "app shell", "destinations", "GoRouter ShellRoute", "TanStack Router layout" | `mobile-navigation` |
| "PEM", "entity", "entities", "PGlite", "TanStack Query", "SWR", "prometheus-entity-management", "registerEntityTransport", "useEntities" | `pem-local-first` |
| "A2UI", "agent UI", "surface", "projection", "registry", "component type" | `a2ui-surface-contract` |
| "AG-UI", "agui", "AG UI", "CopilotKit", "SSE", "streaming events", "run events" | `agui-event-contract` |
| "Axum", "gateway", "agent gateway", "UAR", "agent runtime", "verified session", "tenant", "Cedar" | `axum-agent-gateway` |
| "sync", "replication", "realtime", "offline", "local-first", "subscription", "write queue", "offline write", "conflict", "LWW", "CRDT", "ElectricSQL" | `sync-doctrine` |
| "build", "release", "deploy", "code-sign", "notarize", "Authenticode", "DMG", "MSI", "Tauri signer" | `tauri-react-vite` (the build pattern) |
| "dep update", "bump", "version", "lockfile", "frozen lockfile" | `dependency-pin-discipline` |
| "Tauri command", "Rust backend", "RUST", "src-tauri", "error handling", "thiserror", "anyhow", "actor", "mpsc" | `tauri-react-vite` + `rust/error-handling` + `rust/async-patterns` + `rust/actor-model` |
| "plan a multi-phase change", "orchestrate", "KBD", "Feynman", "Karpathy" | `orchestrate-prometheus-application` |
| "release evidence", "completion gate", "real launch", "public boundary" | `hybrid-runtime-verification` |
| "generate a new skill", "create a skill" | `skill-creator` |
| "refine a skill", "fix a skill" | `skill-refiner` |
| "ecosystem navigation", "find the right skill", "what skill do I use" | `using-agent-skills` |

---

## §3 · The `EnterPlanMode` workflow

When the task is non-trivial, **plan before code**:

1. **Trigger `EnterPlanMode`** when the task:
   - Touches more than one file
   - Adds a new dependency or a new env var
   - Changes the layer rule (§4 of AGENTS.md) or the kebab-case
     rule (§3 of AGENTS.md)
   - Could affect the architecture, build, or release process
   - Has meaningful ambiguity about *what* to do (not just *how*)
2. **Plan content** (use this template):

   ```markdown
   ## Goal
   <one sentence>

   ## Affected subsystems
   - <list>

   ## Approach
   <2-5 bullet summary>

   ## File changes
   - `path/to/file.tsx` — <what>
   - `path/to/other.ts` — <what>

   ## Risks
   - <risks>

   ## Verification
   - `pnpm audit:layer` exits 0
   - `pnpm audit:naming` exits 0
   - <other concrete checks>
   ```

3. **Wait for the user's approval** before executing.
4. **Execute the plan** in one focused pass; do not expand the
   scope.

When the task is trivial (one file, known pattern, single
command), **just do it** — don't ask permission to do a small
thing.

---

## §4 · PostToolUse reminder hooks

The bundled hook chain in `hooks/hooks.json` includes two
advisory reminders that fire after every `Write` / `Edit` to a
UI file (`.tsx` / `.jsx` under `src/`, `.css` under `src/styles/`):

- **`posttool-ui-review-reminder`** — runs `tauri-ui-review`
  reminder ("did you capture the 8 screenshots?")
- **`posttool-a11y-reminder`** — runs `a11y-gate` reminder
  ("did you check the WCAG 2.2 AA checklist?")

The reminders are **advisory, non-blocking**. They do not judge
the code; they ensure the gates are not silently skipped. You
must run the gates.

The reminder messages end with the literal token `RUN_GATE`. If
you see `RUN_GATE` in the hook output, the corresponding
advisory applies.

---

## §5 · The AG-UI memory format

When you have a non-trivial finding (a bug, a fix, a gotcha, an
observation) that should outlive this session, write it to:

```
~/.prometheus/projects/<projectId>/notes/<YYYY-MM-DD>--<slug>.md
```

The file format:

```markdown
---
date: 2026-08-20
session: <sessionId>
agent: <agentName>
tags: [<comma-separated>]
---

# <one-line title>

<2-5 sentences of context, observation, or fix>

## Steps to reproduce (if bug)

1. ...

## Fix

1. ...

## Why this is durable

<one sentence on why a future session needs this>
```

**When to write a note:**

- A fix for a bug that took > 10 minutes to find
- A subtle gotcha that is not in the docs
- An architectural decision that was made
- A tool or skill that the user prefers

**When NOT to write a note:**

- Trivial fixes that are obvious from the diff
- One-off observations
- Anything that will be obvious from the AGENTS.md or the
  architecture doc

---

## §6 · The `[compact]` block

When the session is getting long (typically > 50 tool calls, or
> 75% of the context window), end the next assistant turn with a
`[compact]` block:

```markdown
[compact]
## Position
- Phase: <X> of <Y>
- Task: <name> in <plan>
- Waypoint: <path to current waypoint file>

## Done
- <list of completed tasks>

## Open
- <list of in-progress or pending tasks>

## Key context
- <2-4 bullets of state that survives compaction>

## Next action
- <the single next thing to do>
[/compact]
```

The `[compact]` block is the canonical "I'm about to lose
context" handoff. The next session reads `position-reminder.txt`
and resumes from the "Next action" line.

---

## §7 · The pnpm / bun choice

This project uses **`pnpm`** (not `npm`, not `bun`, not `yarn`).
The reason is matching the rest of the PMP stack.

```bash
pnpm install                  # never npm install
pnpm tauri dev                # never npx tauri dev
pnpm run <script>             # never npm run <script>
```

`corepack enable pnpm` is in the bootstrap.

---

## §8 · The "extend without rebuilding" pattern

The user wants to **extend the Companion from external
directories** (sibling repos, third-party plugins, local
customizations) **without rebuilding the core**.

**The pattern is:**

- **Local extra skills** go in
  `~/.claude/skills/<skill-name>/SKILL.md` (the
  `additionalDirectories` Claude Code setting)
- **Marketplace plugins** go in the `marketplace.json` of the
  Companion and are installed via `/plugin install
  <name>@<marketplace>`
- **Substrate plugins** are feature-gated Rust crates in the
  superworkspace (see §4 of the architecture doc)
- **Tauri custom commands** go in `src-tauri/src/commands/`
  with a kebab-case file name and a typed `tauri-commands.ts`
  wrapper in infrastructure

**Do not** add new functionality by editing the core files
directly. Add a new file in the right layer (domain, application,
infrastructure, presentation), wire it in the composition root,
and let the audit scripts confirm you didn't violate the rules.

---

## §9 · The `~/.prometheus/projects/` schema

This project lives under a Prometheus project record. The schema:

```
~/.prometheus/projects/<projectId>/
├── project.json                ← { schemaVersion, projectId, repositoryFingerprint }
├── knowledge/                  ← read-only symlink to the wiki (Karpathy)
├── events.jsonl                ← append-only session event log
├── traces/                     ← optional OpenTelemetry traces
├── wiki/                       ← local wiki mirror
└── notes/                      ← AG-UI memory format (§5)
```

**`projectId`** is the canonical identity. The
`repositoryFingerprint` is a sha256 of the git remote URL plus
HEAD commit, used to detect cross-project bleed.

When you write a note, a snapshot, or a learning record, prefix
the path with `~/.prometheus/projects/<projectId>/` so it lands
in this project's record and not in a global one.

---

## §10 · Quick reference — Claude Code commands

| Need | Command |
|---|---|
| Find a skill | `/plugin marketplace add <owner>/<repo>` then `/plugin install <name>` |
| Plan before code | `EnterPlanMode` |
| Capture a screenshot for `tauri-ui-review` | `pnpm audit:ui` (uses BrowserClaw under the hood) |
| Run all audits | `pnpm audit` |
| Generate a new skill | `/skill-creator <name>` |
| Refine a skill | `/skill-refiner <skill-path>` |
| Generate rule files | `bash scripts/gen-rule-files.sh` |
| Reset a stuck install | `bash scripts/install-mcp-services.sh --restart` |
| Pair a new device | Tray menu → "Pair new device" (handled by `pairing` use case) |
| Sign a release | `pnpm tauri build --target <target> && signtool / codesign` |

---

*This file is generated from `templates/rules/claude.md.tmpl`.
To change a rule, edit the template, then run
`bash scripts/gen-rule-files.sh`. The CI will fail if a
hand-edit appears in this file. The full rule set is in
`AGENTS.md`.*
