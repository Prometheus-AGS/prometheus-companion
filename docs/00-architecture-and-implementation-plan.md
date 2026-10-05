> Current source status (2026-10-05): this document is retained design history,
> not a shipped capability list or permission to run its older build/test workflow.
> Current operational source and pending release gates are described in
> [the repository README](../README.md), [installation](installation.md),
> [control APIs](control-api.md) and [maintenance](maintenance.md). The main
> window remains the starter UI; service-health tray and headless/control/P2P
> wiring exist in source, pending local integration. UI/mobile roadmap remains
> separate work. Canonical implementation-first/local integration policy governs.

# Prometheus Companion — Architecture, Functional Specification, and Implementation Plan

> **Repo:** `/Users/gqadonis/Projects/prometheus/prometheus-companion/`
> **Author:** Mavis (`MiniMax-M3`) for Travis James / Prometheus AGS
> **Date:** 2026-08-20
> **Status:** v0.1.0 — first draft for review
> **Supersedes:** the Tauri 2.0 scaffold created by `pnpm create tauri-app`
> **Companion to:** the architecture review at
> `prometheus-skill-pack/docs/audits/2026-08-20-skill-pack-architecture-review.md`
> (specifically **Pillar 5** in §12 — the Companion app)
> **TJ-ARCH-MOB-001 compliant** — applies the constraints from
> `/Users/gqadonis/Projects/hybrid-mobile-architecture-src/.kimi-code/skills/`

---

## Table of contents

0. [Vision and one-screen summary](#0--vision-and-one-screen-summary)
1. [How to read this document](#1--how-to-read-this-document)
2. [Source-of-truth skills used to build this spec](#2--source-of-truth-skills-used-to-build-this-spec)
3. [High-level architecture](#3--high-level-architecture)
4. [Process model and binary topology](#4--process-model-and-binary-topology)
5. [Tauri configuration (`tauri.conf.json`)](#5--tauri-configuration-tauriconfjson)
6. [Window model — custom title bar, decorations, controls](#6--window-model--custom-title-bar-decorations-controls)
7. [Frontend architecture (React 19 + Vite 8)](#7--frontend-architecture-react-19--vite-8)
8. [Design tokens (one source for every surface)](#8--design-tokens-one-source-for-every-surface)
9. [shadcn/ui with Base UI (the new default)](#9--shadcnui-with-base-ui-the-new-default)
10. [Prometheus Entity Management integration](#10--prometheus-entity-management-integration)
11. [Tray icon — the always-visible dashboard surface](#11--tray-icon--the-always-visible-dashboard-surface)
12. [Main dashboard — the per-service operations view](#12--main-dashboard--the-per-service-operations-view)
13. [A2UI surface — orchestrator-driven UI](#13--a2ui-surface--orchestrator-driven-ui)
14. [P2P pairing, device identity, sovereign-sync integration](#14--p2p-pairing-device-identity-sovereign-sync-integration)
15. [AG-UI / SSE event streaming](#15--ag-ui--sse-event-streaming)
16. [Service supervisor — what the Companion knows about its substrate](#16--service-supervisor--what-the-companion-knows-about-its-substrate)
17. [Provider, model, and credential management](#17--provider-model-and-credential-management)
18. [Storage and sync — local-first, server-authoritative](#18--storage-and-sync--local-first-server-authoritative)
19. [Mobile parity (Pillar 6 — Flutter + Rust over FFI)](#19--mobile-parity-pillar-6--flutter--rust-over-ffi)
20. [Accessibility (WCAG 2.2 AA — non-negotiable)](#20--accessibility-wcag-22-aa--non-negotiable)
21. [Build, sign, and release](#21--build-sign-and-release)
22. [Test plan — tauri-ui-review + a11y-gate + BDD](#22--test-plan--tauri-ui-review--a11y-gate--bdd)
23. [Implementation phases (the punch list)](#23--implementation-phases-the-punch-list)
24. [Skill invocation map (which skill at which step)](#24--skill-invocation-map-which-skill-at-which-step)
25. [Risks, tradeoffs, and open questions](#25--risks-tradeoffs-and-open-questions)
26. [Standards & rule files — `AGENTS.md` and `CLAUDE.md`](#26--standards--rule-files--agentsmd-and-claudemd)
27. [Appendix A — Cargo.toml additions and Rust crate plan](#appendix-a--cargotoml-additions-and-rust-crate-plan)
28. [Appendix B — package.json additions and dependency plan](#appendix-b--packagejson-additions-and-dependency-plan)
29. [Appendix C — file/folder layout (target tree)](#appendix-c--filefolder-layout-target-tree)
30. [Appendix D — implementation sequence with verifiable checkpoints](#appendix-d--implementation-sequence-with-verifiable-checkpoints)

---

## 0 · Vision and one-screen summary

The **Prometheus Companion** is the operator's single pane of glass for
the entire Prometheus skill pack substrate. It replaces seven macOS
LaunchAgents, four stdio MCPs, a Unix-socket exec daemon, a research
server, and the openai-proxy with **one installable, signed, tray-
resident desktop application** that:

- Owns the full lifecycle of the substrate (install, start, supervise,
  upgrade, uninstall)
- Watches every service with a typed health aggregator that drives a
  color-coded tray icon
- Exposes a real React 19 + Vite 8 + Tauri 2.0 dashboard for
  per-service state, log tails, queue depth, and one-click "fix it"
- Pairs the device (P2P identity, Ed25519 over Iroh QUIC/TLS 1.3) and authorizes
  other devices to use this node as an execution host
- Renders an **A2UI surface** so any of the orchestrator skills
  (`kbd-coach`, `kbd-goal-evaluator`, `pmpo-evolver`, etc.) can paint
  interactive UI inside the Companion
- Streams **AG-UI events** from `prometheus-exec` runs back to the
  Assistant-UI thread view
- Manages providers, models, and credentials through Liter-LLM's
  registry without leaking key values
- Targets macOS, Windows, and Linux today; iOS and Android ship
  as a **separate Flutter shell** that talks to the **same Rust
  substrate crates** over FFI via `flutter_rust_bridge`
  (Pillar 6 of the review — see §19)

**Architectural commitments** (each gets a deep section below):

1. **One substrate binary** (in-process library inside the Companion),
   not 15+ processes
2. **Custom title bar** with platform-correct window controls
   (macOS traffic lights left, Windows-Linux min/max/close right)
3. **shadcn/ui with Base UI primitives** (the new shadcn default; not
   Radix)
4. **Prometheus Entity Management (PEM) 3.x** for all client-side
   normalized state; **no TanStack Query**, ever
5. **A2UI surface contract** for orchestrator-driven UI; the Companion
   is a renderer, not an authority
6. **Sovereign-sync + Iroh QUIC** for P2P device pairing; no raw
   libp2p, no central relay
7. **One design-token source** (TOML) compiled to Tailwind 4 and
   shadcn CSS variables, never hand-mirrored
8. **WCAG 2.2 AA** as a build-blocking gate, not an afterthought

**Effort budget:** 10 engineering weeks to v1.0 (Pillar 5 + Pillar 6
of the review's roadmap). The first 4 weeks are the desktop; weeks
5-8 add P2P, A2UI, and the entity management integration; weeks
9-10 add the mobile target and a TestFlight internal build.

---

## 1 · How to read this document

Every section is grounded in one or more **skills**. Each section ends
with a **"Skills used"** callout that names the source-of-truth skill
and what we read out of it. The first time a skill is referenced we
quote the relevant portion; subsequent uses just name the file.

| Acronym | Meaning |
|---|---|
| **PEM** | `@prometheus-ags/prometheus-entity-management` 3.x — the normalized Zustand entity graph |
| **HMA** | Hybrid Mobile Architecture — the source repo at `/Users/gqadonis/Projects/hybrid-mobile-architecture-src/` |
| **PMP** | `prometheus-skill-pack` — the source repo at `/Users/gqadonis/Projects/prometheus/prometheus-skill-pack/` |
| **UAR** | Universal Agent Runtime — the gateway that owns agent execution authority |
| **A2UI** | The Prometheus Agent UI protocol (typed projection events) |
| **AG-UI** | The CopilotKit-style SSE event protocol |
| **MCP** | Model Context Protocol — the substrate's tool surface |
| **Cedar** | The policy engine used for authorization |
| ~~**Noise**~~ | *Not used.* Removed by `change-cpc-006`: sovereign-sync authenticates peers by Ed25519 EndpointId over iroh QUIC/TLS 1.3, never a Noise handshake |

Read this document alongside:

- The architecture review at `prometheus-skill-pack/docs/audits/2026-08-20-skill-pack-architecture-review.md`
- The HMA repo at `/Users/gqadonis/Projects/hybrid-mobile-architecture-src/`
- The PEM bundle index at `prometheus-skill-pack/skills/react/prometheus-entity-skills/SKILLS.md`

---

## 2 · Source-of-truth skills used to build this spec

The Companion is built **by** the skills, not around them. Every
implementation decision below is sourced from a skill in either the
HMA repo or the PMP repo. The full map is in §24.

### 2.1 From `/Users/gqadonis/Projects/hybrid-mobile-architecture-src/.kimi-code/skills/`

| Skill file | What it gives the Companion |
|---|---|
| `hybrid-design-tokens/SKILL.md` | The one source of design tokens (TOML → Tailwind 4 + shadcn CSS vars) |
| `tauri-custom-titlebar/SKILL.md` | The custom title bar pattern: `decorations:false`, `data-tauri-drag-region`, `isTauri()` gating |
| `tauri-ui-review/SKILL.md` | The screenshot-driven review loop at 320/768/1024/1440 in both themes |
| `a11y-gate/SKILL.md` | WCAG 2.2 AA cross-surface checklist, including the `PostToolUse` reminder hook |
| `mobile-navigation/SKILL.md` | Top-level destinations in the bottom bar; switch to rail by width, never by platform |
| `entity-graph-web-shell/SKILL.md` | The PEM invariants — Components → Hooks → Stores → Transports, never the other direction |
| `pem-local-first/SKILL.md` | Why TanStack Query is prohibited; PGlite as the local store |
| `a2ui-surface-contract/SKILL.md` | A2UI is a projection, not authority; render-only, route actions back through UAR |
| `agui-event-contract/SKILL.md` | AG-UI SSE event shapes; correlation IDs; resume semantics |
| `axum-agent-gateway/SKILL.md` | The Axum boundary contract between the browser and the agent runtime |
| `sync-doctrine/SKILL.md` | Lane choice (relational / CRDT / event-log) and the seven invariants |
| `peer-profile-sync/SKILL.md` | CRDT profile vault pattern for the cross-device P2P identity |
| `deploy-hybrid-agentic-stack/SKILL.md` | The deployment profiles (`local` / `web` / `realtime` / `authenticated` / `full-agentic`) |
| `orchestrate-prometheus-application/SKILL.md` | The PMPO control loop; Karpathy progress recording at phase boundaries |
| `hybrid-runtime-verification/SKILL.md` | The completion gate: real launch, persistence, public boundary, health/readiness |

### 2.2 From `/Users/gqadonis/Projects/prometheus/prometheus-skill-pack/skills/`

| Skill | What it gives the Companion |
|---|---|
| `tauri/tauri-react-vite/SKILL.md` | The Tauri command + typed IPC wrapper pattern (`src/lib/tauri.ts`) |
| `react/react-vite-stack/SKILL.md` | React 19 + Vite 8 + TanStack Router + Zustand 5 + shadcn/ui |
| `react/prometheus-entity-skills/SKILLS.md` | The PEM plugin map (setup, crud, graphql, realtime, prisma, optimize) |
| `learn/ui-surface/SKILL.md` | The cross-harness UI surface resolver; how `learn-*` skills render inside the Companion |
| `process/native-agent/SKILL.md` | The Supabase-style management CLI pattern the Companion mirrors |
| `process/pmpo-evolver/SKILL.md` | The 5-perspective strategy router; the Darwin three-gate validation |
| `process/iterative-evolver/SKILL.md` | Assess → Analyze → Plan → Execute → Reflect loop |
| `rust/error-handling/SKILL.md` | `thiserror`/`anyhow` boundary, `#[cold]` error paths, no `unwrap()` |
| `rust/async-patterns/SKILL.md` | `Arc<RwLock>`, `parking_lot`, broadcast channels, graceful shutdown |
| `rust/workspace-structure/SKILL.md` | `resolver=2`, domain-driven crate decomposition |
| `rust/mcp-server/SKILL.md` | The JSON-RPC 2.0 dispatch + SSE + stdio transport |
| `rust/actor-model/SKILL.md` | mpsc-based actor pattern for the service supervisor |

### 2.3 What is intentionally NOT used

| Not used | Why |
|---|---|
| `tanstack/react-query` | Forbidden by `pem-local-first` — it is a request cache over a data layer that already knows freshness |
| `radix-ui` (the npm package) | The new shadcn default is Base UI; we use Base UI for everything |
| `capacitor` (for mobile) | Mobile is Flutter + Rust over FFI, not Capacitor. Capacitor adds a web layer and latency for no architectural benefit. |
| `tauri 2.0 mobile` (for mobile) | Mobile is Flutter + Rust over FFI, not Tauri 2.0 mobile. Tauri is the **desktop** shell (Pillar 5). |
| `libp2p` directly | Sovereign-sync already uses Iroh; the Companion consumes sovereign-sync |
| `surreal` as a sidecar binary | Replaced by the in-process substrate (see §4) |

---

## 3 · High-level architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│  Prometheus Companion (Tauri 2.0 desktop app, OS-tray resident)        │
│                                                                         │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │  WebView: React 19 + Vite 8 + shadcn/ui (Base UI)               │  │
│  │                                                                  │  │
│  │   ┌──────────────┐  ┌──────────────┐  ┌────────────────────┐    │  │
│  │   │  Title bar   │  │  Tray pop    │  │  Main dashboard    │    │  │
│  │   │  (custom,    │  │  (status     │  │  (per-service,     │    │  │
│  │   │  decorations│  │  aggregator, │  │  queue, logs,      │    │  │
│  │   │  :false)     │  │  P2P menu)   │  │  devices, A2UI)    │    │  │
│  │   └──────┬───────┘  └──────┬───────┘  └─────────┬──────────┘    │  │
│  │          │                 │                    │                │  │
│  │          └──────────┬──────┴────────────────────┘                │  │
│  │                     ▼                                           │  │
│  │   PEM 3.x stores: services, devices, runs, skills, providers    │  │
│  │   (Zustand over PGlite; one transport per entity type)          │  │
│  │                     │                                           │  │
│  └─────────────────────┼───────────────────────────────────────────┘  │
│                        │  Tauri commands (typed, async)               │
│                        ▼                                              │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │  Rust side: prometheus-companion_lib (Tauri 2 commands + state) │  │
│  │                                                                  │  │
│  │   ┌────────────────────────────────────────────────────────────┐ │  │
│  │   │  prometheus-substrate (in-process library)                 │ │  │
│  │   │  - service supervisor (process tree, restart on exit)      │ │  │
│  │   │  - prometheus-exec (signed receipts, identity)             │ │  │
│  │   │  - prometheus-knowledge (Karpathy KB)                      │ │  │
│  │   │  - forge-rs (enrichment)                                   │ │  │
│  │   │  - liter-llm (model router)                                │ │  │
│  │   │  - sovereign-sync (Ed25519 + Iroh QUIC)                    │ │  │
│  │   │  - surface-bridge (A2UI projector)                         │ │  │
│  │   │  - prometheus-research (this server's server)              │ │  │
│  │   │  - skill-index (semantic router)                           │ │  │
│  │   └────────────────────────────────────────────────────────────┘ │  │
│  │                                                                  │  │
│  │   external daemons (when not in-process):                        │  │
│  │   ┌──────────────────────────┐  ┌──────────────────────────┐    │  │
│  │   │  surreal  (:28000)       │  │  openai-proxy  (:8181)   │    │  │
│  │   │  (sidecar)               │  │  (external — not ours)   │    │  │
│  │   └──────────────────────────┘  └──────────────────────────┘    │  │
│  └──────────────────────────────────────────────────────────────────┘  │
│                                                                         │
│  Tray icon (system tray) ◀── live color from health aggregator        │
│  A2UI / AG-UI over QUIC  ◀── paired devices, mobile, other Companions │
└─────────────────────────────────────────────────────────────────────────┘
```

The single binary. The substrate library is in-process inside the
Companion (the Antigravity-Tools pattern: "in-process library, not
sidecar"). External daemons (`surreal`, `openai-proxy`) are spawned
as Tauri sidecars only if the user opts out of the in-process
configuration; the in-process path is the default and recommended.

### Skills used
- `tauri-react-vite/SKILL.md` — the IPC command + state pattern
- `entity-graph-web-shell/SKILL.md` — the layered invariants

---

## 4 · Process model and binary topology

The 1.7.0 install of the pack ships **15+** long-running processes
(see architecture review §3.1). **Amended by D-02 / `change-cpc-007`:** the
Companion does not collapse those processes into itself. The pack's services
keep running exactly where the pack installed them (D-02: the pack is open
source and fully functional with no Companion present); this app's own
substrate crate runs in-process and exposes typed Tauri commands to the
frontend, and it **adopts** the pack's services — probing and restarting them
through the platform supervisor — rather than linking them in. §16.4 has the
full picture of what runs where.

### 4.1 The 4-process runtime

| Process | Identity | Started by | Stops on |
|---|---|---|---|
| **prometheus-companion** | Tauri 2.0 desktop app | `launchd` (via `ai.prometheus.substrate.plist`) on macOS, user systemd on Linux, scheduled task on Windows | user "Quit Companion" or OS shutdown |
| **prometheus-companion helper** (macOS) | `LSUIElement` helper to ensure menu-bar icon | bundled with the app | quit Companion |
| **surreal** (sidecar, optional) | SurrealDB 3.2.0 native binary | Tauri `externalBin`, supervised by prometheus-companion | quit Companion (graceful) |
| **openai-proxy** (external, not ours) | The third-party proxy at :8181 | user's own setup (out of scope) | user |

**Amended by `change-cpc-005`.** The Companion hosts the control-plane node
**in-process**: `prometheus_substrate::node::spawn` is called from the Tauri
`setup` hook and binds a Unix socket. There is no extra process. A second
entry point, `prometheus-companion-headless`, runs the *same* node with no
window and no Stronghold vault — that binary is what the two-process
integration harness launches and what a server-side deployment runs.

**Socket allocation** (previously unspecified — the spec named only surreal
`:28000` and openai-proxy `:8181`). The node binds
`<data_local_dir>/prometheus/run/sovereign-sync.sock`, overridable with
`SOVEREIGN_SYNC_SOCKET`. That is seam 1 of the pack's integration contract, and
the path is what every installed `prometheus` CLI already discovers. The name
`sovereign-sync.sock` is kept in contract v1 for CLI compatibility; it is not a
claim about which binary is listening. No TCP port is allocated: `--tcp` remains
available on the daemon for hosts without Unix sockets, gated behind a
mode-0600 bearer token.

Startup order is load-bearing and is the daemon's own: bind the socket and
answer `/health` **first**, then open canonical authority, then install the
full router. If authority fails the node keeps serving diagnostics rather than
dying silently, so an operator can still ask what went wrong.

**MCP.** `sovereign-sync --mode mcp` moved to this repository with the crate
(`change-cpc-004`) and the Companion's installer owns its registration. The 4
other stdio MCPs (`sycophancy-correction`, `liter-llm`,
`sequential-thinking`, `tavily`) remain stdio because they are
spawned by the AI client (Claude Code, Codex, etc.), not by the
Companion. The Companion's relationship to those is
**observation** (they don't bind a port the Companion can probe) and
**configuration** (the Companion writes the `.mcp.json` that the AI
client reads).

### 4.2 The substrate library structure

```
crates/prometheus-substrate/
├── Cargo.toml                      # workspace member of prometheus-skill-pack superworkspace
├── src/
│   ├── lib.rs                      # public API: Substrate::new(), start(), shutdown()
│   ├── supervisor.rs               # process tree, restart on exit, health aggregator
│   ├── receipt_bus.rs              # bus of signed receipts from prometheus-exec runs
│   ├── platform_id.rs              # this device's identity (Ed25519, sovereign-sync node id)
│   ├── a2ui.rs                     # A2UI projector (subscribes to substrate events)
│   ├── agui.rs                     # AG-UI SSE emitter
│   ├── plugin.rs                   # plugin lifecycle (load, version, hot-reload)
│   └── services/
│       ├── mod.rs
│       ├── prometheus_exec.rs      # the receipt-bearing exec daemon, in-process
│       ├── prometheus_knowledge.rs # Karpathy KB, in-process
│       ├── forge.rs                # forge-rs enrichment
│       ├── liter_llm.rs            # model router
│       ├── sovereign_sync.rs       # P2P transport
│       ├── surface_bridge.rs       # A2UI bridge
│       ├── prometheus_research.rs  # the deep-research server, gated by feature flag
│       └── skill_index.rs          # the semantic router
```

The substrate is **feature-gated** so the same crate compiles for
desktop, mobile, and headless. See Appendix A for the feature matrix.

### 4.3 What runs on first launch

```
1. App boots.
2. Substrate::new() with paths from $PROMETHEUS_HOME or ~/.prometheus
3. Substrate reads ~/.prometheus/substrate/state.toml
   - if missing: this is a first install
   - if present: restore the supervisor state, resume supervision
4. Substrate supervisor spawns surreal if sidecar mode
5. Substrate supervisor opens UDS for prometheus-exec
6. Substrate reads device identity (Ed25519 key in OS secure store)
   - if missing: first run on this device; show "Pair this device" flow
7. Frontend mounts, opens its main window (decorations: false), tray
   icon turns gray (substrate initializing)
8. Health aggregator polls each service; once all "ready", tray icon
   turns green
```

### 4.4 What happens on user "Quit Companion"

```
1. Frontend emits 'app:before-quit' event
2. Substrate gracefully drains the run queue
3. Supervisor sends SIGTERM to children, then SIGKILL after 5s
4. Substrate persists state.toml
5. Tauri exits
```

### Skills used
- `rust/workspace-structure/SKILL.md` — `resolver=2`, workspace layout
- `rust/actor-model/SKILL.md` — mpsc actor for the supervisor
- `rust/async-patterns/SKILL.md` — graceful shutdown with `tokio::sync::broadcast`

---

## 5 · Tauri configuration (`tauri.conf.json`)

The current scaffold is the bare `pnpm create tauri-app` output. The
full config below is the target. Each change cites the skill that
mandates it.

```jsonc
// src-tauri/tauri.conf.json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "Prometheus Companion",
  "version": "0.1.0",
  "identifier": "ai.prometheusags.companion",
  "build": {
    "beforeDevCommand": "pnpm dev",
    "devUrl": "http://localhost:1420",
    "beforeBuildCommand": "pnpm build",
    "frontendDist": "../dist"
  },
  "app": {
    "withGlobalTauri": false,                  // import @tauri-apps/api instead
    "macOSPrivateApi": true,                   // required for transparent window-vibrancy
    "windows": [
      {
        "label": "main",
        "title": "Prometheus Companion",
        "decorations": false,                  // we own the title bar
        "transparent": false,
        "width": 1180,
        "height": 760,
        "minWidth": 880,
        "minHeight": 560,
        "resizable": true,
        "fullscreen": false,
        "center": true,
        "visible": false,                      // tray-resident; open via tray menu
        "titleBarStyle": "Overlay",            // macOS: hide native traffic lights
        "hiddenTitle": true,                   // macOS: do not show window title
        "shadow": true
      },
      {
        "label": "tray-popover",
        "title": "Prometheus",
        "decorations": false,
        "transparent": true,                   // macOS vibrancy on the popover
        "width": 360,
        "height": 480,
        "resizable": false,
        "skipTaskbar": true,
        "alwaysOnTop": true,
        "visible": false,
        "focus": false
      }
    ],
    "security": {
      "csp": "default-src 'self'; connect-src ipc: http://ipc.localhost; img-src 'self' asset: data:; style-src 'self' 'unsafe-inline'; script-src 'self'; font-src 'self' data:"
    }
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": ["icons/32x32.png", "icons/128x128.png", "icons/icon.icns", "icons/icon.ico"],
    "externalBin": [
      "binaries/prometheus-substrate-helper"   // only if we ever ship a separable helper
    ],
    "category": "DeveloperTool",
    "shortDescription": "Operator's tray-resident dashboard for the Prometheus skill pack",
    "longDescription": "...",
    "macOS": {
      "minimumSystemVersion": "13.0",
      "entitlements": "entitlements.plist",
      "providerShortName": null,
      "dmg": { "windowSize": { "width": 660, "height": 400 } }
    },
    "resources": []
  },
  "plugins": {
    "opener": { "requireLiteralLeadingDot": false }
  }
}
```

The non-obvious decisions:

- **`decorations: false` on the main window** — required by
  `tauri-custom-titlebar/SKILL.md`; we own the title bar
- **`titleBarStyle: "Overlay"` + `hiddenTitle: true`** — macOS
  correctly hides the traffic lights so we can place our own
  red/yellow/green controls in the title bar
- **Two windows** — `main` for the dashboard, `tray-popover` for
  the at-a-glance status; both frameless
- **`visible: false` on main** — tray-resident UX; the window
  appears on tray click, not on launch
- **CSP is tight** — only `ipc:` and `http://ipc.localhost` for
  `connect-src`; this is the prod CSP, the dev one is wider
- **No `withGlobalTauri`** — the new tauri-react-vite pattern uses
  ES module imports everywhere

### Skills used
- `tauri-custom-titlebar/SKILL.md` (the configuration checklist)
- `tauri-react-vite/SKILL.md` (the `withGlobalTauri: false` pattern)

---

## 6 · Window model — custom title bar, decorations, controls

This is the user's #1 ask. The full HMA titlebar skill is the source
of truth; we adapt it to the Companion's specific needs.

### 6.1 The decision (per the HMA titlebar skill)

| Surface | Window chrome? | What the Companion does |
|---|---|---|
| **Tauri desktop** (macOS/Windows/Linux) | Owned by us | Full custom title bar (this section) |
| **Web / PWA** (served as a fallback) | Browser owns it | **No title bar.** A plain in-app header with the brand lockup and *no* window controls |
| **iOS / Android** (Pillar 6) | OS owns it | **No title bar.** A platform app bar per the `mobile-navigation` skill |

The title bar is **conditionally rendered** in the React tree, never
unconditionally imported, because every Tauri API throws
synchronously at module scope in a plain web page.

### 6.2 The component contract

> **File naming:** every TypeScript file in this project is **kebab-case**.
> React component symbols remain PascalCase (`Titlebar`,
> `MacOSTrafficLights`), but the files that contain them are
> `titlebar.tsx`, `macos-traffic-lights.tsx`. See §7.0 and `AGENTS.md`.

```tsx
// src/presentation/components/titlebar/titlebar.tsx
import { isTauri } from '@tauri-apps/api/core'
import { getCurrentWindow, type Window as TauriWindow } from '@tauri-apps/api/window'
import { platform } from '@tauri-apps/plugin-os'
import { TrafficButton } from './traffic-button'
import { MacOSTrafficLights } from './macos-traffic-lights'
import { WindowsLinuxControls } from './windows-linux-controls'
import { PrometheusMark } from '@/presentation/components/brand/prometheus-mark'

export function Titlebar({ children }: { children?: React.ReactNode }) {
  // CRITICAL: gating at module load would throw on web. Defer resolution.
  const isTauriEnv = isTauri()
  const platformName = isTauriEnv ? safePlatform() : 'web'
  const appWindow: TauriWindow | null = isTauriEnv ? getCurrentWindow() : null

  // Don't render anything on non-Tauri surfaces
  if (!isTauriEnv) return null

  const startDrag = (e: React.MouseEvent) => {
    if (e.button !== 0) return       // left button only
    void appWindow?.startDragging()
  }

  return (
    <header
      data-tauri-drag-region
      onMouseDown={startDrag}
      className="titlebar select-none h-9 flex items-center bg-(--chrome) text-(--text) border-b border-(--divider)"
    >
      {/* Left: brand lockup */}
      <div className="titlebar__brand flex items-center gap-2 pl-3">
        <PrometheusMark className="w-4 h-4 text-(--accent)" />
        <span className="text-sm font-medium tracking-tight">Prometheus Companion</span>
        {children /* optional context chip */}
      </div>

      {/* Spacer: draggable */}
      <div className="flex-1" />

      {/* Middle: contextual title (KBD phase, etc.) */}
      <div className="titlebar__center text-xs text-(--muted)">
        <kbd className="kbd">⌘K</kbd> command palette
      </div>

      {/* Spacer: draggable */}
      <div className="flex-1" />

      {/* Right: platform-correct window controls */}
      <div className="titlebar__controls flex items-center">
        {platformName === 'macos'
          ? <MacOSTrafficLights appWindow={appWindow} />
          : <WindowsLinuxControls appWindow={appWindow} />}
      </div>
    </header>
  )
}

// re-export the type so other presentation files can type-narrow it
export type { TauriWindow }
```

The three sub-components are extracted into their own kebab-case
files:

- `macos-traffic-lights.tsx` — close / minimize / maximize, left
- `windows-linux-controls.tsx` — minimize / maximize / close, right
- `traffic-button.tsx` — shared `<button>` with `aria-label`,
  focus ring, and platform-correct glyph

```tsx
// src/presentation/components/titlebar/macos-traffic-lights.tsx
import type { TauriWindow } from './titlebar'
import { TrafficButton } from './traffic-button'

export function MacOSTrafficLights({ appWindow }: { appWindow: TauriWindow | null }) {
  // Order: close, minimize, maximize. Left side. The OS would draw
  // them at 0,0,8,8; we draw them at 12,0 to make room for the brand.
  // The OS still treats them as system controls; we just restyle.
  return (
    <div className="flex items-center gap-2 pl-2" data-tauri-drag-region={false}>
      <TrafficButton
        label="Close"
        glyph="×"
        className="hover:bg-(--danger) hover:text-(--onDanger)"
        onClick={() => appWindow?.close()}
      />
      <TrafficButton
        label="Minimize"
        glyph="−"
        className="hover:bg-(--surface)"
        onClick={() => appWindow?.minimize()}
      />
      <TrafficButton
        label="Maximize"
        glyph="◻"
        className="hover:bg-(--surface)"
        onClick={() => appWindow?.toggleMaximize()}
      />
    </div>
  )
}

function WindowsLinuxControls({ appWindow }: { appWindow: TauriWindow | null }) {
  // Lives in src/presentation/components/titlebar/windows-linux-controls.tsx.
  // Order: minimize, maximize/restore, close. Right side.
  const [maximized, setMaximized] = useState(false)
  useEffect(() => {
    void appWindow?.onResized(() => {
      void appWindow?.isMaximized().then(setMaximized)
    })
  }, [appWindow])
  return (
    <div className="flex items-center" data-tauri-drag-region={false}>
      <WinButton
        label="Minimize"
        glyph="−"
        onClick={() => appWindow?.minimize()}
      />
      <WinButton
        label={maximized ? 'Restore' : 'Maximize'}
        glyph={maximized ? '❐' : '◻'}
        onClick={() => appWindow?.toggleMaximize()}
      />
      <WinButton
        label="Close"
        glyph="×"
        className="hover:bg-(--danger) hover:text-(--onDanger)"
        onClick={() => appWindow?.close()}
      />
    </div>
  )
}
```

The full TypeScript source — with the Base UI primitives, the
focus-visible ring, the keyboard handler for `Cmd+W` / `Alt+F4`, the
window-state restore on double-click, the popover `Esc` to close,
the OS context menu on right-click — is the next file we write. The
sketch above captures the architectural shape.

### 6.3 Restore on double-click

The native behavior is: double-click the title bar to maximize/
restore. We need to mirror it:

```tsx
const handleDoubleClick = (e: React.MouseEvent) => {
  if (e.button !== 0) return
  // Only act on the empty drag region, not on interactive children
  const target = e.target as HTMLElement
  if (target.closest('button, [role="button"], a, input, select, textarea')) return
  void appWindow?.toggleMaximize()
}
```

### 6.4 Surface-decision sidebar (the crash the titlebar skill warns about)

The title bar component must **never** be imported at the top of a
file that runs on the web. The pattern is:

```tsx
// src/app.tsx
import { isTauri } from '@tauri-apps/api/core'
import { Titlebar } from '@/presentation/components/titlebar/titlebar'  // <- safe: the file gates itself

export function App() {
  return (
    <div className="app">
      <Titlebar />                    {/* returns null on web */}
      <main className="app__main">
        ...
      </main>
    </div>
  )
}
```

`isTauri()` is the runtime check. `Titlebar` itself defers its
`getCurrentWindow()` resolution to inside the function body, never at
module scope. Web builds do not crash.

### 6.5 The "in-app header for web" alternative

The HMA titlebar skill explicitly says: when the design wants a
branded strip on web too, *offer the user the choice* rather than
assuming. The Companion's web fallback does this:

- Default for web: no header at all (the page is a one-screen
  dashboard, no nav needed)
- If the user wants a brand strip on web: a thin `InAppHeader`
  component that uses the *same* brand tokens as the desktop title
  bar, but with no drag region, no window controls, no `isTauri()`
  import

### 6.6 Accessibility

The HMA titlebar skill mandates:

- `aria-label` on every control: `Close`, `Minimize`, `Maximize`,
  `Restore`
- Real `<button>` elements, focus-reachable
- `select-none` on the bar so drag does not start a text selection
- Visible hover / focus / active states (the OS gave these for free;
  we owe them)
- Contrast of bar foreground against its own background (not the
  page's)

The `a11y-gate` skill (§20) is run over the result like any other
UI.

### Skills used
- `tauri-custom-titlebar/SKILL.md` — the canonical pattern
- `a11y-gate/SKILL.md` — accessibility contract

---

## 7 · Frontend architecture (React 19 + Vite 8, CLEAN)

The frontend is a React 19 SPA served by the Tauri WebView. It is
organized as a **strict 4-layer CLEAN architecture** (per the PMP
`clean-architecture/SKILL.md`) with two extra rules adopted from
this project:

- **Kebab-case for every TypeScript file name.** React component
  *symbols* stay PascalCase (`Titlebar`, `ServiceCard`,
  `MacOSTrafficLights`); the *files* that contain them are
  `titlebar.tsx`, `service-card.tsx`, `macos-traffic-lights.tsx`.
  Folder names are also kebab-case. The only exceptions are
  framework-mandated file names (`__root.tsx` from TanStack Router;
  `index.ts` per platform convention).
- **Composition root at `src/main.tsx`.** Every wiring decision
  (which PEM transport, which Tauri command surface, which
  Stronghold client, which PGlite adapter) happens **once**, in
  one file. Nothing in the lower layers is ever `new`-ed from
  business code.

Both rules are encoded in the project-level `AGENTS.md` and
enforced by `scripts/audit-layer.sh` + `scripts/audit-naming.sh`.

### 7.0 The 4-layer CLEAN model (the source of truth)

All source dependencies point inward. Domain has no imports from
Application, Infrastructure, or Presentation. Infrastructure
implements the ports declared by Application. Presentation is the
only layer that touches React.

```
┌──────────────────────────────────────────────────────────────┐
│  Presentation (P)                                            │
│  React components, routes, Zustand UI stores                 │
│  imports from: Application (hooks), Domain (types)           │
│  exports:    <View />, default-exported page components      │
├──────────────────────────────────────────────────────────────┤
│  Application (A)                                             │
│  Use cases (re-start-service, pair-device, invoke-skill),    │
│  application hooks (use-services, use-device-list),          │
│  Zustand transient stores, error-to-toast mapping            │
│  imports from: Domain (entities, errors, ports)              │
│  exports:    use-case functions, hooks, store creators       │
├──────────────────────────────────────────────────────────────┤
│  Infrastructure (I)                                          │
│  Tauri command wrappers, PEM transport registrations,        │
│  PGlite persistence adapter, Stronghold adapter,             │
│  Iroh P2P client, secure-store wrappers, env readers         │
│  imports from: Domain (ports)                                │
│  exports:    concrete implementations of the ports           │
├──────────────────────────────────────────────────────────────┤
│  Domain (D)                                                  │
│  Entity types, value objects, domain errors, port            │
│  interfaces (ServiceRepository, DeviceRepository, …)         │
│  imports from: NOTHING (no React, no Tauri, no PEM)          │
│  exports:    types, value objects, errors, port interfaces   │
└──────────────────────────────────────────────────────────────┘
```

**Forbidden imports** (enforced by `scripts/audit-layer.sh`):

| From layer | Cannot import | Why |
|---|---|---|
| Domain | anything except other Domain | Domain has no I/O and no framework |
| Application | Infrastructure, Presentation | Business logic should not bind to a transport or a UI |
| Infrastructure | Presentation | Adapters are not React components |
| Presentation | Infrastructure (direct) | Components must go through Application |

> **The PEM exception.** The `pem-local-first` rule says
> components consume entity data through PEM hooks, which in our
> CLEAN model is a *Presentation* layer hook. The hook itself lives
> in **Application** (`application/hooks/use-services.ts`) and
> pulls from the PEM store, which is registered as an
> **Infrastructure** adapter (`infrastructure/pem/`). The
> composition root wires them together. Components do not import
> PEM directly.

### 7.1 Folder layout (kebab-case + CLEAN)

```
src/
├── app.tsx                          ← React 19 root, the chrome shell
├── main.tsx                         ← composition root, bootstrap
├── theme.css                        ← GENERATED from design-tokens.toml
├── composition-root.ts              ← wires Infra → App → Presentation
│
├── domain/                          ← pure types, no framework imports
│   ├── entities/                    ← PEM entity definitions
│   │   ├── service.ts
│   │   ├── device.ts
│   │   ├── run.ts
│   │   ├── skill.ts
│   │   ├── provider.ts
│   │   ├── model.ts
│   │   ├── plugin.ts
│   │   ├── setting.ts
│   │   └── queue-item.ts
│   ├── ports/                       ← interfaces (repositories, services)
│   │   ├── service-repository.ts
│   │   ├── device-repository.ts
│   │   ├── run-repository.ts
│   │   ├── skill-repository.ts
│   │   ├── provider-repository.ts
│   │   ├── secret-store.ts
│   │   ├── p2p-transport.ts
│   │   ├── agui-event-bus.ts
│   │   ├── a2ui-projector.ts
│   │   └── supervisor-port.ts
│   ├── value-objects/               ← value types, no behavior
│   │   ├── service-status.ts
│   │   ├── tray-state.ts
│   │   ├── a2ui-envelope.ts
│   │   └── agui-event.ts
│   └── errors/                      ← typed domain errors
│       ├── domain-error.ts
│       ├── supervisor-error.ts
│       └── transport-error.ts
│
├── application/                     ← use cases, hooks, transient stores
│   ├── use-cases/                   ← one use case per file (verb-noun)
│   │   ├── services/
│   │   │   ├── get-aggregate-health.ts
│   │   │   ├── restart-service.ts
│   │   │   ├── open-service-log.ts
│   │   │   └── get-service-detail.ts
│   │   ├── devices/
│   │   │   ├── list-paired-devices.ts
│   │   │   ├── start-pairing.ts
│   │   │   ├── complete-pairing.ts
│   │   │   └── revoke-device.ts
│   │   ├── runs/
│   │   │   ├── list-runs.ts
│   │   │   ├── get-run-receipt.ts
│   │   │   └── verify-receipt.ts
│   │   ├── skills/
│   │   │   ├── list-skills.ts
│   │   │   ├── search-skills.ts
│   │   │   └── invoke-skill.ts
│   │   ├── providers/
│   │   │   ├── list-providers.ts
│   │   │   ├── configure-provider.ts
│   │   │   ├── test-provider-connection.ts
│   │   │   └── delete-api-key.ts
│   │   └── chat/
│   │       ├── send-message.ts
│   │       ├── cancel-run.ts
│   │       └── stream-events.ts
│   ├── hooks/                       ← use-cases-as-hooks (consumed by UI)
│   │   ├── use-services.ts
│   │   ├── use-service.ts
│   │   ├── use-device-list.ts
│   │   ├── use-pairing-flow.ts
│   │   ├── use-runs.ts
│   │   ├── use-skills.ts
│   │   ├── use-providers.ts
│   │   ├── use-theme.ts
│   │   ├── use-tray-state.ts
│   │   └── use-agui-stream.ts
│   ├── stores/                      ← Zustand transient state
│   │   ├── ui-store.ts
│   │   ├── command-palette-store.ts
│   │   ├── toast-store.ts
│   │   └── pairing-store.ts
│   ├── mappers/                     ← port ↔ PEM entity mappers
│   │   ├── service-mapper.ts
│   │   ├── device-mapper.ts
│   │   └── run-mapper.ts
│   └── errors/                      ← error-to-toast mapping
│       └── error-to-toast.ts
│
├── infrastructure/                  ← concrete adapters, no React
│   ├── tauri/
│   │   ├── tauri-commands.ts        ← typed invoke() wrappers
│   │   ├── tauri-events.ts          ← typed listen() subscriptions
│   │   ├── tauri-platform.ts        ← isTauri(), platform() with fallback
│   │   ├── tauri-window.ts          ← getCurrentWindow(), getAllWindows()
│   │   └── tauri-secret-store.ts    ← Stronghold adapter (secret-store port)
│   ├── pem/
│   │   ├── persistence.ts           ← PGlite adapter (createPGlitePersistenceAdapter)
│   │   ├── graph.ts                 ← startLocalFirstGraph() init
│   │   └── transports/              ← one per entity type
│   │       ├── service-transport.ts
│   │       ├── device-transport.ts
│   │       ├── run-transport.ts
│   │       ├── skill-transport.ts
│   │       ├── provider-transport.ts
│   │       └── setting-transport.ts
│   ├── secrets/
│   │   └── api-key-store.ts         ← stronghold adapter (port: secret-store)
│   ├── p2p/
│   │   └── iroh-transport.ts        ← Iroh QUIC transport (port: p2p-transport)
│   ├── agui/
│   │   └── event-bus.ts             ← Tauri event listen port impl
│   ├── a2ui/
│   │   └── projector.ts             ← A2UI event projector (port: a2ui-projector)
│   ├── supervisor/
│   │   └── client.ts                ← Tauri command client (port: supervisor-port)
│   ├── env/
│   │   └── env.ts                   ← typed import.meta.env (zod)
│   └── log/
│       └── logger.ts                ← tracing-subscriber bridge
│
├── presentation/                    ← React components, routes
│   ├── components/
│   │   ├── ui/                      ← shadcn/ui (Base UI) — kebab-case
│   │   │   ├── button.tsx
│   │   │   ├── card.tsx
│   │   │   ├── dialog.tsx
│   │   │   ├── popover.tsx
│   │   │   ├── command.tsx
│   │   │   ├── field.tsx
│   │   │   ├── select.tsx
│   │   │   ├── switch.tsx
│   │   │   ├── badge.tsx
│   │   │   ├── toast.tsx
│   │   │   ├── tooltip.tsx
│   │   │   ├── tabs.tsx
│   │   │   ├── scroll-area.tsx
│   │   │   ├── separator.tsx
│   │   │   ├── input.tsx
│   │   │   ├── textarea.tsx
│   │   │   ├── combobox.tsx
│   │   │   ├── dropdown-menu.tsx
│   │   │   └── index.ts             ← re-exports for clean imports
│   │   ├── titlebar/
│   │   │   ├── titlebar.tsx
│   │   │   ├── macos-traffic-lights.tsx
│   │   │   ├── windows-linux-controls.tsx
│   │   │   ├── traffic-button.tsx
│   │   │   └── prometheus-mark.tsx
│   │   ├── tray/
│   │   │   └── tray-popover.tsx
│   │   ├── dashboard/
│   │   │   ├── service-grid.tsx
│   │   │   ├── service-card.tsx
│   │   │   ├── log-tail.tsx
│   │   │   ├── queue-depth.tsx
│   │   │   └── health-banner.tsx
│   │   ├── a2ui/
│   │   │   ├── a2ui-renderer.tsx
│   │   │   ├── action-router.tsx
│   │   │   ├── a2ui-registry.ts
│   │   │   ├── a2ui-types.ts
│   │   │   └── components/
│   │   │       ├── text-block.tsx
│   │   │       ├── markdown-block.tsx
│   │   │       ├── form-block.tsx
│   │   │       ├── field-block.tsx
│   │   │       ├── switch-block.tsx
│   │   │       ├── dialog-block.tsx
│   │   │       ├── table-block.tsx
│   │   │       ├── card-block.tsx
│   │   │       ├── code-block.tsx
│   │   │       ├── image-block.tsx
│   │   │       └── unknown-block.tsx
│   │   ├── pairing/
│   │   │   ├── qr-display.tsx
│   │   │   ├── device-list.tsx
│   │   │   └── pairing-dialog.tsx
│   │   ├── chat/
│   │   │   ├── thread.tsx
│   │   │   ├── content-blocks.tsx
│   │   │   ├── provider-switcher.tsx
│   │   │   └── a2ui-panel.tsx
│   │   ├── skills/
│   │   │   ├── skill-search.tsx
│   │   │   ├── skill-list.tsx
│   │   │   └── skill-detail-drawer.tsx
│   │   ├── runs/
│   │   │   ├── run-tail.tsx
│   │   │   ├── run-detail-drawer.tsx
│   │   │   └── receipt-verifier.tsx
│   │   ├── settings/
│   │   │   ├── general-settings.tsx
│   │   │   ├── provider-form.tsx
│   │   │   ├── plugin-list.tsx
│   │   │   └── about-panel.tsx
│   │   └── brand/
│   │       └── prometheus-mark.tsx
│   ├── nav/                         ← top-level destinations chrome
│   │   ├── destinations.ts          ← one list, never two (mobile-nav rule)
│   │   ├── bottom-bar.tsx
│   │   ├── nav-rail.tsx
│   │   └── nav-shell.tsx
│   ├── routes/                      ← TanStack Router file-based
│   │   ├── __root.tsx
│   │   ├── index.tsx
│   │   ├── services/
│   │   │   ├── index.tsx
│   │   │   └── $service-id.tsx
│   │   ├── devices/
│   │   │   ├── index.tsx
│   │   │   └── pair.tsx
│   │   ├── chat/
│   │   │   ├── index.tsx
│   │   │   └── $thread-id.tsx
│   │   ├── skills/
│   │   │   ├── index.tsx
│   │   │   └── $skill-id.tsx
│   │   ├── runs/
│   │   │   └── index.tsx
│   │   └── settings/
│   │       ├── index.tsx
│   │       ├── providers.tsx
│   │       ├── plugins.tsx
│   │       └── about.tsx
│   └── theme/
│       └── theme-provider.tsx
│
├── a2ui/                            ← cross-cutting A2UI types & registry
│   ├── registry.ts
│   ├── types.ts
│   └── renderer.tsx
│
└── styles/
    ├── globals.css                  ← @import "tailwindcss"; + theme.css
    └── tailwind.css
```

The `__root.tsx` is a TanStack Router convention (a special file
name the framework expects). The `$service-id.tsx` / `$thread-id.tsx`
files use TanStack's dynamic-segment prefix; the body of the
filename is still kebab-case.

### 7.2 The composition root

Every wiring decision happens in `src/main.tsx` and
`src/composition-root.ts`. Nothing in Domain, Application, or
Infrastructure is ever `new`-ed from anywhere else.

```ts
// src/main.tsx — entry point
import React from 'react'
import ReactDOM from 'react-dom/client'
import { bootstrap } from './composition-root'
import { App } from './app'

const ctx = bootstrap()  // wires everything once
ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App ctx={ctx} />
  </React.StrictMode>
)
```

```ts
// src/composition-root.ts — the wiring
import { getCurrentWindow } from '@tauri-apps/api/window'
import { createPGlitePersistenceAdapter } from '@prometheus-ags/prometheus-entity-management/persistence-pglite'
import { startLocalFirstGraph } from '@prometheus-ags/prometheus-entity-management'
import { TauriCommands } from './infrastructure/tauri/tauri-commands'
import { TauriSecretStore } from './infrastructure/secrets/api-key-store'
import { registerAllTransports } from './infrastructure/pem/transports'
import { TauriEventBus } from './infrastructure/agui/event-bus'
import { TauriSupervisorClient } from './infrastructure/supervisor/client'
import { makeRestartService } from './application/use-cases/services/restart-service'
import { makePairDevice } from './application/use-cases/devices/pair-device'
// ... etc

export interface AppContext {
  commands: TauriCommands
  events: TauriEventBus
  secretStore: TauriSecretStore
  supervisor: TauriSupervisorClient
  useCases: {
    services: { restart: ReturnType<typeof makeRestartService>; ... }
    devices: { pair: ReturnType<typeof makePairDevice>; ... }
    // ... etc
  }
  pem: { graph: Awaited<ReturnType<typeof startLocalFirstGraph>> }
  shutdown(): Promise<void>
}

export function bootstrap(): AppContext {
  // 1. Build infrastructure (Tauri client, PGlite adapter, stronghold, iroh)
  const commands = new TauriCommands()
  const events = new TauriEventBus()
  const secretStore = new TauriSecretStore()
  const supervisor = new TauriSupervisorClient(commands)
  const persistence = createPGlitePersistenceAdapter(/* ... */)

  // 2. Start the PEM graph (Infrastructure)
  const graph = await startLocalFirstGraph({ persistence, ... })

  // 3. Register the PEM transports
  registerAllTransports({ commands, events, graph })

  // 4. Build the use cases (Application) — each takes its port as a dep
  const useCases = {
    services: { restart: makeRestartService({ supervisor }) },
    devices:  { pair:    makePairDevice({ p2p, supervisor }) },
    // ... etc
  }

  // 5. Shutdown handler
  async function shutdown() {
    await graph.stop()
    await events.close()
  }

  return { commands, events, secretStore, supervisor, useCases, pem: { graph }, shutdown }
}
```

> **Why a `bootstrap()` function and not top-level `await`?**
> Because PGlite is async, `startLocalFirstGraph` is async, and we
> want the composition root to be testable (you can `bootstrap()`
> in a test with mock adapters). Top-level `await` would force the
> whole app to be ESM-with-top-level-await and would be harder to
> test.

### 7.3 The chrome (`app.tsx`)

```tsx
// src/app.tsx
import { isTauri } from '@tauri-apps/api/core'
import { Titlebar } from '@/presentation/components/titlebar/titlebar'
import { NavShell } from '@/presentation/nav/nav-shell'
import { Outlet, RouterProvider, createRouter, createRoute, createRootRoute } from '@tanstack/react-router'
import { ThemeProvider } from '@/presentation/theme/theme-provider'
import { ToastProvider } from '@/presentation/components/ui/toast'
import type { AppContext } from './composition-root'

export function App({ ctx }: { ctx: AppContext }) {
  return (
    <ThemeProvider>
      <ToastProvider>
        <AppContextProvider value={ctx}>
          <RouterProvider router={router} />
        </AppContextProvider>
      </ToastProvider>
    </ThemeProvider>
  )
}

const rootRoute = createRootRoute({ component: () => (
  <>
    {isTauri() ? <Titlebar /> : null}
    <NavShell>
      <main className="app__main"><Outlet /></main>
    </NavShell>
  </>
)})
// ... routes wired from src/presentation/routes/
```

The `AppContextProvider` is a thin React context that exposes
`ctx` to application hooks. The hooks do not import the context
directly; they call a factory that takes the context as a
parameter (which the provider does on render).

### 7.4 The Tauri command + typed wrapper pattern (infrastructure)

Every Tauri command has a TypeScript wrapper in
`src/infrastructure/tauri/tauri-commands.ts`. **Application
use-cases** call the wrapper; **presentation components** never
call `invoke()` directly.

```ts
// src/infrastructure/tauri/tauri-commands.ts
import { invoke } from '@tauri-apps/api/core'
import type { ServiceHealth, AggregateHealth } from '@/domain/entities/service'

export class TauriCommands {
  async getAggregateHealth(): Promise<AggregateHealth> {
    return invoke<AggregateHealth>('get_aggregate_health')
  }
  async restartService(id: string): Promise<void> {
    return invoke('restart_service', { id })
  }
  async openServiceLog(id: string): Promise<void> {
    return invoke('open_service_log', { id })
  }
  // ... one method per command, typed against domain entities
}
```

> **Why a class?** The PEM / composition-root pattern wants a
> single object that holds all the infrastructure calls, and that
> object is what we inject into use cases. A class with typed
> methods gives us a single point for testing (mock the whole
> class) and a single point for future surface changes (e.g. move
> to a `tauri-plugin-` IPC).

### 7.5 The use case (application)

A use case is a function that takes its dependencies (via the
**port** interface, not the concrete adapter) and returns the
result:

```ts
// src/domain/ports/supervisor-port.ts — the port
export interface SupervisorPort {
  getAggregateHealth(): Promise<AggregateHealth>
  restartService(id: string): Promise<void>
  openServiceLog(id: string): Promise<void>
  listServices(): Promise<ServiceHealth[]>
}
```

```ts
// src/application/use-cases/services/restart-service.ts
import type { SupervisorPort } from '@/domain/ports/supervisor-port'
import { DomainError } from '@/domain/errors/domain-error'

export function makeRestartService(deps: { supervisor: SupervisorPort }) {
  return async function restartService(id: string): Promise<void> {
    if (!id) throw new DomainError('restart-service: missing id')
    return deps.supervisor.restartService(id)
  }
}
```

> **Why `make`-prefixed factories?** Use cases need their
> dependencies to be wired once, in the composition root. A
> factory function returns a closure that captures the deps. This
> keeps each use case testable in isolation (pass mock ports) and
> keeps the call site simple (`const restart = makeRestartService({ supervisor })`).

### 7.6 The application hook (consumed by components)

```ts
// src/application/hooks/use-services.ts
import { useEntity, useEntityMutation } from '@prometheus-ags/prometheus-entity-management/react'
import { serviceEntity } from '@/domain/entities/service'
import { useAppContext } from '@/presentation/app-context'

export function useServiceList() {
  const { pem } = useAppContext()
  return useEntities(serviceEntity, { orderBy: { label: 'asc' } })
}

export function useService(id: string) {
  return useEntity(serviceEntity, id)
}

export function useRestartService() {
  const { useCases } = useAppContext()
  return useEntityMutation(serviceEntity, 'restartService', {
    optimistic: true,
    execute: (id) => useCases.services.restart(id),
  })
}
```

### 7.7 The component (presentation)

```tsx
// src/presentation/components/dashboard/service-card.tsx
import { useService, useRestartService } from '@/application/hooks/use-services'
import { Card, CardHeader, CardTitle, CardContent, CardFooter, Button } from '@/presentation/components/ui'
import { ServiceStatusBadge } from './service-status-badge'
import { Metric } from './metric'
import { ServiceCardSkeleton } from './service-card-skeleton'
import { ServiceCardError } from './service-card-error'
import { humantime } from '@/lib/format'

export function ServiceCard({ id }: { id: string }) {
  const { data: service, isPending, error } = useService(id)
  const restart = useRestartService()
  if (isPending) return <ServiceCardSkeleton />
  if (error)     return <ServiceCardError error={error} />
  return (
    <Card>
      <CardHeader>
        <CardTitle>{service.label}</CardTitle>
        <ServiceStatusBadge status={service.status} />
      </CardHeader>
      <CardContent>
        <Metric label="Liveness" value={service.liveness} />
        <Metric label="Readiness" value={service.readiness} />
        <Metric label="PID" value={service.pid} />
        <Metric label="Uptime" value={humantime(service.uptimeSec)} />
      </CardContent>
      <CardFooter>
        <Button variant="ghost" onClick={() => openLog(id)}>View log</Button>
        <Button variant="destructive" onClick={() => restart.mutate(id)}>Restart</Button>
      </CardFooter>
    </Card>
  )
}
```

The component imports from **`application/hooks`** and
**`presentation/components/ui`**. It does not import from
`infrastructure/` directly. `audit-layer.sh` enforces this.

### 7.8 The `audit-layer.sh` script (the enforcement)

`scripts/audit-layer.sh` runs on every CI build and locally in
`pnpm audit:layer`. The full check:

```bash
#!/usr/bin/env bash
# scripts/audit-layer.sh — enforce the 4-layer CLEAN model + kebab-case
set -euo pipefail
cd "$(dirname "$0")/.."

# 1. Domain may not import from any other layer
violations=$(grep -rE "from ['\"]@?/" src/domain/ | \
  grep -vE "from ['\"]@?/domain/" | \
  grep -vE "from ['\"](\.\./)+(domain|entities|value-objects|ports|errors)" || true)
[ -z "$violations" ] || { echo "Domain imports violation:"; echo "$violations"; exit 1; }

# 2. Application may not import from Infrastructure or Presentation
violations=$(grep -rE "from ['\"]@?/infrastructure/" src/application/ | true)
[ -z "$violations" ] || { echo "Application imports Infrastructure:"; echo "$violations"; exit 1; }
violations=$(grep -rE "from ['\"]@?/presentation/" src/application/ | true)
[ -z "$violations" ] || { echo "Application imports Presentation:"; echo "$violations"; exit 1; }

# 3. Infrastructure may not import from Presentation
violations=$(grep -rE "from ['\"]@?/presentation/" src/infrastructure/ | true)
[ -z "$violations" ] || { echo "Infrastructure imports Presentation:"; echo "$violations"; exit 1; }

# 4. Presentation may not import from Infrastructure (direct)
#    (only via the AppContext provided by composition root)
violations=$(grep -rE "from ['\"]@?/infrastructure/" src/presentation/ | true)
[ -z "$violations" ] || { echo "Presentation imports Infrastructure (direct):"; echo "$violations"; exit 1; }

# 5. No `invoke(` or `listen(` in presentation/components/**
violations=$(grep -rE "\\binvoke\\(|\\blisten\\(" src/presentation/components/ | true)
[ -z "$violations" ] || { echo "Presentation uses invoke/listen directly:"; echo "$violations"; exit 1; }

# 6. No @tanstack/react-query anywhere
violations=$(grep -rE "@tanstack/react-query" src/ | true)
[ -z "$violations" ] || { echo "react-query found in src/:"; echo "$violations"; exit 1; }

# 7. No direct radix-ui imports
violations=$(grep -rE "@radix-ui/" src/ | true)
[ -z "$violations" ] || { echo "Radix found in src/:"; echo "$violations"; exit 1; }

echo "audit-layer: PASS"
```

### 7.9 The `audit-naming.sh` script (kebab-case enforcement)

```bash
#!/usr/bin/env bash
# scripts/audit-naming.sh — enforce kebab-case file names under src/
set -euo pipefail
cd "$(dirname "$0")/.."

# 1. Every .ts/.tsx file under src/ must be kebab-case or a known exception
violations=$(find src -type f \( -name "*.ts" -o -name "*.tsx" \) | \
  grep -vE "/(node_modules|dist)/" | \
  while read f; do
    base=$(basename "$f")
    case "$base" in
      __root.tsx)         ;;   # TanStack Router convention
      __root.ts)          ;;
      index.ts|index.tsx) ;;   # platform convention
      *.ts|*.tsx)         echo "$f" ;;
    esac
  done | \
  grep -vE "/[a-z0-9]+(-[a-z0-9]+)*\.(ts|tsx)$" | true)

# 2. Every folder under src/ must be kebab-case (with allowed exceptions)
folder_violations=$(find src -mindepth 1 -type d | \
  grep -vE "/[a-z0-9]+(-[a-z0-9]+)*$" | true)

if [ -n "$violations$folder_violations" ]; then
  echo "kebab-case violations:"
  echo "$violations"
  echo "$folder_violations"
  exit 1
fi

echo "audit-naming: PASS"
```

Both scripts are wired into the `pnpm audit` umbrella and the
local CI.

### 7.10 Routing (TanStack Router)

File-based routes in `src/presentation/routes/`. The root route
renders the Titlebar (Tauri only) and the NavShell. Each route
file exports a default component that uses **application hooks**
for data — no `useEffect` data fetching, ever.

### 7.11 State management (the "no query cache" rule)

Per `pem-local-first/SKILL.md`:

- **Durable data** (services, devices, runs, skills, providers,
  receipts) → PEM 3.x entity stores, in the **infrastructure** layer
- **Application state** (computed views, mappers, error-to-toast
  glue) → Application hooks
- **Transient interaction state** (panel open/closed, command
  palette text, toast queue, layout density) → Zustand stores in
  **`application/stores/`** (Application layer — they are not
  durable)
- **No TanStack Query**, ever. `audit-layer.sh` fails a project
  that depends on `@tanstack/react-query`.

### Skills used
- `clean-architecture/SKILL.md` (PMP, the canonical 4-layer model
  + React mapping)
- `tauri-react-vite/SKILL.md` (typed IPC)
- `react/react-vite-stack/SKILL.md` (React 19 + Vite 8 + TanStack
  Router)
- `entity-graph-web-shell/SKILL.md` (layer contract)
- `pem-local-first/SKILL.md` (no query cache)

---

## 8 · Design tokens (one source for every surface)

The HMA `hybrid-design-tokens` skill is the canonical rule:
**one source of truth (TOML) → generated outputs for every surface.**

### 8.1 The source

```toml
# assets/templates/design-tokens/tokens.toml
# PROMETHEUS COMPANION — single source of design truth.
# Edit ONLY this file. Re-run `bash scripts/gen-design-tokens.sh .`
# Both light and dark themes must satisfy WCAG 2.2 AA.

[meta]
generator = "gen-design-tokens.sh"
version = "1"

[color.canvas]
light = "#FAFAF7"   # near-white, warm
dark  = "#0B0F14"   # near-black, cool

[color.chrome]      # title bar, tray popover
light = "#FFFFFF"
dark  = "#11151B"

[color.surface]     # cards, panels
light = "#FFFFFF"
dark  = "#161B22"

[color.raised]      # elevated surfaces (popover, dialog)
light = "#FFFFFF"
dark  = "#1C232C"

[color.hover]
light = "#F2F1ED"
dark  = "#232A35"

[color.text]
light = "#0B0F14"
dark  = "#E6EDF3"

[color.muted]
light = "#5B6470"
dark  = "#8B949E"

[color.divider]
light = "#E8E5DE"
dark  = "#1F2630"

[color.accent]
light = "#5B6CFF"   # Prometheus indigo
dark  = "#7C8BFF"

[color.accent.muted]
light = "#E8EAFF"
dark  = "#1B1F3A"

[color.danger]
light = "#D14343"
dark  = "#F85149"

[color.warning]
light = "#D98E2B"
dark  = "#D29922"

[color.success]
light = "#2DA44E"
dark  = "#3FB950"

[color.focus]
light = "#5B6CFF"
dark  = "#7C8BFF"

[spacing.scale]
"0" = "0px"
"1" = "4px"
"2" = "8px"
"3" = "12px"
"4" = "16px"
"5" = "20px"
"6" = "24px"
"8" = "32px"
"10" = "40px"
"12" = "48px"
section = "clamp(48px, 6vw, 96px)"

[radius.scale]
none = "0"
xs   = "4px"
sm   = "6px"
md   = "10px"
lg   = "14px"
xl   = "20px"
full = "9999px"

[font.family]
sans  = "Inter, system-ui, -apple-system, 'Segoe UI', sans-serif"
mono  = "JetBrains Mono, ui-monospace, 'SF Mono', Menlo, monospace"

[font.size]
xs   = "12px"
sm   = "13px"
base = "14px"
md   = "15px"
lg   = "17px"
xl   = "20px"
"2xl" = "24px"
"3xl" = "30px"
hero = "clamp(28px, 4vw, 44px)"

[motion.duration]
fast   = "120ms"
medium = "200ms"
slow   = "320ms"

[motion.ease]
standard = "cubic-bezier(0.2, 0, 0, 1)"
decel    = "cubic-bezier(0, 0, 0, 1)"
accel    = "cubic-bezier(0.3, 0, 1, 1)"

[breakpoint]
# Mobile nav rule: 600px (not Tailwind's default 640)
sm = "600px"
md = "768px"
lg = "1024px"
xl = "1440px"
```

### 8.2 The generated outputs

`scripts/gen-design-tokens.sh .` reads the TOML and writes:

- `src/theme.css` — Tailwind 4 `@theme` block + light override
  (`:root[data-theme="light"]`) + dark override (`:root[data-theme="dark"]`)
  + shadcn CSS variables (`--background`, `--foreground`,
  `--primary`, etc.)
- `crates/prometheus-substrate/src/theme.rs` — a typed
  `AppTheme` struct that exposes the same tokens to Rust (for
  tray icon color generation)

The CSS file is generated; the script enforces the
"GENERATED — DO NOT EDIT" header. The HMA design-tokens skill
mandates this.

### 8.3 Why this is generated, not "kept in sync"

The skill is explicit: hand-mirroring causes drift, and two
palettes that agree on intent and disagree on values is worse than
one ugly palette. Generation makes drift impossible.

### 8.4 Token consumption rules

The HMA skill mandates:

1. No magic values in components. Web references `var(--…)` /
   Tailwind token classes. Rust references the typed `AppTheme`
   struct.
2. Both themes must feel intentional. The default theme is
   **system** (CSS `prefers-color-scheme`); the user can override.
3. Semantic naming wins. Tokens are named by role (`accent`,
   `danger`), not by hue (`blue`, `red`).
4. Cross-surface parity. The same semantic token must resolve to
   the same intent on web, tray, and Rust. Change a token →
   regenerate in the same commit.

### Skills used
- `hybrid-design-tokens/SKILL.md` (the entire pattern)

---

## 9 · shadcn/ui with Base UI (the new default)

The user explicitly asked for "shadcn-ui using the base-ui version."
That is now the shadcn default. We use it.

### 9.1 What changed in shadcn (July 2026)

The shadcn registry now ships components built on **Base UI**
(`@base-ui-components/react`) by default, not Radix. The package
boundary is `base-ui` vs `radix-ui`:

- **Radix UI** — `radix-ui` (v1.6.1), WorkOS, mature, frozen-ish
- **Base UI** — `@base-ui-components/react` (v1.6.0), MUI team,
  active monthly releases

shadcn abstracts the primitive layer; the same `npx shadcn@latest add
button` produces a Base UI anatomy in a new project, or a Radix
anatomy if the project opted into Radix. Source: [ui.beste.co "Base
UI Is the New shadcn/ui Default"](https://ui.beste.co/blog/base-ui-the-new-default)
and [shadcndeck.com "Radix vs Base UI"](https://www.shadcndeck.com/blog/radix-vs-base-ui).

### 9.2 Initialize with Base UI

```bash
# first time only
npx shadcn@latest init --base-ui --yes

# components.json afterwards:
{
  "$schema": "https://ui.shadcn.com/schema.json",
  "style": "new-york",
  "rsc": false,
  "tsx": true,
  "tailwind": {
    "config": "",
    "css": "src/styles/tailwind.css",
    "baseColor": "neutral",
    "cssVariables": true,
    "prefix": ""
  },
  "aliases": {
    "components": "@/components",
    "utils": "@/lib/utils",
    "ui": "@/components/ui",
    "lib": "@/lib",
    "hooks": "@/hooks"
  },
  "iconLibrary": "lucide",
  "base": "base-ui"
}
```

The `base: "base-ui"` key is the source of truth for which
primitive the shadcn CLI emits.

### 9.3 What we add (and why)

| Component | Why we need it |
|---|---|
| `button` | Title bar controls, dashboard actions |
| `dialog` | "Pair new device" modal, "Restart service" confirm |
| `popover` | Tray popover, command palette |
| `dropdown-menu` | Per-service context menu |
| `tooltip` | Tray icon hover, status chip explanations |
| `tabs` | Service detail (Overview / Logs / Metrics / Config) |
| `card` | Service cards, device cards |
| `input` / `textarea` | Provider configuration forms |
| `select` | Provider picker, model picker |
| `switch` | Auto-update toggle, sync toggle |
| `badge` | Service status, queue depth |
| `scroll-area` | Log tail, run history |
| `toast` | "Service restarted", "Device paired" |
| `command` | The ⌘K command palette (cmdk under the hood; Base UI's combobox is better than Radix's) |
| `separator` | Visual section dividers (only when the flat-2.0 rule allows it) |
| `field` | Form fields (Base UI's `Field` is a better API than Radix's `Form`) |
| `combobox` | Skill browser, model picker, provider search (Base UI has Combobox; Radix does not) |

### 9.4 The asChild → render migration

A library is not what we touch (we use the shadcn-generated
anatomy, which already has the right callsite), but the pattern is:

| Radix | Base UI |
|---|---|
| `<Dialog.Trigger asChild><button>…</button></Dialog.Trigger>` | `<Dialog.Trigger render={<button>…</button>}>…</Dialog.Trigger>` |
| `Overlay` | `Backdrop` |
| `onOpenChange={(open) => …}` | `onOpenChange={(open, eventDetails) => { const reason = eventDetails.reason; … }}` |
| `forceMount` | `keepMounted` (with slightly different animation timing) |

The shadcn registry generates the right one for us, so the only
place we hand-write Base UI calls is in custom components
(Titlebar, ServiceCard, A2UIRenderer) and those follow the same
patterns.

### 9.5 Coexistence with Radix (the strangler fig)

For the first few weeks, the project may have a few `radix-ui`
imports left over (e.g. in a third-party shadcn block we haven't
regenerated). That's fine: Base UI and Radix coexist in the same
project. We migrate one component at a time, leaf first (button
and label before the components that import them), one commit per
component.

### 9.6 The strict-flat-2.0 reminder

The HMA `tauri-ui-review` skill is strict: no visible borders, no
divider lines, no layout shadows. Adjacent regions differ by
**background color**. The shadcn theme CSS variables are tuned to
make this work without per-component overrides. When a shadcn
component ships with a default `border` (e.g. `Input`), we
override the `border` to `transparent` and use `--divider` only on
the bottom edge where the design requires it.

### Skills used
- `tauri-ui-review/SKILL.md` (the design-quality bar)
- `hybrid-design-tokens/SKILL.md` (the token consumer contract)

---

## 10 · Prometheus Entity Management integration

The Companion is a **prometheus-entity-skills** consumer. Every
durable datum in the app is a PEM entity, registered as an
infrastructure adapter and exposed to the application layer
through the **port** + **use case** pattern from CLEAN. The full
PEM bundle index is at
`prometheus-skill-pack/skills/react/prometheus-entity-skills/SKILLS.md`.

### 10.1 The entity inventory

| Entity type | Domain file | Sync lane |
|---|---|---|
| `service` | `src/domain/entities/service.ts` | relational (server-authoritative LWW) |
| `device` | `src/domain/entities/device.ts` | relational + CRDT (profile vault) |
| `run` (prometheus-exec receipt) | `src/domain/entities/run.ts` | append-only event log |
| `skill` | `src/domain/entities/skill.ts` | relational (server-authoritative, ETag refresh) |
| `provider` | `src/domain/entities/provider.ts` | relational |
| `model` | `src/domain/entities/model.ts` | relational |
| `plugin` | `src/domain/entities/plugin.ts` | relational |
| `setting` | `src/domain/entities/setting.ts` | local-only (never server-synced) |
| `queue_item` | `src/domain/entities/queue-item.ts` | append-only event log |
| `toast` | (transient) | (Zustand only — not a PEM entity) |

The `pem-local-first` skill is the rule: relational lane for shared
data, CRDT for user-owned collaborative state (the profile vault),
event log for append-only history (runs, telemetry, sessions).

### 10.2 Domain — the entity schema (one example — `service`)

```ts
// src/domain/entities/service.ts
import { defineEntity } from '@prometheus-ags/prometheus-entity-management'

export type ServiceStatus = 'green' | 'yellow' | 'red' | 'gray'
export type ServiceLiveness = 'up' | 'down' | 'unknown'
export type ServiceReadiness = 'ready' | 'starting' | 'degraded' | 'unreachable'

export interface ServiceHealth {
  id: string
  label: string
  liveness: ServiceLiveness
  readiness: ServiceReadiness
  lastExit: number | null
  pid: number | null
  port: number | null
  logTail: string[]                  // last 20 lines
  uptimeSec: number
}

export interface AggregateHealth {
  status: ServiceStatus
  services: ServiceHealth[]
  queueDepth: number
  lastDoctorRun: string | null
  deviceId: string
  deviceName: string
}

export const serviceEntity = defineEntity({
  name: 'service',
  privacyClass: 'public',           // see sync-doctrine
  fields: {
    id:           { type: 'id',     clientId: true },
    tenantId:     { type: 'string', nullable: false, indexed: true },
    updatedAt:    { type: 'datetime', nullable: false, indexed: true },
    label:        { type: 'string', nullable: false },
    status:       { type: 'enum', enum: ['green', 'yellow', 'red', 'gray'] },
    liveness:     { type: 'enum', enum: ['up', 'down', 'unknown'] },
    readiness:    { type: 'enum', enum: ['ready', 'starting', 'degraded', 'unreachable'] },
    lastExit:     { type: 'number', nullable: true },
    pid:          { type: 'number', nullable: true },
    port:         { type: 'number', nullable: true },
    logTail:      { type: 'string[]' },
    uptimeSec:    { type: 'number' },
    configHash:   { type: 'string' },   // detect drift from installed binary
    binarySha:    { type: 'string' },
  },
  relations: {
    runs:         { type: 'hasMany',  target: 'run',   foreignKey: 'serviceId' },
  },
})
```

Domain files have **no Tauri, no React, no PEM runtime imports
above the `defineEntity` type-only call** — the
`@prometheus-ags/prometheus-entity-management` package's
`defineEntity` is a pure type. This is what `audit-layer.sh`
verifies.

### 10.3 Domain — the port

```ts
// src/domain/ports/supervisor-port.ts
import type { AggregateHealth, ServiceHealth } from '../entities/service'

export interface SupervisorPort {
  getAggregateHealth(): Promise<AggregateHealth>
  listServices(): Promise<ServiceHealth[]>
  restartService(id: string): Promise<void>
  openServiceLog(id: string): Promise<void>
}
```

### 10.4 Infrastructure — the PEM transport

```ts
// src/infrastructure/pem/transports/service-transport.ts
import { registerEntityTransport } from '@prometheus-ags/prometheus-entity-management'
import type { TauriCommands } from '@/infrastructure/tauri/tauri-commands'
import { serviceEntity } from '@/domain/entities/service'

export function registerServiceTransport(deps: { commands: TauriCommands }) {
  registerEntityTransport(serviceEntity, {
    list:   async (query) => (await deps.commands.getAggregateHealth()).services,
    get:    async (id)   => { /* ... */ },
    create: async (data) => { /* ... */ },
    update: async (id, patch) => { /* ... */ },
    delete: async (id)   => { /* ... */ },
  })
}
```

The transport reads the local store (PGlite) and, when the local
store is empty, falls back to the Tauri command. **Transports
never open sockets and never call remote APIs directly** (ADR-LFS-2
in `pem-local-first/SKILL.md`).

### 10.5 Infrastructure — the supervisor adapter (port impl)

```ts
// src/infrastructure/supervisor/client.ts
import type { SupervisorPort } from '@/domain/ports/supervisor-port'
import type { TauriCommands } from '@/infrastructure/tauri/tauri-commands'

export class TauriSupervisorClient implements SupervisorPort {
  constructor(private readonly commands: TauriCommands) {}
  async getAggregateHealth() { return this.commands.getAggregateHealth() }
  async listServices()        { return (await this.getAggregateHealth()).services }
  async restartService(id: string) { return this.commands.restartService(id) }
  async openServiceLog(id: string)  { return this.commands.openServiceLog(id) }
}
```

### 10.6 Application — the use case

```ts
// src/application/use-cases/services/restart-service.ts
import type { SupervisorPort } from '@/domain/ports/supervisor-port'
import { DomainError } from '@/domain/errors/domain-error'

export function makeRestartService(deps: { supervisor: SupervisorPort }) {
  return async function restartService(id: string): Promise<void> {
    if (!id) throw new DomainError('restart-service: missing id')
    return deps.supervisor.restartService(id)
  }
}
```

### 10.7 Application — the hook (consumed by components)

```ts
// src/application/hooks/use-services.ts
import { useEntity, useEntityMutation, useEntities } from '@prometheus-ags/prometheus-entity-management/react'
import { serviceEntity } from '@/domain/entities/service'
import { useAppContext } from '@/presentation/app-context'

export function useServiceList() {
  return useEntities(serviceEntity, { orderBy: { label: 'asc' } })
}

export function useService(id: string) {
  return useEntity(serviceEntity, id)
}

export function useRestartService() {
  const { useCases } = useAppContext()
  return useEntityMutation(serviceEntity, 'restartService', {
    optimistic: true,
    execute: (id: string) => useCases.services.restart(id),
  })
}
```

### 10.8 Presentation — the component (consumes only the hook)

```tsx
// src/presentation/components/dashboard/service-card.tsx
import { useService, useRestartService } from '@/application/hooks/use-services'
import { Card, CardHeader, CardTitle, CardContent, CardFooter, Button } from '@/presentation/components/ui'
import { ServiceStatusBadge } from './service-status-badge'
import { Metric } from './metric'
import { ServiceCardSkeleton } from './service-card-skeleton'
import { ServiceCardError } from './service-card-error'
import { humantime } from '@/lib/format'

export function ServiceCard({ id }: { id: string }) {
  const { data: service, isPending, error } = useService(id)
  const restart = useRestartService()
  if (isPending) return <ServiceCardSkeleton />
  if (error)     return <ServiceCardError error={error} />
  return (
    <Card>
      <CardHeader>
        <CardTitle>{service.label}</CardTitle>
        <ServiceStatusBadge status={service.status} />
      </CardHeader>
      <CardContent>
        <Metric label="Liveness" value={service.liveness} />
        <Metric label="Readiness" value={service.readiness} />
        <Metric label="PID" value={service.pid} />
        <Metric label="Uptime" value={humantime(service.uptimeSec)} />
      </CardContent>
      <CardFooter>
        <Button variant="ghost" onClick={() => openLog(id)}>View log</Button>
        <Button variant="destructive" onClick={() => restart.mutate(id)}>Restart</Button>
      </CardFooter>
    </Card>
  )
}
```

The component imports from **`application/hooks`** and
**`presentation/components/ui`**. It does not import from
`infrastructure/` directly. `audit-layer.sh` enforces this.

### 10.9 Sync-fed reactivity (the killer feature)

When a paired device restarts `prometheus-knowledge`, the
sovereign-sync gossip brings the new state to this node. The PEM
transport re-validates from the local store, the hook re-emits, the
component re-renders. The user sees the green dot return without
ever refreshing.

This is the architectural payoff for PEM over a query cache. A query
cache would re-fetch on window focus or interval. PEM updates
**because the data changed**, not because some interval said so.

### Skills used
- `pem-local-first/SKILL.md` (the schema/transport/hook pattern)
- `entity-graph-web-shell/SKILL.md` (the invariants)
- `sync-doctrine/SKILL.md` (lane choice per entity)
- `prometheus-entity-skills/SKILLS.md` (the plugin map)
- `clean-architecture/SKILL.md` (the layer mapping)

---

## 11 · Tray icon — the always-visible dashboard surface

The tray icon is the **only** surface the user sees by default. Its
color is the substrate's heartbeat.

### 11.1 The icon states

| State | Color | When |
|---|---|---|
| Green | `var(--success)` | All required services `liveness=up, readiness=ready`, queue depth < threshold |
| Yellow | `var(--warning)` | At least one service is `liveness=up` but `readiness!=ready` (degraded) |
| Red | `var(--danger)` | At least one required service is `liveness=down` for ≥ 30s |
| Gray | `var(--muted)` | Substrate paused by the user |
| Spinner | animated | Substrate initializing (first launch, or after a global restart) |

### 11.2 The icon set

Five PNGs at 16/22/32/44/88/176 px (the sizes Tauri tray requires
on macOS / Windows / Linux). Generated by `scripts/gen-tray-icons.sh`
from a single SVG source-of-truth, themed for light and dark OS
modes. The Rust side selects the right one for the current state.

```rust
// crates/prometheus-companion/src/tray.rs
use tauri::tray::{TrayIcon, TrayIconBuilder, TrayIconEvent, MouseButton, MouseButtonState}
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem}
use tauri::Manager

pub fn build_tray(app: &tauri::App) -> tauri::Result<TrayIcon> {
    let show = MenuItem::with_id(app, "show", "Show dashboard", true, None::<&str>)?;
    let status = MenuItem::with_id(app, "status", "Substrate status", true, None::<&str>)?;
    let pause = MenuItem::with_id(app, "pause", "Pause substrate", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let pair = MenuItem::with_id(app, "pair", "Pair new device…", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Companion", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[
        &show, &status, &pause, &settings, &sep, &pair, &sep, &quit,
    ])?;

    TrayIconBuilder::with_id("prometheus-tray")
        .icon(app.default_window_icon().unwrap().clone())
        .icon_as_template(true)              // macOS template image
        .menu(&menu)
        .on_menu_event(|app, ev| match ev.id.as_ref() {
            "show"     => show_main(app),
            "status"   => show_tray_popover(app),
            "pause"    => toggle_pause(app),
            "settings" => show_settings(app),
            "pair"     => show_pairing(app),
            "quit"     => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, ev| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = ev {
                show_tray_popover(tray.app_handle());
            }
        })
        .build(app)
}
```

### 11.3 The tray popover window

`tray-popover` window in `tauri.conf.json` is frameless,
transparent, `skipTaskbar: true`, `alwaysOnTop: true`. It opens
under the tray icon on left-click and shows a compact status
surface:

- Substrate health (green/yellow/red/gray) with one-line summary
- Queue depth (X pending, Y processing, Z dead-lettered)
- Last doctor run (timestamp + link to full report)
- Quick actions: "Show dashboard", "Pause", "Pair device"

The popover closes on `Esc` and on click-outside (a Tauri window
event listener).

### 11.4 The health aggregator (Rust)

The aggregator subscribes to each service's health update and
applies the policy in §11.1. It emits a `tray:state` event to the
frontend whenever the state changes.

```rust
// crates/prometheus-companion/src/health.rs
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TrayState { Green, Yellow, Red, Gray, Initializing }

pub struct HealthAggregator {
    state: Arc<RwLock<TrayState>>,
    supervisor: Arc<Supervisor>,
}

impl HealthAggregator {
    pub async fn run(self: Arc<Self>) {
        let mut rx = self.supervisor.subscribe_health();
        while let Some(update) = rx.recv().await {
            let new = self.compute(&update);
            if new != *self.state.read().await {
                *self.state.write().await = new.clone();
                self.app_handle.emit("tray:state", &new).ok();
                self.set_icon(&new).await;
            }
        }
    }
    fn compute(&self, update: &HealthUpdate) -> TrayState {
        // the policy in §11.1
    }
    async fn set_icon(&self, state: &TrayState) {
        let icon = self.icons.get(state);
        self.app_handle.tray_handle().set_icon(icon).ok();
    }
}
```

### Skills used
- (no direct skill, but the pattern comes from the architecture
  review §2.3 and the Tauri 2 docs)

---

## 12 · Main dashboard — the per-service operations view

The main window is what the user sees when they click the tray and
choose "Show dashboard" or click the icon's right side. The
top-level destinations follow the **mobile-navigation** skill rule
(bottom bar by width, never by platform), even on desktop:

### 12.1 The destinations (one list, never two)

| Path | Label | Icon | Width rail? |
|---|---|---|---|
| `/` | Overview | `LayoutDashboard` | rail at ≥ 600px |
| `/services` | Services | `Server` | rail |
| `/devices` | Devices | `Network` | rail |
| `/chat` | Chat | `MessagesSquare` | rail |
| `/skills` | Skills | `Sparkles` | rail |
| `/runs` | Runs | `Activity` | rail |
| `/settings` | Settings | `Settings` | rail |

The rail is a `NavigationRail` component (Base UI primitives +
`useBreakpoint`); the bottom bar is a `BottomBar` component. The
list of destinations is **one** array; the chrome is chosen by
`useBreakpoint` (≥ 600px → rail, < 600px → bar). The `mobile-
navigation` skill is explicit: **one list, never two.**

### 12.2 The Overview page

- A health banner (one line: "All systems green" or "1 of 9 services
  degraded")
- A "Today" panel (KBD phase, learning queue, last run receipt, last
  doctor run, last update)
- A grid of service cards (compact, 3 columns at 1024+ / 2 columns
  at 768 / 1 column at 320)
- A "Recent activity" panel (last 10 runs, last 5 paired devices,
  last 3 sync errors)

### 12.3 The Services page

- A search/filter bar (Base UI Combobox, async against the skill
  index)
- A table of all services (PEM `useEntities('service')`) with
  columns: Name, Status, Liveness, Readiness, PID, Uptime, Actions
- Per-row context menu: View log, Restart, Open in browser, View
  config, Disable
- A "Add service" button (for plugin-supplied services)

### 12.4 The Devices page

- This device (own identity key, public key, node id, current IP)
- Paired devices (one card each: name, type [mobile/desktop/cli/
  headless], last seen, capabilities, authorized skills)
- "Pair new device" button → QR + 6-digit short code flow

### 12.5 The Chat page

- An Assistant-UI Thread (the same library the native agent
  generator uses)
- A provider / model switcher in the header
- An A2UI surface panel docked to the right (toggleable)
- All AG-UI events streamed from the substrate's `agui` module
  flow into the ContentBlocks renderer

### 12.6 The Skills page

- A search bar (the semantic router from the architecture review
  §7.4)
- A list of skills with the use-frequency telemetry (the router's
  hit-rate log)
- A "Skill details" drawer with description, when-to-use, last
  invocation, hit-rate chart
- A "Disable" / "Reorder" / "Pin" set of actions

### 12.7 The Runs page

- A live tail of recent `prometheus-exec` receipts
- Filters: by service, by status, by time range, by skill
- A receipt detail drawer with the signed JSON, the inputs hash,
  the output hash, the artifacts, and a "Verify" button that calls
  `prometheus-exec verify` and shows the result

### 12.8 The Settings page

- Tabs: General, Providers, Plugins, About
- **General**: theme (light / dark / system), notification
  preferences, log retention, telemetry opt-in
- **Providers**: add / remove provider, configure API key (stored
  in OS secure store, never persisted to PEM), test connection
- **Plugins**: installed plugin list, "Update available" badge,
  "Install from marketplace" link, "Discover" tab
- **About**: version, build SHA, substrate versions, links to docs

### Skills used
- `mobile-navigation/SKILL.md` (one list, width-based switch)
- `tauri-ui-review/SKILL.md` (the breakpoint-and-theme review)
- `react/react-vite-stack/SKILL.md` (the routing primitives)

---

## 13 · A2UI surface — orchestrator-driven UI

A2UI is **a projection, not authority** (per
`a2ui-surface-contract/SKILL.md`). The Companion renders the
projection; actions go back through UAR.

### 13.1 The A2UI registry

```ts
// src/presentation/components/a2ui/a2ui-registry.ts
import { TextBlock } from './components/text-block'
import { MarkdownBlock } from './components/markdown-block'
import { FormBlock } from './components/form-block'
import { FieldBlock } from './components/field-block'
import { SwitchBlock } from './components/switch-block'
import { DialogBlock } from './components/dialog-block'
import { TableBlock } from './components/table-block'
import { CardBlock } from './components/card-block'
import { CodeBlock } from './components/code-block'
import { ImageBlock } from './components/image-block'
import { UnknownBlock } from './components/unknown-block'

export const a2uiRegistry = {
  Text:        TextBlock,
  Markdown:    MarkdownBlock,
  Form:        FormBlock,
  Field:       FieldBlock,
  Switch:      SwitchBlock,
  Dialog:      DialogBlock,
  Table:       TableBlock,
  Card:        CardBlock,
  Code:        CodeBlock,
  Image:       ImageBlock,
  Unknown:     UnknownBlock,        // unknown entity kinds remain inspectable
} as const
```

### 13.2 The renderer

```tsx
// src/presentation/components/a2ui/a2ui-renderer.tsx
import { a2uiRegistry } from './a2ui-registry'
import { ActionRouter } from './action-router'
import type { A2UIEnvelope } from '@/domain/value-objects/a2ui-envelope'

export function A2UIRenderer({ envelope }: { envelope: A2UIEnvelope }) {
  const Component = a2uiRegistry[envelope.componentType] ?? a2uiRegistry.Unknown
  return (
    <Component {...envelope.props}>
      {/* If the component produces user actions, route them through ActionRouter */}
      <ActionRouter actions={envelope.actions} envelopeId={envelope.id} />
    </Component>
  )
}
```

### 13.3 The contract (the seven rules)

Per `a2ui-surface-contract/SKILL.md`:

1. Parse a versioned discriminated event envelope.
2. Render only registered component types and validated props.
3. Preserve unknown events as inspectable artifacts.
4. Route actions back through UAR with run, event, and idempotency IDs.
5. Show pending approval, cancellation, denial, failure, and resumed
   states.
6. Never execute tool calls, provider routing, or policy decisions
   in the UI.
7. Sanitize rich content and external URLs.

### 13.4 The test surface

The A2UI surface is verified by:

- Deterministic rendering (same envelope → same DOM)
- Action continuation after restart (idempotency-keyed actions
  resume)
- Duplicate event handling (no double-render)
- Unknown event display (preserved as inspectable, never silently
  dropped)
- Cancellation, denied actions, malformed props (graceful)
- Equivalent semantics across web and (future) mobile

### Skills used
- `a2ui-surface-contract/SKILL.md` (the contract)
- `a2ui-surface/SKILL.md` (the cross-harness resolver)

---

## 14 · P2P pairing, device identity, sovereign-sync integration

The Companion is a sovereign-sync node. Pairing a new device is a
Tailscale-style flow over Iroh QUIC, authenticated by Ed25519 endpoint identity.

### 14.1 This device's identity

Each Companion has an Ed25519 keypair generated on first run, stored
in the OS secure store (Keychain on macOS, Credential Manager on
Windows, Secret Service on Linux). The public key is the node id in
the P2P overlay; the private key never leaves the device.

### 14.2 Pairing flow

**Corrected by `change-cpc-006`.** There is no Noise handshake. `grep -i noise`
over the sovereign-sync sources and manifest returns nothing: peers authenticate
by **Ed25519 EndpointId over iroh's QUIC/TLS 1.3**, and admission is a pairing
ticket plus an endpoint allowlist (`export_ticket` / `import_ticket` /
`authorize_endpoint`). The steps below describe what is implemented.

1. Operator clicks "Pair new device" in the tray menu
2. The Companion exports a pairing ticket — group secret, endpoint id, and
   signing-key fingerprint, base64url — and displays it as a
   `prometheus://pair?ticket=…` QR code
3. The new device scans the QR and imports the ticket
4. The importing side verifies the ticket's protocol version, group secret
   length, endpoint id, and signing-key fingerprint, then authorizes that
   endpoint; the transport itself is authenticated by iroh's QUIC/TLS 1.3
5. The new device sends its capability manifest (`mobile` /
   `desktop` / `cli` / `headless`) and authorized-skill list
6. The Companion shows a "Pair this device?" dialog with the
   manifest, requiring explicit approval
7. After approval, the new device appears in the Devices page

### 14.3 The Iroh transport

The Companion speaks Iroh directly (the same stack GuardianDB
uses, per the architecture review §4.2). Iroh provides:

- **NAT traversal** via magicsock (Tailscale-derived)
- **Roaming** across IP changes (WiFi → 5G)
- **QUIC transport** with TLS 1.3
- **BLAKE3 addressing** for blobs

Sovereign-sync is the high-level protocol (event types, identity
verification, device authorization). Iroh is the transport.

### 14.4 What paired devices can do

By default, a paired mobile device can:

- Read the substrate's health (the same data the local dashboard
  shows, but mobile-shaped)
- Stream AG-UI events from a running skill
- Send small requests (≤ 1 MB) to the substrate for read-only
  operations
- NOT write to sovereign-memory
- NOT execute skills on the node (the operator must explicitly
  authorize execution)

The operator can change the capability per device in the Devices
page. The authorization is enforced at the Axum boundary (per
`axum-agent-gateway/SKILL.md`).

### Skills used
- `peer-profile-sync/SKILL.md` (the profile vault pattern)
- `axum-agent-gateway/SKILL.md` (the authorization boundary)

---

## 15 · AG-UI / SSE event streaming

AG-UI is the SSE event protocol. The substrate emits; the
Companion's Chat page consumes. The contract is in
`agui-event-contract/SKILL.md`.

### 15.1 The event shapes

```ts
// src/lib/agui.ts
import { z } from 'zod'

export const AGUIEvent = z.discriminatedUnion('type', [
  z.object({ type: z.literal('run.started'),     runId: z.string(), threadId: z.string() }),
  z.object({ type: z.literal('text.delta'),      runId: z.string(), delta: z.string() }),
  z.object({ type: z.literal('thinking.delta'),  runId: z.string(), delta: z.string() }),
  z.object({ type: z.literal('tool.started'),    runId: z.string(), toolName: z.string(), args: z.unknown() }),
  z.object({ type: z.literal('tool.completed'),  runId: z.string(), toolName: z.string(), result: z.unknown() }),
  z.object({ type: z.literal('artifact.created'), runId: z.string(), artifact: Artifact }),
  z.object({ type: z.literal('citation'),         runId: z.string(), source: Citation }),
  z.object({ type: z.literal('memory.recalled'),  runId: z.string(), item: MemoryItem }),
  z.object({ type: z.literal('run.completed'),    runId: z.string(), status: z.enum(['ok', 'error', 'cancelled']), error: z.string().optional() }),
  z.object({ type: z.literal('run.approval'),     runId: z.string(), reason: z.string() }),
])
```

**Amended by `change-cpc-005`.** AG-UI has two consumers, not one:

- **The Companion's own UI** consumes it over **Tauri events**, as the code
  below shows. No HTTP, no serialization round trip.
- **External callers** — the pack's `prometheus` CLI, other machines, the
  Codex/Kimi harnesses — consume the **SSE** stream on the control socket.

The original text described only the first and would have been read as
forbidding the second. Both are required: dropping SSE would break every
existing external consumer, and routing the Companion's own UI through HTTP
would add a hop for no benefit.

### 15.2 The consumer

```ts
// src/application/hooks/use-agui-stream.ts
import { useEffect, useState } from 'react'
import { useAppContext } from '@/presentation/app-context'
import type { AGUIEvent } from '@/domain/value-objects/agui-event'

export function useAguiStream(runId: string): AGUIEvent[] {
  const { events } = useAppContext()                    // TauriEventBus (infrastructure)
  const [stream, setStream] = useState<AGUIEvent[]>([])

  useEffect(() => {
    const unlisten = events.subscribe<AGUIEvent>('agui:event', (e) => {
      if (e.runId !== runId) return
      setStream((prev) => [...prev, e])
    })
    return () => { unlisten() }
  }, [events, runId])

  return stream
}
```

The `listen` call lives in the **infrastructure** layer (the
`TauriEventBus` adapter), the hook is in the **application** layer,
and the component consumes the hook. The component never imports
`@tauri-apps/api/event` directly — `audit-layer.sh` enforces this.

### 15.3 Resumability

Every event carries the `runId` and the `threadId`. The Chat page
can resume from the last event id after a reload (per the
`axum-agent-gateway/SKILL.md` resumability contract). The PEM
`run` entity stores the last event id for each run.

### Skills used
- `agui-event-contract/SKILL.md` (the event shapes)
- `axum-agent-gateway/SKILL.md` (the resumability contract)
- `content-block-ui/SKILL.md` (the per-variant styling)

---

## 16 · Service supervisor — what the Companion knows about its substrate

The supervisor is the heart of the substrate. It is implemented
as an mpsc actor in `crates/prometheus-substrate/src/supervisor.rs`
(per `rust/actor-model/SKILL.md`).

### 16.1 The supervisor's responsibilities

- Start the substrate's children (in-process services + the
  optional `surreal` sidecar) in dependency order
- Watch each child's exit; restart on failure (with exponential
  backoff)
- Aggregate per-child health (liveness, readiness, last exit
  code, PID, port)
- Emit health updates to the health aggregator (§11.4) and to
  the receipt bus
- Persist supervisor state to `~/.prometheus/substrate/state.toml`
  for restart recovery
- Handle graceful shutdown on quit

### 16.2 The supervisor state

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupervisorState {
    pub schema_version: u32,
    pub children: HashMap<String, ChildState>,
    pub started_at: DateTime<Utc>,
    pub last_health: HashMap<String, ChildHealth>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChildState {
    pub id: String,
    pub kind: ChildKind,                   // InProcess | ExternalSidecar
    pub start_count: u32,
    pub last_started_at: DateTime<Utc>,
    pub last_exit_code: Option<i32>,
    pub last_exit_reason: Option<String>,
    pub consecutive_failures: u32,
    pub disabled: bool,                    // user disabled in the UI
}
```

### 16.3 The restart policy

- First failure: immediate restart
- Consecutive failures: exponential backoff (1s, 2s, 4s, 8s, 16s,
  32s, max 60s)
- After 10 consecutive failures: mark `disabled`, emit a red
  tray event, surface a "Service disabled after 10 failures"
  toast
- On user re-enable: reset `consecutive_failures`, restart

### 16.4 The dependency order

**Amended by `change-cpc-007`.** The nine-service in-process list below
predates operator decision D-02 (option D) and no longer describes the
architecture. Kept struck through rather than deleted, so a reader following
history sees what changed and why — see [`docs/decisions/`](decisions/) for
D-02 itself.

<details>
<summary>Superseded: the original in-process dependency order</summary>

```
surreal (sidecar, optional) → surreal-memory (in-process)
                            → prometheus-knowledge (in-process)
                            → prometheus-research (in-process, feature-gated)
                            → forge-rs (in-process)
                            → liter-llm (in-process)
                            → sovereign-sync (in-process, opens Iroh endpoint)
                            → surface-bridge (in-process)
                            → skill-index (in-process)
                            → prometheus-exec (in-process)
```

</details>

**What actually runs where, as of `change-cpc-007`:**

The pack installs and owns every one of those nine services, and they keep
running exactly where the pack put them (D-02: "adoption, not relocation" —
see `docs/integration-contract.md` Seam 3). None of them link into this
process. The Companion hosts exactly **one** in-process thing: its own
`sovereign-sync` node (§4.1), reached over the contract socket. Everything
else is **adopted**: `crates/prometheus-substrate::supervisor::Supervisor`
reads the pack's generated `shared/services.manifest.json`, probes each
entry — on its own endpoint when it has one (2 of 12 services today), through
`launchctl print`/`systemctl --user show` otherwise (the other 10) — and
restarts a service **through the platform supervisor**
(`launchctl kickstart` / `systemctl --user restart`) when found down. It never
starts, stops, or links a pack service directly.

So there is no dependency order to draw among these nine: they are not this
process's children, and this process does not sequence their startup. The
only real order is the Companion's own: bind the contract socket and answer
`/health` before opening its node's authority (§4.1's "not negotiable"
ordering), then attach supervision, then install the full router
(`crates/prometheus-substrate/src/node.rs`).

The Companion's UI is the "topmost" of the one thing it does host — it
subscribes to `Supervisor::subscribe()`'s folded `TrayState`
(`crates/prometheus-substrate/src/health.rs`) and repaints the tray icon
(`src-tauri/src/tray.rs`) on change, but does not depend on any specific
adopted service being healthy to render (the tray shows `Starting` gracefully
until the first probe pass completes).

### Skills used
- `rust/actor-model/SKILL.md` (the mpsc actor pattern)
- `rust/async-patterns/SKILL.md` (graceful shutdown, broadcast)
- `rust/error-handling/SKILL.md` (typed errors, no unwrap)

---

## 17 · Provider, model, and credential management

The Companion manages providers (Anthropic, OpenAI, OpenRouter,
Groq, Together, local) and models through Liter-LLM's registry.
Per `deploy-hybrid-agentic-stack/SKILL.md`:

- Derive providers and capabilities from Liter-LLM's registry; do
  not hard-code a stale provider list.
- Anonymous hosted keys are memory-only and session-bound. Durable
  hosted keys require authenticated identity and encrypted Flint
  Vault references.
- Local desktop/mobile keys use platform secure storage.
- Never store secret values in PEM, PGlite, Zustand, logs, URLs,
  ordinary database columns, images, Compose files, or ConfigMaps.
  APIs return metadata, never key values.

### 17.1 The key storage path

```ts
// src/features/providers/storage.ts
import { invoke } from '@tauri-apps/api/core'

export async function storeApiKey(provider: string, key: string): Promise<void> {
  // Tauri command on the Rust side writes to OS secure store.
  // The TS side NEVER persists the key value to PEM, PGlite, or
  // any other store.
  return invoke('store_api_key', { provider, key })
}

export async function hasApiKey(provider: string): Promise<boolean> {
  return invoke<boolean>('has_api_key', { provider })
}

export async function deleteApiKey(provider: string): Promise<void> {
  return invoke('delete_api_key', { provider })
}
```

The Rust side:

```rust
// crates/prometheus-companion/src/commands/secrets.rs
#[tauri::command]
pub async fn store_api_key(
    app: AppHandle,
    provider: String,
    key: String,
) -> Result<(), String> {
    use tauri_plugin_stronghold::Stronghold;
    let stronghold = Stronghold::default();
    let client = stronghold.get_client("prometheus-companion").map_err(|e| e.to_string())?;
    let vault = client.vault("provider-keys");
    vault.store(&provider, key.as_bytes()).map_err(|e| e.to_string())?;
    Ok(())
}
```

The `tauri-plugin-stronghold` is the Tauri-native secret store; the
key is encrypted at rest, only the running process can decrypt it,
and it is never sent back to the frontend.

### 17.2 The provider model surface

- The Settings → Providers page lists every provider in Liter-LLM's
  registry
- For each provider: a "Configure" button that opens a dialog
  asking for the API key
- After entry: a "Test connection" button that calls
  `liter-llm`'s `GET /v1/models` (or equivalent) with the key
- The key is never displayed again — the UI shows a "Configured
  ✓" badge instead

### Skills used
- `deploy-hybrid-agentic-stack/SKILL.md` (the BYOK rules)
- `tauri-react-vite/SKILL.md` (the command pattern)

---

## 18 · Storage and sync — local-first, server-authoritative

The Companion is a local-first app. The full doctrine is in
`sync-doctrine/SKILL.md`.

### 18.1 The lane map

| Data | Lane | Server-synced? |
|---|---|---|
| `service` | relational | yes (this node is server for other nodes) |
| `device` | relational + CRDT (profile vault) | yes (paired across nodes) |
| `run` | append-only event log | yes (per-device, batched) |
| `skill` | relational | yes (server-authoritative, ETag refresh) |
| `provider` | relational | yes (server-authoritative) |
| `model` | relational | yes (server-authoritative) |
| `plugin` | relational | yes (server-authoritative) |
| `setting` | local-only | **never** (privacy class: `local`) |
| `toast` | Zustand only | n/a (transient) |

### 18.2 The seven invariants (per `sync-doctrine/SKILL.md`)

1. Server is authority for shared relational data — client optimism
   is UI overlay.
2. Client-generated UUID PKs; every write idempotent/replayable.
3. Additive-only local migrations (`deprecated_at`, never destructive).
4. Secrets and `local`-class data never enqueue for server sync.
5. Boot order: migrations → seed/lookup → pre-onboarding load →
   [onboarding] → post-onboarding load → sync attach. Surface as
   idle→hydrating→syncing→ready.
6. One abstraction, many transports: features talk to
   `SyncTransport` / repositories — never to a specific gateway
   or socket.
7. Assume per-event authz re-check server-side; never widen
   subscriptions past the user's scopes; fail closed when authz
   is unreachable.

### 18.3 The boot order (on first launch)

1. Migrate PGlite schema (additive only)
2. Load lookup data (providers, models, skill descriptions)
3. Pre-onboarding load (anonymous-safe data: e.g. the device's
   own identity, the substrate's own service list)
4. Onboarding (only if the user has not yet paired this device:
   show the "Pair this device" flow)
5. Post-onboarding load (paired device data, sync subscriptions)
6. Attach the sync transport
7. Surface: idle → hydrating → syncing → ready

### 18.4 The "one queue, not two" rule

The write path is:

```
Component → Hook → PEM mutation → local store + _operation_queue
                                              ↓
                                        sync engine (separate process)
                                              ↓
                                        server (PEM server)
```

There is no second replay queue in JS. The queue in the local store
is the truth. On `FATAL_ERROR → DEAD_LETTER` the mutation rolls
back visibly (toast + SyncChip detail).

### Skills used
- `sync-doctrine/SKILL.md` (the entire doctrine)
- `pem-local-first/SKILL.md` (the wiring recipe)
- `peer-profile-sync/SKILL.md` (the CRDT profile vault)

---

## 18.5 · Connected Skill Packages Management (Pillar 7 — HMA × PMP × Companion joint work)

The Companion is not just a substrate supervisor. It is also
the **operator's view of the skill package ecosystem**. The
Companion can install, upgrade, validate, and remove
git-based skill packages (the HMA is the canonical
example) without leaving the tray app. This section is
the Companion-side of the joint design in
`/Users/gqadonis/Projects/hybrid-mobile-architecture-src/docs/05-hma-pmp-companion-architecture.md`.

### 18.5.1 The data model (PEM 3.x, application layer)

```ts
// src/domain/entities/skill-package.ts
import { defineEntity } from '@prometheus-ags/prometheus-entity-management'

export const skillPackageEntity = defineEntity({
  name: 'skillPackage',
  privacyClass: 'public',
  fields: {
    id:           { type: 'id', clientId: true },
    tenantId:     { type: 'string', nullable: false, indexed: true },
    updatedAt:    { type: 'datetime', nullable: false, indexed: true },
    label:        { type: 'string', nullable: false },
    slug:         { type: 'string', nullable: false, indexed: true },
    version:      { type: 'string', nullable: false },
    gitUrl:       { type: 'string', nullable: false },
    gitRef:       { type: 'string', nullable: false },
    installedSha: { type: 'string' },
    installPath:  { type: 'string', nullable: false },
    marketplaceJson: { type: 'jsonb' },
    enabled:      { type: 'boolean', nullable: false },
    enabledPlugins: { type: 'string[]' },
    enabledSkills:  { type: 'string[]' },
    pmpCompatibility: { type: 'string' },
    lastValidatedAt: { type: 'datetime' },
    lastValidationResult: {
      type: 'enum',
      enum: ['ok', 'warn', 'fail', 'unknown'],
    },
  },
})
```

`lane`: **relational** (server-authoritative LWW). The
git SHA is the natural conflict-resolution key.

### 18.5.2 The application hooks (consumed by the UI)

```ts
// src/application/hooks/use-skill-packages.ts
import { useEntity, useEntities, useEntityMutation } from '@prometheus-ags/prometheus-entity-management/react'
import { useAppContext } from '@/presentation/app-context'
import { skillPackageEntity } from '@/domain/entities/skill-package'

export function useSkillPackageList() {
  return useEntities(skillPackageEntity, { orderBy: { label: 'asc' } })
}

export function useInstallSkillPackage() {
  const { useCases } = useAppContext()
  return useEntityMutation(skillPackageEntity, 'installSkillPackage', {
    optimistic: true,
    execute: (spec: SkillPackageSpec) => useCases.skillPackages.install(spec),
  })
}

export function useUpgradeSkillPackage() {
  const { useCases } = useAppContext()
  return useEntityMutation(skillPackageEntity, 'upgradeSkillPackage', {
    optimistic: true,
    execute: ({ id, targetSha }) => useCases.skillPackages.upgrade(id, targetSha),
  })
}

export function useValidateSkillPackage() {
  const { useCases } = useAppContext()
  return useEntityMutation(skillPackageEntity, 'validateSkillPackage', {
    execute: (id: string) => useCases.skillPackages.validate(id),
  })
}

export function useRemoveSkillPackage() {
  const { useCases } = useAppContext()
  return useEntityMutation(skillPackageEntity, 'removeSkillPackage', {
    execute: ({ id, confirm }) => useCases.skillPackages.remove(id, confirm),
  })
}
```

### 18.5.3 The application use case (install example)

```ts
// src/application/use-cases/skill-packages/install-skill-package.ts
import type { SkillPackagePort } from '@/domain/ports/skill-package-port'
import { DomainError } from '@/domain/errors/domain-error'

export function makeInstallSkillPackage(deps: {
  skillPackages: SkillPackagePort
  harness: HarnessPort
}) {
  return async function installSkillPackage(spec: SkillPackageSpec) {
    if (!spec.gitUrl) throw new DomainError('install-skill-package: gitUrl required')

    // 1. git clone to ~/.prometheus/skill-packages/<id>/
    const target = await deps.skillPackages.resolveInstallPath(spec.id)
    if (await deps.skillPackages.exists(target)) {
      throw new DomainError(`install-skill-package: ${spec.id} already installed`)
    }
    await deps.skillPackages.gitClone(spec, target)

    // 2. Verify the install contract (the 4 conditions)
    const report = await deps.skillPackages.validateInstallContract(target)
    if (!report.ok) {
      await deps.skillPackages.rm(target)
      throw new DomainError(
        `install-skill-package: contract failed: ${report.errors.join('; ')}`
      )
    }

    // 3. Register with the active harness
    await deps.harness.registerSkillPackage(target, spec.id)

    // 4. Persist the PEM entity
    return deps.skillPackages.upsertSkillPackage({
      id: spec.id, label: spec.label, slug: spec.id,
      version: report.version, gitUrl: spec.gitUrl, gitRef: spec.gitRef,
      installedSha: report.installedSha, installPath: target,
      marketplaceJson: report.marketplaceJson, enabled: true,
      enabledPlugins: report.plugins, enabledSkills: report.skills,
      pmpCompatibility: report.pmpCompatibility,
      lastValidatedAt: new Date().toISOString(), lastValidationResult: 'ok',
    })
  }
}
```

### 18.5.4 The infrastructure port and Tauri command

```ts
// src/domain/ports/skill-package-port.ts
import type { SkillPackageSpec, ValidationReport, SkillPackage } from '@/domain/entities/skill-package'

export interface SkillPackagePort {
  resolveInstallPath(id: string): Promise<string>
  exists(path: string): Promise<boolean>
  gitClone(spec: SkillPackageSpec, target: string): Promise<void>
  validateInstallContract(target: string): Promise<ValidationReport>
  rm(path: string): Promise<void>
  upsertSkillPackage(pkg: SkillPackage): Promise<void>
  listInstalled(): Promise<SkillPackage[]>
}
```

The Tauri commands (in `crates/prometheus-companion/src/commands/skill_packages.rs`)
implement the port:

```rust
#[tauri::command]
pub async fn install_skill_package(
    state: State<'_, AppState>,
    spec: SkillPackageSpec,
) -> Result<SkillPackage, String> {
    let target = state.skill_packages
        .resolve_install_path(&spec.id)
        .await.map_err(|e| e.to_string())?;
    if state.skill_packages.exists(&target).await {
        return Err(format!("{} already installed", spec.id));
    }
    state.skill_packages.git_clone(&spec, &target).await
        .map_err(|e| e.to_string())?;
    let report = state.skill_packages.validate_install_contract(&target).await
        .map_err(|e| e.to_string())?;
    if !report.ok {
        state.skill_packages.rm(&target).await.ok();
        return Err(format!("contract failed: {:?}", report.errors));
    }
    state.harness.register_skill_package(&target, &spec.id).await
        .map_err(|e| e.to_string())?;
    state.skill_packages.upsert(&report.to_pem()).await
        .map_err(|e| e.to_string())?;
    Ok(report.to_pem())
}
```

### 18.5.5 The UI page

The "Plugins & Skill Packages" page lives in Settings and
shows: the connected list, a "Discover" tab, and a health
section. See the joint HMA spec §4.2 for the wireframe
and the styling (CLEAN + kebab-case + shadcn Base UI).

### 18.5.6 The Realtime skill-refiner panel

The Companion's UI also has a **"Refinement queue" panel**
that surfaces the `realtime-skill-refiner` HMA skill
(see
[`/Users/gqadonis/Projects/hybrid-mobile-architecture-src/docs/04-skill-refiner-loop.md`](file:///Users/gqadonis/Projects/hybrid-mobile-architecture-src/docs/04-skill-refiner-loop.md)).

The 5-stage Detect → Triage → Refine → Verify → Ship
loop always halts at Triage for human approval (per the
architecture review's "producer never grades its own
work" rule).

### 18.5.7 The hooks-reliability enforcement

The Companion's `doctor` command runs the HMA's
`scripts/verify-hooks-reliability.sh` against every
installed skill package that ships a `hooks/hooks.json`.
The verify script checks the 9 fixes from the
architecture review §6 (per
[`/Users/gqadonis/Projects/hybrid-mobile-architecture-src/docs/03-hooks-reliability.md`](file:///Users/gqadonis/Projects/hybrid-mobile-architecture-src/docs/03-hooks-reliability.md)).

### 18.5.8 The log monitor for connected services

The Companion's `log_monitor` Tauri command tails every
connected service's log file (including those declared
in the HMA's plist or the Companion's own substrate
services). On a "FATAL" pattern, the monitor calls the
`realtime-skill-refiner` skill via the A2UI bridge. See
the joint HMA spec §7 for the full pattern.

### 18.5.9 The Supervisors panel

The Companion's UI has a **"Supervisors" panel** (in
Settings → Supervisors) that lists every launchd /
systemd / Scheduled Task plist managed by the HMA's
`launchagent-supervisor` skill. Per-row: status, last
exit, self-heal count, "View plist", "Restart", "Disable",
"Re-bootstrap".

### 18.5.10 Skills used (this section)

- `hybrid-mobile-architecture/skills/connected-skill-packages` (new in HMA v0.2.0)
- `hybrid-mobile-architecture/skills/realtime-skill-refiner` (new)
- `hybrid-mobile-architecture/skills/claude-hooks-reliability` (new)
- `hybrid-mobile-architecture/skills/launchagent-supervisor` (new)
- `hybrid-mobile-architecture/skills/tauri-tray-app` (new)
- `prometheus-skill-pack/skills/pem-local-first` (the entity pattern)
- `prometheus-skill-pack/skills/process/skill-refiner` (the underlying refinement)
- `prometheus-skill-pack/skills/process/sycophancy-correction` (the bug detector)

---

## 18.6 · Auto Skill Package Integration (the no-friction path)

The "Connected Skill Packages" section (§18.5) is the
**manual** path: install, upgrade, validate, remove.
This section is the **no-friction** path: the operator
points the Companion at a source, and the Companion
auto-detects new skills and installs them.

Two source types are supported:

| Source | What it is | How the Companion watches it |
|---|---|---|
| **Git URL** | a git repo with a `marketplace.json` at the root (the HMA, the PMP, any third-party marketplace) | `git fetch` + `git diff` every N minutes (configurable; default 15 min) |
| **Local directory** | a directory on disk with a `SKILL.md` per subdir | `notify` (macOS) / `inotifywait` (Linux) / `ReadDirectoryChangesW` (Windows) for fs events |

The full spec is at
`/Users/gqadonis/Projects/hybrid-mobile-architecture-src/docs/08-auto-skill-package-integration.md`.
The HMA-side `auto-skill-package-integration` skill ships
in HMA v0.2.0.

### 18.6.1 The data model (PEM 3.x, application layer)

```ts
// src/domain/entities/skill-source.ts
import { defineEntity } from '@prometheus-ags/prometheus-entity-management'

export const skillSourceEntity = defineEntity({
  name: 'skillSource',
  privacyClass: 'local',   // never server-synced
  fields: {
    id: { type: 'id', clientId: true },
    label: { type: 'string' },
    kind: { type: 'enum', enum: ['git', 'local-dir'] },
    location: { type: 'string' },
    branch: { type: 'string' },
    mode: { type: 'enum', enum: ['confirm', 'auto'] },
    pollIntervalSec: { type: 'number' },
    enabled: { type: 'boolean' },
    lastSeenSha: { type: 'string' },
    lastSeenMtime: { type: 'number' },
    lastCheckAt: { type: 'datetime' },
    lastDetectedSkills: { type: 'string[]' },
    verifiedPublisher: { type: 'boolean' },
  },
})

// src/domain/entities/detected-skill.ts
export const detectedSkillEntity = defineEntity({
  name: 'detectedSkill',
  privacyClass: 'local',
  fields: {
    id: { type: 'id', clientId: true },
    sourceId: { type: 'string', indexed: true },
    packageId: { type: 'string' },
    skillName: { type: 'string' },
    detectedAt: { type: 'datetime' },
    installPath: { type: 'string' },
    status: { type: 'enum', enum: [
      'new', 'confirmed', 'installing', 'installed', 'skipped', 'failed'
    ] },
    errorMessage: { type: 'string' },
  },
})
```

`lane`: **local** (never server-synced). The detected
state is local to the operator's machine.

### 18.6.2 The application hooks (consumed by the UI)

```ts
// src/application/hooks/use-skill-sources.ts
import { useEntities, useEntity, useEntityMutation } from '@prometheus-ags/prometheus-entity-management/react'
import { useAppContext } from '@/presentation/app-context'
import { skillSourceEntity, detectedSkillEntity } from '@/domain/entities/skill-source'
import { detectedSkillEntity } from '@/domain/entities/detected-skill'

export function useSkillSourceList() {
  return useEntities(skillSourceEntity, { orderBy: { label: 'asc' } })
}

export function useDetectedSkillList() {
  return useEntities(detectedSkillEntity, {
    where: { status: 'new' },
    orderBy: { detectedAt: 'desc' },
  })
}

export function useAddSkillSource() {
  const { useCases } = useAppContext()
  return useEntityMutation(skillSourceEntity, 'addSkillSource', {
    optimistic: true,
    execute: (spec: SkillSourceSpec) => useCases.skillSources.add(spec),
  })
}

export function useConfirmDetectedSkill() {
  const { useCases } = useAppContext()
  return useEntityMutation(detectedSkillEntity, 'confirmDetectedSkill', {
    execute: (id: string) => useCases.skillSources.confirm(id),
  })
}

export function useCheckAllSkillSources() {
  const { useCases } = useAppContext()
  return useCases.skillSources.checkAll()
}
```

### 18.6.3 The application use case (the periodic check)

```ts
// src/application/use-cases/skill-sources/periodic-check.ts
import type { SkillSourcePort } from '@/domain/ports/skill-source-port'
import type { SkillPackagePort } from '@/domain/ports/skill-package-port'
import type { HarnessPort } from '@/domain/ports/harness-port'
import { DomainError } from '@/domain/errors/domain-error'

export function makePeriodicCheck(deps: {
  skillSources: SkillSourcePort
  skillPackages: SkillPackagePort
  harness: HarnessPort
}) {
  return async function periodicCheck() {
    const sources = await deps.skillSources.listAll()
    const results = []

    for (const source of sources) {
      if (!source.enabled) continue

      let detected = []
      try {
        if (source.kind === 'git') {
          detected = await deps.skillSources.pollGit(source)
        } else {
          detected = await deps.skillSources.scanLocalDir(source)
        }
      } catch (err) {
        results.push({ source, error: err.message })
        continue
      }

      // 4-condition install contract on each
      for (const skill of detected) {
        const report = await deps.skillPackages.validateInstallContract(skill.installPath)
        if (!report.ok) {
          await deps.skillSources.recordFailure(source, skill, report.errors)
          continue
        }

        if (source.mode === 'auto' && source.verifiedPublisher) {
          // Auto-install
          await deps.skillPackages.installFromPath(skill)
          await deps.harness.registerSkillPackage(skill.installPath, skill.packageId)
        }
        // (else: confirm mode — surface in UI as a `detectedSkill`)
      }
    }

    return results
  }
}
```

The periodic check is called by a timer:

- **macOS:** a `StartInterval` LaunchAgent that calls
  `prometheus-substrate periodic-check` every N minutes
- **Linux:** a systemd `.timer` + `.service`
- **In the Companion itself:** a `tokio::spawn` task on
  startup that runs every N minutes

### 18.6.4 The Tauri commands

```rust
// crates/prometheus-companion/src/commands/skill_sources.rs
#[tauri::command]
pub async fn add_skill_source(spec: SkillSourceSpec) -> Result<SkillSource, String>;

#[tauri::command]
pub async fn remove_skill_source(id: String) -> Result<(), String>;

#[tauri::command]
pub async fn check_skill_source(id: String) -> Result<Vec<DetectedSkill>, String>;

#[tauri::command]
pub async fn check_all_skill_sources() -> Result<HashMap<String, Vec<DetectedSkill>>, String>;

#[tauri::command]
pub async fn confirm_detected_skill(id: String) -> Result<(), String>;

#[tauri::command]
pub async fn skip_detected_skill(id: String) -> Result<(), String>;
```

### 18.6.5 The UI (the "Auto-integrate" panel)

A new "Auto-integrate" panel in the Connected Skill
Packages page shows:

- The watched sources (git URLs and local directories)
- The detected-but-not-installed skills (one row each)
  with "Install" / "Skip" actions
- The installed skills (with their source attribution)
- A per-source "Last checked at" + "Last detected" + "Mode"
- A per-source "Check now" button (manual re-scan)
- A "Watch a directory…" button (opens a file picker)
- A "Watch a git URL…" button (opens a URL form)

See the HMA spec §7 for the full wireframe.

### 18.6.6 The security considerations

The local-directory source is the **most dangerous**
source type, because a malicious file in a watched
directory could be auto-installed if the operator chose
auto mode for an untrusted directory. Defenses:

1. **The 4-condition install contract** — `SKILL.md`
   files are validated against the contract; malicious
   files (e.g. with embedded `curl ... | bash`) are
   flagged
2. **The `verifiedPublisher` flag** — only sources the
   operator has marked trusted can run in auto mode
3. **The "auto mode" warning** — when the operator tries
   to enable auto mode for an unverified source, the
   Companion shows: "Auto mode is risky for unverified
   sources. Confirm mode is recommended."
4. **The skill-refiner loop** — if a watched skill
   produces a sycophancy flag or a log monitor hit, the
   Companion auto-opens a `bugTicket` and runs the
   realtime skill-refiner loop

### 18.6.7 The recommended sources (shipped with the Companion)

The Companion ships with a default
`recommended-sources.json`:

```jsonc
[
  {
    "id": "hma-marketplace",
    "label": "KnowMe Builder (Hybrid Mobile Architecture)",
    "kind": "git",
    "location": "https://github.com/Know-Me-Tools/hybrid-mobile-architecture-skill.git",
    "branch": "main",
    "mode": "confirm",
    "pollIntervalSec": 900,
    "verifiedPublisher": true
  },
  {
    "id": "prometheus-skill-pack",
    "label": "Prometheus Skill Pack",
    "kind": "git",
    "location": "https://github.com/Prometheus-AGS/prometheus-skill-system.git",
    "branch": "main",
    "mode": "confirm",
    "pollIntervalSec": 900,
    "verifiedPublisher": true
  },
  {
    "id": "user-shared-skills",
    "label": "My team's shared skills",
    "kind": "local-dir",
    "location": "~/my-org-skills",
    "mode": "auto",
    "verifiedPublisher": true
  }
]
```

### 18.6.8 Skills used (this section)

- `hybrid-mobile-architecture/skills/auto-skill-package-integration`
  (new in HMA v0.2.0; the canonical source of truth for
  this feature)
- `hybrid-mobile-architecture/skills/connected-skill-packages`
  (the install / upgrade / validate / remove operations)
- `prometheus-skill-pack/skills/pem-local-first` (the entity
  pattern)
- `prometheus-skill-pack/skills/process/skill-refiner` (the
  malicious-skill detector)

---

## 19 · Mobile parity (Pillar 6 — Flutter + Rust over FFI)

The same Rust crate (`prometheus-companion_lib`) ships as the
mobile binary — the only change is the **UI shell**: a separate
Flutter app (Dart) that talks to the Rust core over FFI via
`flutter_rust_bridge`. The React frontend that powers desktop
**does not** run on mobile. Tauri 2.0 mobile is explicitly out
of scope (Tauri is the desktop shell, full stop).

The HMA's central architectural commitment is exactly this:
**Flutter mobile / Tauri desktop, one Rust core, FFI seam**.
Validated by 1Password (Rust core + per-platform shells) and
AppFlowy (Flutter + Rust core). Same crates, different shells,
one substrate binary.

### 19.1 The mobile app skeleton

```
prometheus-companion/
├── apps/
│   ├── desktop/                    # Tauri 2.0 + React 19 + Vite
│   │   ├── src/                    # the React app
│   │   └── src-tauri/              # the Tauri host
│   └── mobile/                     # NEW — Flutter + Rust FFI
│       ├── lib/                    # Dart UI (Material 3 + Cupertino)
│       ├── ios/                    # iOS Runner
│       ├── android/                # Android Runner
│       └── rust/                   # ← flutter_rust_bridge generated
│                                     bindings; re-exports the substrate
├── crates/
│   ├── prometheus-substrate/       # the Rust core (shared)
│   └── prometheus-companion_lib/   # the FFI surface (new)
└── pubspec.yaml                    # Flutter dependencies
```

The `prometheus-companion_lib` crate is the **FFI surface** —
it wraps `prometheus-substrate` and exposes the same async
functions to Dart that the desktop React app consumes via
Tauri commands. `flutter_rust_bridge` generates the Dart
bindings at build time from the Rust function signatures.

### 19.2 The mobile feature split

The substrate has a `mobile` feature that excludes the heavy
services:

```toml
# crates/prometheus-substrate/Cargo.toml
[features]
default = ["desktop"]
mobile = ["client", "skills", "literllm", "sovereign", "exec-core"]
desktop = ["client", "executor", "knowledge", "skills",
           "forge", "surface", "literllm", "exec", "sovereign",
           "research"]
```

A `prometheus mobile build` invocation uses
`--no-default-features --features mobile` so the FFI binary is
small enough for a phone. The Flutter app is ~10 MB; the FFI
binary is ~25 MB; total ~35 MB on TestFlight.

### 19.3 The mobile wire format (over the FFI seam)

The Flutter app calls into the FFI binary **synchronously** for
local operations (skill search, receipt read) and **over Iroh
QUIC** for offloaded operations (skill execution). The wire
format is the **same AG-UI SSE protocol** the desktop uses. The
mobile app opens an Iroh `quinn::Connection`, wraps it in an
HTTP/1.1 request, and the node serves the same endpoints. Zero
new protocol design — same Rust protocol implementation.

### 19.4 The mobile sync story

The mobile app is a **thin client for orchestration**, not a peer
that runs the full stack. The mobile substrate enforces the
same identity, signs the same receipts, and routes everything
through sovereign-sync. If the phone is offline, it queues jobs
in the local learning queue (the same one `prometheus-learning-
worker` already drains) and sends them when connectivity returns.

### 19.5 The Flutter native plugins

| Flutter package | What it does |
|---|---|
| `local_auth` | Face/Touch ID gate for the device identity |
| `flutter_secure_storage` | Store the Ed25519 device key in Keychain / Keystore |
| `flutter_local_notifications` | Local push for receipts, completed jobs |
| `flutter/services.dart` HapticFeedback | Confirmations, alerts (built-in) |
| `geolocator` | Optional: tag receipts with location |
| `device_info_plus` + `dart:io` `Platform` | Detect platform for the dashboard |

### 19.6 The "no desktop chrome on mobile" rule (re-stated)

The HMA titlebar skill is explicit: the title bar is a
**desktop-window** concern. Flutter has its own per-platform
chrome (Cupertino `NavigationBar` on iOS, Material 3
`NavigationBar` on Android). The mobile app **does not** use
`tauri-custom-titlebar`; that skill is desktop-only.

### Skills used
- `flutter-rust-ffi/SKILL.md` (the FFI codegen + binding
  pattern)
- `flutter-building-forms/SKILL.md` (the Form / Riverpod
  patterns)
- `mobile-navigation/SKILL.md` (the bottom-bar rule — also
  applies to Flutter via `NavigationBar` widgets)
- `dart-flutter-patterns/SKILL.md` (the general Dart/Flutter
  idiom pack)

### 19.7 The `apps/mobile/pubspec.yaml` (canonical Flutter deps)

```yaml
name: prometheus_companion_mobile
description: Prometheus Mobile — Flutter shell over the Rust substrate
publish_to: 'none'
version: 0.1.0+1

environment:
  sdk: ^3.6.0
  flutter: ">=3.27.0"

dependencies:
  flutter:
    sdk: flutter
  # FFI to the Rust substrate
  flutter_rust_bridge: ^2.0
  # Platform-native
  local_auth: ^2.3              # Face / Touch ID gate
  flutter_secure_storage: ^9.2   # Ed25519 keypair in Keychain / Keystore
  flutter_local_notifications: ^17.0
  geolocator: ^13.0
  device_info_plus: ^11.0
  # Iroh P2P over QUIC (Dart side)
  iroh: ^0.1
  # State + forms
  flutter_riverpod: ^2.5
  freezed_annotation: ^2.4
  json_annotation: ^4.9

dev_dependencies:
  flutter_test:
    sdk: flutter
  flutter_lints: ^4.0
  build_runner: ^2.4
  freezed: ^2.5
  json_serializable: ^6.8

flutter:
  uses-material-design: true
```

---

## 20 · Accessibility (WCAG 2.2 AA — non-negotiable)

The HMA `a11y-gate` skill is the contract. The Companion passes
the gate or it doesn't ship.

### 20.1 The checklist (per `a11y-gate/SKILL.md`)

- [ ] **Contrast ≥ AA** — 4.5:1 body text, 3:1 large text / UI
      components / focus indicators, in BOTH themes
- [ ] **Every actionable element has an accessible name** and a
      visible label or tooltip
- [ ] **Keyboard reachable and operable** — full flow with Tab /
      Shift-Tab / Enter / Escape; logical focus order; no
      keyboard trap; visible focus ring
- [ ] **Images/media have alt/label** — decorative marked as such;
      informative described
- [ ] **Reduced motion honored** — respect `prefers-reduced-motion`
- [ ] **Live regions announce streaming** — streaming `text` /
      `thinking` and status changes on `toolUse` / `skill` are
      announced (aria-live)
- [ ] **Target size ≥ 24×24 CSS px** (AA 2.5.8), ≥ 44px recommended
      for primary touch
- [ ] **Status not by color alone** — pair color with icon/text
      (service status, sync chip, tray state)

### 20.2 The PostToolUse reminder hook

The HMA scaffolded project installs a `PostToolUse` hook
(matcher: `Write|Edit`) that, when a UI file is touched
(`.tsx`/`.jsx` under `src/`), prints a reminder to run this
gate. The hook is advisory (non-blocking) — it does not judge
the code, it ensures the gate isn't silently skipped.

### 20.3 The review loop (per `tauri-ui-review/SKILL.md`)

The screenshot-driven review at **320, 768, 1024, 1440** in
**both themes** runs after every UI change. The full list of
8 shots per themed surface is in the tauri-ui-review skill.

### Skills used
- `a11y-gate/SKILL.md` (the checklist)
- `tauri-ui-review/SKILL.md` (the screenshot loop)

---

## 21 · Build, sign, and release

The Tauri 2.0 build, sign, and release path is the production
standard. The Companion follows it.

### 21.1 The target matrix

| OS | Tauri target | Notes |
|---|---|---|
| macOS (Apple Silicon) | `aarch64-apple-darwin` | DMG + ZIP, codesigned + notarized |
| macOS (Intel) | `x86_64-apple-darwin` | DMG + ZIP, codesigned + notarized |
| macOS (universal) | universal binary via `lipo` | Single DMG |
| Windows (x64) | `x86_64-pc-windows-msgsvc` | MSI + NSIS, Authenticode-signed |
| Windows (ARM64) | `aarch64-pc-windows-msgsvc` | MSI + NSIS, Authenticode-signed |
| Linux (x64) | `x86_64-unknown-linux-gnu` | AppImage + .deb + .rpm |
| Linux (ARM64) | `aarch64-unknown-linux-gnu` | .deb + .rpm |
| iOS | `aarch64-apple-ios` (device) + `aarch64-apple-ios-sim` (sim) | TestFlight, then App Store |
| Android | `aarch64-linux-android` + `armv7-linux-androideabi` + `x86_64-linux-android` | Play Store (AAB) |

### 21.2 The release pipeline

```bash
# 1. bump version in Cargo.toml, tauri.conf.json, package.json
# 2. pnpm install
# 3. pnpm tauri build --target <target>
# 4. pnpm tauri sign (for macOS / Windows)
# 5. xcrun notarytool submit ... --wait (macOS)
# 6. upload to GitHub Releases / App Store / Play Store
```

### 21.3 The auto-updater

Tauri 2.0 has a built-in auto-updater (`tauri-plugin-updater`).
The Companion uses it for desktop releases. Mobile uses the App
Store / Play Store update path.

The updater signs updates with a Tauri signing key generated at
first build (`npx @tauri-apps/cli signer generate -w
~/.tauri/companion.key`). The public key is in `tauri.conf.json`
under `plugins.updater.pubkey`. The private key is **only** in
the maintainer's CI secret store.

### 21.4 The release cadence

- **Patch releases** (0.0.x) — weekly, bug fixes only
- **Minor releases** (0.x.0) — monthly, new features
- **Major releases** (x.0.0) — quarterly, breaking changes

The release notes are generated from `CHANGELOG.md` (enforced by
the `validate-changelog.sh` script).

### Skills used
- `tauri-react-vite/SKILL.md` (the build pattern)
- `dependency-pin-discipline/SKILL.md` (the dep-update cadence)

---

## 22 · Test plan — tauri-ui-review + a11y-gate + BDD

Three independent test layers run in CI before any merge.

### 22.1 Layer 1 — `tauri-ui-review` (screenshot-driven)

- 8 screenshots per themed surface (320/768/1024/1440 in both
  themes)
- The checklist in `tauri-ui-review/SKILL.md` (no horizontal
  overflow, no layout shift, complete product coverage, hierarchy
  via scale contrast, intentional rhythm, designed hover/focus/
  active states, both themes intentional, not a default template,
  flat 2.0, shadcn/assistant-ui mounted at the real boundary,
  motion is compositor-friendly, touch targets ≥ 44px)
- Failure → PR is blocked; the screenshot diff is the artifact

### 22.2 Layer 2 — `a11y-gate` (WCAG 2.2 AA)

- The checklist in `a11y-gate/SKILL.md` (§20.1)
- Automated axe scan via Playwright (advisory, not sufficient)
- Manual keyboard walk (Tab, Shift-Tab, Enter, Escape, focus
  ring visible)
- Screen reader walk on macOS VoiceOver (one full flow per
  surface)
- Failure → PR is blocked; the violation is the artifact

### 22.3 Layer 3 — BDD (cucumber-rs)

- One feature file per user-facing flow
  - `features/services/restart.feature`
  - `features/devices/pair.feature`
  - `features/chat/streaming.feature`
  - `features/skills/invoke.feature`
  - `features/runs/verify.feature`
- Step definitions in `crates/prometheus-companion/tests/bdd/`
  that drive the actual Tauri window via the test harness
- Failure → PR is blocked; the failing scenario is the artifact

### 22.4 The `hybrid-runtime-verification` gate

The HMA `hybrid-runtime-verification/SKILL.md` is the completion
gate for the whole app. The Companion is not "done" until a clean
checkout proves:

- Clean install
- Production build
- Real launch (on macOS, Windows, Linux)
- Persistence (state survives a restart)
- One public-boundary workflow (P2P pair, skill invoke, run
  verify)
- Health / readiness
- The selected Compose / Kustomize / installer profile

### Skills used
- `tauri-ui-review/SKILL.md`
- `a11y-gate/SKILL.md`
- `bdd-cucumber-rs/SKILL.md` (the BDD harness)
- `hybrid-runtime-verification/SKILL.md` (the completion gate)

---

## 23 · Implementation phases (the punch list)

This is the punch list — the ordered list of work that turns the
scaffold into the v1.0. Each phase is sized to one engineering
week; each phase ends with a verifiable checkpoint.

### Phase 0 — Rule files + CLEAN skeleton (week 0, 2-3 days)

This phase runs **before** Phase 1. It lands the rule files, the
audit scripts, and the CLEAN folder skeleton so every later phase
inherits a working `pnpm audit:layer && pnpm audit:naming` gate.

- [ ] Create `templates/rules/agents.md.tmpl` (the source for
      `AGENTS.md`)
- [ ] Create `templates/rules/claude.md.tmpl` (the source for
      `CLAUDE.md`)
- [ ] Write `scripts/gen-rule-files.sh` (renders the templates to
      `AGENTS.md` and `CLAUDE.md` with a GENERATED banner)
- [ ] Generate the first `AGENTS.md` and `CLAUDE.md`
- [ ] Write `scripts/audit-naming.sh` (kebab-case enforcement)
- [ ] Write `scripts/audit-layer.sh` (CLEAN layer enforcement)
- [ ] Write `scripts/audit-generated.sh` (catches hand-edits to
      the generated rule files)
- [ ] Create the four `src/{domain,application,infrastructure,presentation}/`
      directories
- [ ] Move `pnpm create tauri-app`'s default `src/` into
      `src/presentation/` (or recreate it there) so the empty
      skeleton is in the right place
- [ ] Add `pnpm audit` and the four sub-commands to
      `package.json`
- [ ] Wire `pnpm audit` into the pre-push hook
- [ ] Verify: every check in Appendix D step 0.1-0.6 passes

**Definition of done:** the four `audit:layer`, `audit:naming`,
`audit:generated`, and `audit:secret-leak` scripts all pass on
the empty skeleton, and the rule files are the **only** way any
later phase's code can be written without violating the rules.

### Phase 1 — Foundation (week 1)

- [ ] Initialize the superworkspace (merge `prometheus-companion`
      into the prometheus-skill-pack superworkspace per the
      architecture review §5.3.2)
- [ ] Install `sccache`, `mold`/`zld`, `cargo-nextest`,
      `cargo-cache` per §5.3
- [ ] Author `assets/templates/design-tokens/tokens.toml` with
      the full color/spacing/type/motion/breakpoint scale
- [ ] Write `scripts/gen-design-tokens.sh` (generates
      `src/theme.css` and `crates/.../theme.rs`)
- [ ] Initialize shadcn/ui with Base UI: `npx shadcn@latest init
      --base-ui --yes`
- [ ] Add the `components.json` keys for `base: "base-ui"`,
      `style: "new-york"`, `cssVariables: true`
- [ ] Add the shadcn components we'll use (§9.3)
- [ ] Verify: `pnpm tauri dev` opens, the title bar (next
      phase) renders, the tokens compile

### Phase 2 — Custom title bar (week 2)

- [ ] Update `tauri.conf.json` to `decorations: false`,
      `titleBarStyle: "Overlay"`, `hiddenTitle: true`
- [ ] Write `src/components/titlebar/Titlebar.tsx` per §6.2
- [ ] Write `src/components/titlebar/TrafficButton.tsx` and
      `WindowsLinuxControls.tsx`
- [ ] Add the platform-detection helper (`src/lib/platform.ts`)
- [ ] Add the `data-tauri-drag-region` + `startDragging()` mousedown
      handler
- [ ] Add the right-click context-menu suppression on the bar
- [ ] Add the focus-visible ring
- [ ] Add the `aria-label` on every control
- [ ] Add the keyboard shortcuts (`Cmd+W`, `Alt+F4`, `Cmd+M`)
- [ ] Run `a11y-gate` on the result
- [ ] Run `tauri-ui-review` at 320/768/1024/1440 in both themes
- [ ] Verify by hand: drag works on the empty region, controls
      fire, double-click maximizes, Esc minimizes (macOS)
- [ ] Verify the web build does not crash (the surface-decision
      rule)

### Phase 3 — Tray + popover + health aggregator (week 3)

- [ ] Add `tauri-plugin-positioner` (TrayCenter placement)
- [ ] Add the tray icon Rust code (`crates/prometheus-companion/src/tray.rs`)
- [ ] Generate the 5 icon states (green / yellow / red / gray /
      spinner) from a single SVG
- [ ] Add the `tray-popover` window to `tauri.conf.json`
- [ ] Add the `tray-state` event emitter
- [ ] Add the popover React component (`src/components/tray/TrayPopover.tsx`)
- [ ] Add the popover close-on-blur, close-on-Esc
- [ ] Add the in-process `prometheus-substrate` library to
      `crates/`
- [ ] Add the `supervisor` actor
- [ ] Add the `health` aggregator
- [ ] Verify: launch, see the tray icon, click, see the popover,
      restart a service from the popover, see the icon turn yellow
      → red → green

### Phase 4 — PEM integration + dashboard (week 4)

- [ ] Add `@prometheus-ags/prometheus-entity-management` to
      `package.json`
- [ ] Set up PGlite (`createPGlitePersistenceAdapter`)
- [ ] Register the entity types: `service`, `device`, `run`,
      `skill`, `provider`, `model`, `plugin`, `setting`
- [ ] Register one transport per entity type (calls into
      `src/lib/tauri.ts`)
- [ ] Write the feature hooks: `useServiceList`, `useService`,
      `useRestartService`, etc.
- [ ] Write the routing (`src/routes/`): root, services, devices,
      chat, skills, runs, settings
- [ ] Write the main dashboard pages (§12)
- [ ] Write the `mobile-navigation`-style rail/bar chrome
- [ ] Verify: the dashboard opens, the service cards render with
      live data, restart works, log tail streams

### Phase 5 — A2UI + AG-UI + Chat (week 5)

- [ ] Add the A2UI component registry (`src/a2ui/registry.ts`)
- [ ] Write the A2UI renderer (`src/a2ui/A2UIRenderer.tsx`)
- [ ] Add the A2UI Rust projector (in `crates/prometheus-substrate/src/a2ui.rs`)
- [ ] Add the AG-UI SSE emitter (in `crates/prometheus-substrate/src/agui.rs`)
- [ ] Add Assistant-UI Thread in the Chat page
- [ ] Add the ContentBlocks renderer
- [ ] Add the ActionRouter for A2UI actions
- [ ] Verify: chat with a skill, A2UI surfaces render, actions
      round-trip through UAR, AG-UI events stream

### Phase 6 — P2P pairing + sovereign-sync (week 6)

- [ ] Add the sovereign-sync dependency to the substrate
- [ ] Add the Ed25519 identity generation on first run
- [ ] Add the OS secure-store integration
      (`tauri-plugin-stronghold`)
- [ ] Add the Iroh transport (the substrate's `sovereign_sync.rs`)
- [ ] Add the pairing flow: QR ticket (`prometheus://pair?ticket=…`); a 6-digit PAKE short code is optional and undecided (see `change-cpc-006`). No Noise handshake — iroh QUIC/TLS 1.3 authenticates the transport
- [ ] Write the Devices page
- [ ] Add the capability manifest per device
- [ ] Add the authorization check at the Axum boundary
- [ ] Verify: pair a second device, see it in the Devices page,
      send a request from it, see the result

### Phase 7 — Providers, models, settings (week 7)

- [ ] Add the Liter-LLM registry reader (in the substrate)
- [ ] Add the provider / model entities to PEM
- [ ] Add the Settings → Providers page
- [ ] Add the secure-store-backed API key entry
- [ ] Add the test-connection flow
- [ ] Add the Settings → Plugins page
- [ ] Add the Settings → About page
- [ ] Verify: configure a provider, test it, see the model list,
      run a chat, see the result

### Phase 8 — Hardening (week 8)

- [ ] Run the full `tauri-ui-review` loop on every themed surface
- [ ] Run the full `a11y-gate` on every themed surface
- [ ] Run the BDD suite (`cargo nextest run` + the cucumber-rs
      features)
- [ ] Run the `hybrid-runtime-verification` completion gate
- [ ] Run the `check-mcp-health.sh` against the substrate
- [ ] Code-sign the macOS DMG
- [ ] Code-sign the Windows MSI
- [ ] Generate the Tauri signing key for the auto-updater
- [ ] Set up the auto-updater endpoint
- [ ] Cut the 0.1.0 release

### Phase 9 — Mobile target (week 9)

- [ ] `flutter create --org com.prometheus apps/mobile`
- [ ] Add `flutter_rust_bridge` to the workspace; scaffold the
      `prometheus-companion_lib` FFI crate
- [ ] Generate the Dart bindings from the Rust FFI surface
      (`flutter_rust_bridge_codegen`)
- [ ] Apply the mobile feature split on `prometheus-substrate`
      (`--no-default-features --features mobile`)
- [ ] Add `local_auth` for the Face/Touch ID device-identity
      gate
- [ ] Add `flutter_secure_storage` for the Ed25519 keypair
- [ ] Add the Material 3 / Cupertino `NavigationBar` per
      platform
- [ ] Add the mobile-only pairing flow (QR-code scan → Iroh
      node ID)
- [ ] Verify: TestFlight internal build, install on a phone,
      pair with a Companion desktop, see the substrate's health
      on the phone

### Phase 10 — Polish + launch (week 10)

- [ ] Run the full a11y gate on the mobile target too
- [ ] Run the BDD suite on the mobile target
- [ ] Cut the 1.0 release
- [ ] Write the launch post (Medium / website)
- [ ] Submit the App Store / Play Store builds
- [ ] Monitor the first-week telemetry; patch as needed

### Skills used
- All of them, in order

---

## 24 · Skill invocation map (which skill at which step)

This is the explicit map from task → skill. The CI hook on UI
edits (`a11y-gate`) and the PostToolUse reminder on
`tauri-ui-review` enforce the in-line invocations; the bigger
map is the human protocol.

| Task | Skill to invoke first | When |
|---|---|---|
| Add a new service card | `pem-local-first` → `entity-graph-web-shell` | before writing the feature |
| Add a new theme color | `hybrid-design-tokens` | before any CSS |
| Add a new shadcn component | `shadcn` MCP (or skill) | before adding to `components.json` |
| Add a new top-level route | `mobile-navigation` | before writing the route |
| Add a new A2UI component | `a2ui-surface-contract` | before registering |
| Add a new AG-UI event | `agui-event-contract` | before emitting from Rust |
| Add a new P2P message | `axum-agent-gateway` + `peer-profile-sync` | before the gossip envelope is signed |
| Add a new provider | `deploy-hybrid-agentic-stack` (BYOK rules) | before writing the storage code |
| Add a new hook | `tauri-react-vite` (typed IPC) | before the hook script |
| Add a new Tauri command | `tauri-react-vite` + `rust/error-handling` | before the Rust handler |
| Add a new Tauri window | `tauri-custom-titlebar` | before the window config |
| Touch any UI file | `tauri-ui-review` (reminder) + `a11y-gate` (reminder) | after the edit |
| Touch any service config | `rust/error-handling` + `rust/async-patterns` | before the implementation |
| Touch any cross-surface data | `sync-doctrine` | before the schema |
| Touch any entity | `pem-local-first` (the layering rule) | before the hook |
| Touch any agent | `axum-agent-gateway` (the boundary rule) | before the UAR call |
| Cut a release | `hybrid-runtime-verification` | before "done" |
| Plan a multi-phase change | `orchestrate-prometheus-application` | before the plan |
| Generate a new skill | `skill-creator` | when a gap is proven |
| Refine a skill | `skill-refiner` | when a skill has a concrete bug |

---

## 25 · Risks, tradeoffs, and open questions

### 25.1 Risks

| Risk | Severity | Mitigation |
|---|---|---|
| PGlite + Tauri + macOS sandbox integration is untested at the scale we need | High | Phase 1 spike: prove the PGlite-in-Tauri-macOS path before committing to it |
| Flutter + Rust FFI is a less-traveled path than Tauri 2.0 mobile; codegen surprises possible | Medium | Phase 9 begins with the smallest possible mobile app (one screen, one button) and grows. `flutter_rust_bridge` is mature (used by AppFlowy at scale). |
| The substrate's `mobile` feature split has not been exercised in the wild | High | Phase 9 begins with the smallest possible mobile app (one screen, one button) and grows |
| Iroh is a moving target; the API has changed across versions | Medium | Pin the version in the superworkspace; do not bump without a regression run |
| shadcn Base UI migration cost is non-zero for any component we write by hand | Medium | Migrate leaf first (button, label, field); check shadcn-generated components before writing new ones |
| The custom title bar removes OS-provided accessibility | Medium | The `a11y-gate` is the contract; run it on every change |
| ~~The substrate as in-process library means a substrate crash brings the Companion down~~ **Superseded by `change-cpc-007`:** the pack's services are adopted, not linked — a crash in one of them cannot crash this process. The remaining in-process risk is narrower: this process's own node dying. | Medium | `Supervisor` restarts an adopted service through the platform supervisor (`launchctl kickstart`/`systemctl --user restart`), with backoff and a 10-failure disable cap (§16.3); the Companion's own launchd plist (`shared/launchd/ai.prometheus.companion.plist.tmpl`) restarts the Companion process itself if it dies — verified: `minimum runtime = 15` and `semaphores = { successful exit => 0, after crash => 1 }` read back from a real `launchctl print` after loading the rendered plist |
| Tauri auto-updater on Windows requires the user to grant permission on first run | Low | Document; show a one-time prompt on first launch |
| A headless node cannot answer the macOS keychain ACL prompt for `prometheus-kbd-device`, which the pack's daemon created | Medium | `change-cpc-005`: the headless entry point refuses to start unless a signing identity is reachable unattended (`PROMETHEUS_DEVICE_KEY_FILE`, or a managed key file). It fails with exit 1 and names both remedies *before* binding anything, rather than failing mid-mutation |

### 25.2 Tradeoffs

| Decision | Alternative | Why this |
|---|---|---|
| ~~Substrate as in-process library~~ **Amended by D-02 / `change-cpc-007`:** the substrate crate (`prometheus-substrate`) is still in-process — it is this app's own Rust code, hosting this app's own node. What changed is which *services* it hosts: none of the pack's 12 services link in; they run where the pack installed them, and the substrate adopts them by manifest + platform-supervisor restart, never by linking. | Tauri sidecar (for the Companion's own node) · pack-owned adoption (for the pack's services, decided) | The Companion's own node in-process is faster and shares the trust boundary; the pack's services stay adopted because D-02 requires the pack to work fully with no Companion installed — linking them in would make the Companion a hard runtime dependency of behavior the pack must own alone |
| Tauri 2.0 over Electron | Electron is faster to ship | Tauri 2.0 has the mobile story; the substrate is Rust; smaller binary |
| shadcn Base UI over Radix | Radix is more mature | shadcn default is now Base UI; better combobox, better API; active monthly releases |
| PEM 3.x over TanStack Query | TanStack Query has more docs | Query cache is the wrong layer for local-first (per `pem-local-first`); PEM is the data layer |
| One substrate binary | Keep the 15+ processes | The architecture review is the case; the install becomes one command |
| PGlite for local store | SQLite via FFI | PGlite is in-process; no FFI; same SQL; works on web, desktop, mobile |
| Iroh over libp2p | libp2p is the canonical P2P stack for Rust | Iroh is modern, QUIC-native, NAT-traversal built-in, and imposes no DHT tax *by default*. **Amended by `change-cpc-006`:** Mainline DHT address lookup is now available via `iroh-mainline-address-lookup`, but it is **off by default** — enabling it publishes this node's signed pkarr record (endpoint id, relay URL, direct addresses) to the public BitTorrent DHT, so it is a per-operator opt-in rather than a default cost. mDNS (`iroh-mdns-address-lookup`) is on by default and stays on the local link |
| A2UI in-Companion over a separate surface | Separate web app for A2UI | The Companion is the operator's primary surface; A2UI is part of it |
| Custom title bar with platform-correct controls | Use the OS title bar | The user explicitly asked for it; brand lockup is part of the design |

### 25.3 Open questions (for Travis)

1. **App Store / Play Store branding.** Do we ship the Companion
   as "Prometheus Companion" or under the Prometheus AGS
   organization? Affects the App Store metadata, the signing
   certificate, the privacy policy URL.
2. **Data residency.** When a mobile device is paired with a node,
   is the data residency the node's, the mobile's, or both? Affects
   the GDPR / HIPAA / SOC2 posture (Tribe Health is the
   background).
3. **The substrate's Tauri command surface.** How many Tauri
   commands do we want? The architecture review's substrate
   library has a defined surface; the Companion may need a few
   more (e.g. "show in Finder", "open log in editor").
4. **The auto-updater signing key custody.** Where does the
   private signing key live? CI secret? Hardware token? Affects
   the release security model.
5. **The pairing flow UX.** Is a 6-digit short code enough, or do
   we want a second factor (e.g. biometric confirmation on the
   desktop)?
6. **The dashboard's "first run" experience.** What does the user
   see on first launch? A welcome tour? A "pair this device"
   prompt? A blank dashboard? The answer affects Phase 1.
7. **The mobile app's "offline" experience.** When the phone is
   offline and the user tries to invoke a heavy skill, do we
   queue and show "will run when online", or do we fail fast?
8. **The `prometheus-research` server in the desktop binary.** It
   is a powerful feature but it adds weight. Is it on by default,
   or opt-in? Affects the binary size and the install footprint.
9. **The substrate's licensing story.** The substrate library is
   MIT (per the architecture review); do we want a BSL/Enterprise
   tier for some pieces (e.g. sovereign-sync with specific
   transports)?

---

## 26 · Standards & rule files — `AGENTS.md` and `CLAUDE.md`

The Companion's standards live in two checked-in rule files at the
project root:

- **`AGENTS.md`** — read by every AI agent that opens this repo
  (Claude Code, OpenCode, Codex, Kimi, MiniMax, Cursor, Aider,
  Cline, Roo, Windsurf, Zed, Gemini CLI, etc.). The content is
  harness-agnostic: it states the rules, the rationale, and the
  enforcement scripts, and it does not assume any particular tool.
- **`CLAUDE.md`** — read only by Claude Code. It inherits the
  rules from `AGENTS.md` and adds Claude-Code-specific behavior
  (skill invocation triggers, the `EnterPlanMode` workflow, the
  `PostToolUse` reminder hooks, the AG-UI memory format, etc.).

Both files are generated from a single `templates/rules/` source
and pinned in version control. The full content is in
`/Users/gqadonis/Projects/prometheus/prometheus-companion/AGENTS.md`
and `.../CLAUDE.md`; this section explains the **structure** and
the **rules** that go in them.

### 26.1 The `AGENTS.md` structure (and the rules we adopted)

| Section | What goes here | Source skill |
|---|---|---|
| **§0 The Constitution** | The 7 inviolable rules (think before coding, observed problems only, security traces to a real boundary, simplicity, truth over fluency, verified vs. self-reported, preserve intent, architecture before code, test at phase completion). | `prometheus-skill-pack/AGENT_BASE_RULES.md §A` |
| **§1 Local-only validation** | All builds / tests / linting / doctors run locally. No GitHub Actions for validation. | `prometheus-skill-pack/AGENTS.md` ("Local-Only Validation MANDATORY") |
| **§2 Progress signaling** | "Starting phase X out of Y" / "Completed phase X out of Y" at every task boundary. | `prometheus-skill-pack/AGENTS.md` ("Progress Signaling MANDATORY") |
| **§3 File naming** | **Kebab-case for every TypeScript file** (and folder). The only exceptions are `__root.tsx`, `index.ts`, and framework-mandated file names. | (this project — adopted §7.1) |
| **§4 Layer rule (CLEAN)** | Domain → Application → Infrastructure → Presentation. Domain imports nothing. Application imports Domain. Infrastructure implements Application ports. Presentation imports Application + Domain. **No cross-layer imports. No `invoke()` / `listen()` in components. No `useEffect` data fetching.** | `clean-architecture/SKILL.md` + `entity-graph-web-shell/SKILL.md` |
| **§5 Tauri surface gating** | Every `@tauri-apps/*` import is wrapped in `isTauri()`. Web build must not crash. Title bar is desktop only; **mobile has its own Flutter app** (Material 3 / Cupertino `NavigationBar` per platform) — Tauri is never imported on mobile. | `tauri-custom-titlebar/SKILL.md` + `flutter-rust-ffi/SKILL.md` |
| **§6 Custom title bar** | `decorations: false` + `titleBarStyle: Overlay` + `hiddenTitle: true` on the main window. macOS traffic lights **left** (close / minimize / maximize). Windows-Linux min / max / close **right**. `data-tauri-drag-region` **and** explicit `startDragging()` on left-button mousedown. Real `<button>` with `aria-label` on every control. | `tauri-custom-titlebar/SKILL.md` + `a11y-gate/SKILL.md` |
| **§7 Mobile navigation** | Top-level destinations go in a **bottom bar** on every platform. Switch to a **rail by width** (≥ 600px). One list, never two. No `Platform.isIOS` checks. | `mobile-navigation/SKILL.md` |
| **§8 Component library** | shadcn/ui with **Base UI** primitives (`@base-ui-components/react`). No direct Radix imports. The `components.json` `base: "base-ui"` key is the source of truth. Strangler-fig for any leftover Radix. | (this project — adopted §9) |
| **§9 Entity management** | PEM 3.x is the data layer. **No TanStack Query.** Components consume hooks; hooks consume PEM. Optimistic mutations, idempotent retries, one queue not two. | `pem-local-first/SKILL.md` + `entity-graph-web-shell/SKILL.md` |
| **§10 Sync doctrine** | Three lanes (relational / CRDT / event log) chosen per entity. Two privacy classes (`public` / `trusted` server-synced, `local` never). Boot order is migrations → seed/lookup → pre-onboarding → post-onboarding → sync attach. | `sync-doctrine/SKILL.md` |
| **§11 Accessibility** | WCAG 2.2 AA. Contrast ≥ 4.5:1 body / 3:1 large. Every actionable element has an accessible name. Full keyboard operability. Reduced motion honored. Target size ≥ 24×24 (44px primary touch). Status not by color alone. | `a11y-gate/SKILL.md` |
| **§12 UI review** | Screenshot-driven review at 320 / 768 / 1024 / 1440 in both themes after every UI change. 8 shots per themed surface. `tauri-ui-review` checklist. | `tauri-ui-review/SKILL.md` |
| **§13 Secrets** | API keys go in OS secure store (Stronghold). Never in PEM, PGlite, Zustand, logs, URLs, or images. APIs return metadata, never key values. | `deploy-hybrid-agentic-stack/SKILL.md` |
| **§14 Build & release** | `pnpm tauri build --target <target>`. Code-sign macOS + Windows. Auto-updater signing key in CI secret. Release cadence: weekly patch, monthly minor, quarterly major. | (this project — adopted §21) |
| **§15 Dependencies** | Pinned versions. No `^` for breaking-deps. `pnpm install --frozen-lockfile` in CI. `dependency-pin-discipline` skill is the source of truth. | `dependency-pin-discipline/SKILL.md` |
| **§16 Hooks (Claude Code)** | `UserPromptSubmit` forced-eval hook. `PostToolUse` reminder for `tauri-ui-review` + `a11y-gate` after UI file edits. Bundled hook runtime is immutable (sha256-pinned). | `hooks/hooks.json` + `bootstrap-hook-runtime.sh` |
| **§17 Agents and skills** | What skill to invoke first for each task. The full table is in §24. | (cross-reference) |
| **§18 Anti-patterns** | No `unwrap()` in Rust. No `new ConcreteThing()` in business code. No Radix imports. No `tanstack/react-query`. No `Platform.isIOS` for nav. No hand-mirroring of design tokens. No `cargo clean` without committing the change to the lockfile. | (this project — adopted) |

### 26.2 The `CLAUDE.md` structure (Claude-Code-specific)

`CLAUDE.md` is shorter and points at `AGENTS.md` for the full set
of rules. It adds:

- **Skill triggers** — the explicit `description:` blocks that
  cause a skill to auto-fire on certain phrases. Every new skill
  registered to the project's marketplace gets a trigger list
  here.
- **The `EnterPlanMode` workflow** — when to plan, when to ask
  the user, when to just do.
- **The PostToolUse reminder hooks** — the hooks installed in
  `hooks/hooks.json` that fire after every `Write` / `Edit` to
  a UI file.
- **The AG-UI memory format** — how to write durable notes to
  `~/.prometheus/projects/<id>/` so the next session can pick
  up where this one left off.
- **The bun / pnpm choice** — `pnpm` for the Companion (matches
  the rest of the PMP stack).
- **The `[compact]` block** — a short summary block at the
  end of long sessions that survives context compaction.

### 26.3 The enforcement scripts

The `AGENTS.md` rules are enforced by these scripts, all wired
into `pnpm audit`:

| Script | Enforces | Exit code on fail |
|---|---|---|
| `scripts/audit-layer.sh` | CLEAN layer rule (§4) | 1 |
| `scripts/audit-naming.sh` | kebab-case file names (§3) | 1 |
| `scripts/audit-tokens.sh` | no hardcoded colors / radii / spacing in components | 1 |
| `scripts/audit-a11y.sh` | `a11y-gate` per-file heuristic (aria-label on buttons, no `tabindex>0`, no `outline: none` without replacement) | 1 |
| `scripts/audit-deps.sh` | pinned versions for breaking deps, no `*` in `Cargo.toml` | 1 |
| `scripts/audit-secret-leak.sh` | no API key literals, no `.env` committed | 1 |
| `scripts/audit-skill-descriptions.sh` | description < 1024 chars, third person, "USE WHEN" / "Do NOT use for" | 1 |
| `scripts/audit-progress-signals.sh` | the `## Progress Signals` section in every process skill | 1 |

`pnpm audit` is wired into the pre-push hook and into the local
CI runner. None of these scripts call GitHub Actions or any
hosted CI — per the PMP `AGENTS.md` "Local-Only Validation
MANDATORY" rule.

### 26.4 How `AGENTS.md` and `CLAUDE.md` are generated

Both files are **generated** from a single source-of-truth:

```
prometheus-companion/
├── templates/
│   └── rules/
│       ├── agents.md.tmpl        ← the source for AGENTS.md
│       └── claude.md.tmpl        ← the source for CLAUDE.md
├── AGENTS.md                     ← GENERATED — DO NOT EDIT
└── CLAUDE.md                     ← GENERATED — DO NOT EDIT
```

`scripts/gen-rule-files.sh` reads the templates and writes
`AGENTS.md` / `CLAUDE.md` with a `<!-- GENERATED — DO NOT EDIT
— source: templates/rules/agents.md.tmpl -->` banner at the
top. The CI fails if a hand-edit appears in either output file
(`scripts/audit-generated.sh`).

This is the same pattern as the HMA design-tokens
("generated, not kept in sync") — drift is impossible by
construction.

### 26.5 What is **not** in `AGENTS.md`

To keep the file resident and high-signal, the following are
intentionally not in `AGENTS.md`:

- Long form documentation (lives in `docs/`)
- The full skill index (lives in the marketplace)
- The architecture (lives in `docs/00-architecture-and-implementation-plan.md`)
- The dependency table (lives in `versions.toml`)
- The release notes (live in `CHANGELOG.md`)

`AGENTS.md` is the **rule book**, not the **manual**. The manual
is the doc you're reading right now.

### Skills used
- `prometheus-skill-pack/AGENT_BASE_RULES.md` (the canonical
  Constitution)
- `prometheus-skill-pack/AGENTS.md` (Local-Only Validation +
  Progress Signaling)
- `clean-architecture/SKILL.md` (the CLEAN layer rule)
- `tauri-custom-titlebar/SKILL.md` (the title bar rule)
- `a11y-gate/SKILL.md` (the accessibility rule)
- `tauri-ui-review/SKILL.md` (the UI review rule)
- `mobile-navigation/SKILL.md` (the mobile nav rule)
- `pem-local-first/SKILL.md` (the entity management rule)
- `sync-doctrine/SKILL.md` (the sync rule)
- `deploy-hybrid-agentic-stack/SKILL.md` (the secrets rule)
- `dependency-pin-discipline/SKILL.md` (the dep rule)

The Companion's Rust crate lives in
`/Users/gqadonis/Projects/prometheus/prometheus-companion/src-tauri/`
and depends on the new `prometheus-substrate` library crate that
will live in the prometheus-skill-pack superworkspace.

### A.1 The Companion crate

```toml
# src-tauri/Cargo.toml (target)
[package]
name = "prometheus-companion"
version = "0.1.0"
description = "Operator's tray-resident dashboard for the Prometheus skill pack"
authors = ["Travis James <tjames@prometheusags.ai>"]
edition = "2021"
rust-version = "1.94"
license = "MIT"

[lib]
name = "prometheus_companion_lib"
crate-type = ["staticlib", "cdylib", "rlib"]

[[bin]]
name = "prometheus-companion"
path = "src/main.rs"

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
# Tauri
tauri = { version = "2", features = ["tray-icon", "image-png"] }
tauri-plugin-opener = "2"
tauri-plugin-positioner = { version = "2", features = ["tray-icon"] }
tauri-plugin-store = "2"
tauri-plugin-stronghold = "2"
tauri-plugin-notification = "2"          # desktop only; mobile uses flutter_local_notifications
tauri-plugin-os = "2"
tauri-plugin-updater = "2"
tauri-plugin-window-state = "2"
# Biometric, secure-storage, haptics: NOT here. Mobile is Flutter; those
# capabilities live in apps/mobile/pubspec.yaml as local_auth,
# flutter_secure_storage, and flutter/services.dart HapticFeedback.
# See §19.5 for the Flutter package list.

# Substrate (this repository's own workspace member)
prometheus-substrate = { path = "../crates/prometheus-substrate", features = ["desktop"] }

# Async / runtime
tokio = { version = "1", features = ["full"] }
async-trait = "0.1"
futures = "0.3"

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"

# Errors
thiserror = "2"
anyhow = "1"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }

# Time
chrono = { version = "0.4", features = ["serde"] }

# Sync
parking_lot = "0.12"

# Crypto
ed25519-dalek = { version = "2", features = ["rand_core"] }
rand_core = { version = "0.6", features = ["getrandom"] }
sha2 = "0.10"

# HTTP (for substrate's client)
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }

# Mobile parity
[target.'cfg(target_os = "ios")'.dependencies]
# (none additional)

[target.'cfg(target_os = "android")'.dependencies]
# (none additional)

[dev-dependencies]
assert_cmd = "2"
predicates = "3"
cucumber = "0.21"
tokio-test = "0.4"

[profile.release]
codegen-units = 1
lto = "thin"
opt-level = 3
panic = "abort"
strip = true
```

### A.2 The substrate crate (new)

```toml
# crates/prometheus-substrate/Cargo.toml (target)
[package]
name = "prometheus-substrate"
version = "0.1.0"
description = "The Prometheus substrate: in-process supervisor, exec, knowledge, forge, liter-llm, sovereign-sync, A2UI, AG-UI, skill-index"
authors = ["Travis James <tjames@prometheusags.ai>"]
edition = "2021"
rust-version = "1.94"
license = "MIT"

[features]
default = ["desktop"]
# In-process: only what this repository actually links.
client      = []
skills      = []                       # skill-index is a git-rev dep, always on
sovereign   = ["dep:sovereign-sync", "dep:sovereign-client", "dep:iroh-docs-adapter"]
mobile-sync = ["dep:kbd-mobile"]
kpi-open    = []
# Adopted external services (change-cpc-007) are supervised, not linked, so they
# are not cargo features: executor, knowledge, forge, surface, literllm, exec,
# research.

# Surface presets
desktop  = ["client", "skills", "sovereign", "kpi-open"]
mobile   = ["client", "skills", "sovereign", "mobile-sync"]
headless = ["client", "skills", "sovereign"]   # no window, no Stronghold vault

[dependencies]
# Crates that MOVE into this repository (change-cpc-004): local workspace members.
sovereign-sync    = { path = "../sovereign-sync", optional = true }
sovereign-client  = { path = "../sovereign-client", optional = true }
kbd-mobile        = { path = "../kbd-mobile", optional = true }
iroh-docs-adapter = { path = "../iroh-docs-adapter", optional = true }

# Crates that STAY in the open-source pack: consumed by git rev, never by path.
# The pack never depends on the Companion (operator decision D-02, option D), so
# this direction is one-way. A `path` dependency into the pack, or a committed
# [patch] pointing at a local checkout, is a defect: it makes a Companion build
# depend on a working copy that no other machine has.
kbd-runtime      = { git = "https://github.com/Prometheus-AGS/prometheus-skill-system", rev = "<pack commit containing change-cpc-002>" }
storage-provider = { git = "https://github.com/Prometheus-AGS/prometheus-skill-system", rev = "<same rev>", default-features = false }
learner-model    = { git = "https://github.com/Prometheus-AGS/prometheus-skill-system", rev = "<same rev>" }
skill-index      = { git = "https://github.com/Prometheus-AGS/prometheus-skill-system", rev = "<same rev>" }   # always on

# Services that STAY in the pack and are adopted as external processes, not
# linked (change-cpc-007): prometheus-exec, prometheus-knowledge, forge-rs,
# liter-llm, surface-bridge, prometheus-research, surrealdb. The supervisor
# reads the pack's generated `shared/services.manifest.json` and manages them
# where they already run. They are deliberately NOT dependencies here.

# Inherited transitively from kbd-runtime; pinned in versions.toml.
# keyring =3.6.3 holds the device key as credential-store item
# `prometheus-kbd-device`. Do not add a second major version of keyring.

# Async / runtime
tokio = { version = "1", features = ["full"] }
async-trait = "0.1"
futures = "0.3"

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"
rmp-serde = "1"

# Errors
thiserror = "2"
anyhow = "1"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }

# Time
chrono = { version = "0.4", features = ["serde"] }

# Sync
parking_lot = "0.12"

# HTTP
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }

# Crypto (for sovereign-sync identity)
ed25519-dalek = { version = "2", features = ["rand_core"] }
rand_core = { version = "0.6", features = ["getrandom"] }
sha2 = "0.10"
blake3 = "1"

# P2P (Iroh)
iroh = { version = "1.0.3", optional = true }  # gated by sovereign; 1.0.2 is the security floor (relay DoS)

# MCP
rmcp = { version = "1.8", features = ["server", "transport-io", "transport-sse", "macros"] }

[dev-dependencies]
tempfile = "3"
tokio-test = "0.4"

[profile.release]
codegen-units = 1
lto = "thin"
opt-level = 3
panic = "abort"
strip = true
```

---

## Appendix B — package.json additions and dependency plan

The current scaffold is the bare `pnpm create tauri-app` output.
The full dependency plan is below.

```jsonc
// package.json (target)
{
  "name": "prometheus-companion",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "preview": "vite preview",
    "tauri": "tauri",
    "tauri:dev": "tauri dev",
    "tauri:build": "tauri build",
    "test": "vitest",
    "test:e2e": "playwright test",
    "test:bdd": "cargo nextest run -p prometheus-companion --features bdd",
    "lint": "eslint . --ext .ts,.tsx",
    "format": "prettier --write .",
    "typecheck": "tsc --noEmit",
    "audit:layer": "bash scripts/audit-layer.sh",
    "audit:a11y": "bash scripts/audit-a11y.sh",
    "audit:ui": "bash scripts/audit-ui.sh",
    "gen:tokens": "bash scripts/gen-design-tokens.sh .",
    "lint:skills": "bash scripts/lint-skills.sh"
  },
  "dependencies": {
    "react": "^19.1.0",
    "react-dom": "^19.1.0",

    // Tauri
    "@tauri-apps/api": "^2",
    "@tauri-apps/plugin-opener": "^2",
    "@tauri-apps/plugin-positioner": "^2",
    "@tauri-apps/plugin-store": "^2",
    "@tauri-apps/plugin-os": "^2",
    "@tauri-apps/plugin-notification": "^2",
    "@tauri-apps/plugin-updater": "^2",
    "@tauri-apps/plugin-window-state": "^2",
    "@tauri-apps/plugin-stronghold": "^2"
    // Biometric, secure-storage, haptics: NOT here. Mobile is Flutter;
    // those capabilities live in apps/mobile/pubspec.yaml as local_auth,
    // flutter_secure_storage, and flutter/services.dart HapticFeedback.
    // See §19.5 for the Flutter package list.

    // Routing & state
    "@tanstack/react-router": "^1",
    "zustand": "^5",

    // Entity management
    "@prometheus-ags/prometheus-entity-management": "^3",
    "pglite": "^0.2",
    "@electric-sql/pglite": "^0.2",

    // UI (shadcn/ui with Base UI)
    "@base-ui-components/react": "^1.6",
    "class-variance-authority": "^0.7",
    "clsx": "^2",
    "tailwind-merge": "^2",
    "lucide-react": "^0.5",

    // Chat (Assistant-UI)
    "@assistant-ui/react": "^0.10",
    "@assistant-ui/react-markdown": "^0.10",

    // Forms
    "react-hook-form": "^7",
    "zod": "^3",
    "@hookform/resolvers": "^3",

    // QR codes (for P2P pairing)
    "qrcode": "^1.5",

    // Utilities
    "date-fns": "^4",
    "nanoid": "^5"
  },
  "devDependencies": {
    "@types/react": "^19.1.8",
    "@types/react-dom": "^19.1.6",
    "@types/qrcode": "^1.5",
    "@vitejs/plugin-react": "^6.0.2",
    "typescript": "~6.0.3",
    "vite": "^8.0.16",
    "@tauri-apps/cli": "^2",

    // shadcn
    "shadcn": "^3",

    // Tailwind 4
    "tailwindcss": "^4",
    "@tailwindcss/vite": "^4",

    // Testing
    "vitest": "^2",
    "@playwright/test": "^1.5",
    "axe-playwright": "^2",

    // Lint / format
    "eslint": "^9",
    "@typescript-eslint/parser": "^8",
    "@typescript-eslint/eslint-plugin": "^8",
    "prettier": "^3"
  }
}
```

### B.1 Notable version pins

- **React 19.1** — the same major as the rest of the PMP stack
- **Tauri 2** — the only supported Tauri major; Tauri 1 is legacy
- **Vite 8** — matches the rest of the PMP stack
- **TypeScript 6.0.3** — same as the rest of the PMP stack
- **PGlite 0.2** — the local store
- **Base UI 1.6** — the new shadcn default
- **shadcn 3** — the CLI version
- **Zustand 5** — same as the rest of the PMP stack

### B.2 What is NOT in the dependencies

- **`@tanstack/react-query`** — explicitly excluded by
  `pem-local-first`
- **`radix-ui`** (the npm package) — replaced by `@base-ui-components/react`
- **`electron`** — we chose Tauri
- **`@capacitor/*`** — mobile is Flutter + Rust over FFI, not Capacitor
- **`@tauri-apps/api` on mobile** — the mobile shell is Flutter, not Tauri; the desktop app uses Tauri, the mobile app uses `flutter_rust_bridge` bindings to the same Rust substrate
- **`libp2p`** directly — the substrate's sovereign-sync uses Iroh
- **`swr`** — same reason as TanStack Query

---

## Appendix C — File/folder layout (target tree)

```
prometheus-companion/
├── docs/
│   ├── 00-architecture-and-implementation-plan.md   (this file)
│   ├── 01-titlebar-implementation.md
│   ├── 02-tray-and-popover.md
│   ├── 03-dashboard-pages.md
│   ├── 04-a2ui-integration.md
│   ├── 05-p2p-pairing.md
│   ├── 06-mobile-target.md
│   ├── 07-release-runbook.md
│   └── adr/
│       ├── 0001-tauri-over-electron.md
│       ├── 0002-base-ui-over-radix.md
│       ├── 0003-substrate-in-process.md
│       ├── 0004-iroh-over-libp2p.md
│       └── 0005-pem-over-tanstack-query.md
├── AGENTS.md                                        (project-level rules)
├── CHANGELOG.md
├── LICENSE
├── README.md
├── index.html
├── package.json
├── pnpm-lock.yaml
├── tsconfig.json
├── tsconfig.node.json
├── vite.config.ts
├── tailwind.config.ts
├── postcss.config.js
├── .editorconfig
├── .prettierrc.json
├── .gitignore
├── .nvmrc
│
├── assets/
│   ├── icons/                    (shadcn + brand icons)
│   ├── tray/                     (5 state PNGs × 6 sizes)
│   │   ├── green/
│   │   ├── yellow/
│   │   ├── red/
│   │   ├── gray/
│   │   └── spinner/
│   └── templates/
│       └── design-tokens/
│           └── tokens.toml       (the single source of truth)
│
├── public/
│   ├── favicon.svg
│   ├── pwa-manifest.json
│   └── robots.txt
│
├── scripts/
│   ├── gen-design-tokens.sh      (TOML → theme.css + theme.rs)
│   ├── gen-tray-icons.sh         (SVG → 5 state PNGs)
│   ├── audit-layer.sh            (the entity-graph-web-shell invariant)
│   ├── audit-a11y.sh             (the a11y-gate checklist)
│   ├── audit-ui.sh               (the tauri-ui-review loop)
│   ├── lint-skills.sh
│   └── verify-changelog.sh
│
├── src/                          (see §7.1 for the full tree)
│
├── src-tauri/
│   ├── Cargo.toml                (see Appendix A.1)
│   ├── tauri.conf.json           (see §5)
│   ├── build.rs
│   ├── entitlements.plist        (macOS)
│   ├── icons/
│   │   ├── 32x32.png
│   │   ├── 128x128.png
│   │   ├── 128x128@2x.png
│   │   ├── icon.icns
│   │   └── icon.ico
│   ├── binaries/                 (Tauri externalBin)
│   ├── capabilities/
│   │   └── default.json
│   └── src/
│       ├── main.rs
│       ├── lib.rs
│       ├── tray.rs
│       ├── titlebar_commands.rs  (any commands the titlebar needs)
│       ├── health.rs
│       ├── secrets.rs
│       ├── pairing.rs
│       └── theme.rs              (generated from tokens.toml)
│
├── features/                     (the BDD features)
│   ├── services/
│   │   └── restart.feature
│   ├── devices/
│   │   └── pair.feature
│   ├── chat/
│   │   └── streaming.feature
│   ├── skills/
│   │   └── invoke.feature
│   └── runs/
│       └── verify.feature
│
└── tests/                        (the BDD step definitions)
    ├── bdd.rs
    ├── steps/
    │   ├── services.rs
    │   ├── devices.rs
    │   ├── chat.rs
    │   ├── skills.rs
    │   └── runs.rs
    └── fixtures/
```

---

## Appendix D — Implementation sequence with verifiable checkpoints

Each step ends with a one-line verification. The verification is
what makes the step "done" (not "PR opened" or "code written").

| Step | Output | Verification |
|---|---|---|
| 1.1 | `prometheus-skill-pack/Cargo.toml` superworkspace members | `cargo metadata --format-version 1 --no-deps` lists every member |
| 1.2 | `~/.cargo/config.toml` with sccache + mold | `cargo build` is at least 30% faster than baseline |
| 1.3 | `assets/templates/design-tokens/tokens.toml` | `bash scripts/gen-design-tokens.sh .` writes `src/theme.css` |
| 1.4 | `components.json` with `base: "base-ui"` | `npx shadcn@latest info` shows Base UI |
| 1.5 | All 18 shadcn components in `src/components/ui/` | `npx tauri dev` opens; each component renders |
| 2.1 | `tauri.conf.json` with `decorations: false` | `pnpm tauri dev` shows a frameless window |
| 2.2 | `Titlebar.tsx` with platform-correct controls | Hand-test on macOS, Windows, Linux |
| 2.3 | Drag works on every pixel of the title bar's empty area | Hand-test; no dead zones |
| 2.4 | `a11y-gate` passes | `pnpm audit:a11y` exits 0 |
| 2.5 | `tauri-ui-review` passes at 8 shots | `pnpm audit:ui` exits 0 |
| 2.6 | Web build does not crash | `pnpm build` exits 0; the page loads in a browser |
| 3.1 | Tray icon renders, color follows state | Hand-test on macOS, Windows, Linux |
| 3.2 | Popover opens on left-click, closes on Esc / blur | Hand-test |
| 3.3 | `prometheus-substrate` compiles | `cargo check -p prometheus-substrate --features desktop` |
| 3.4 | Supervisor starts the in-process services | `pnpm tauri dev`; see the green tray icon after 5s |
| 3.5 | Service restart updates the tray icon | Click "Restart" in the popover; see the icon go yellow → green |
| 4.1 | PGlite boots; schema migrates | `pnpm tauri dev`; no errors in the dev console |
| 4.2 | PEM entities registered; transports work | `useServices` returns the 9 services |
| 4.3 | Dashboard pages render with live data | Hand-test; restart a service, see the card update |
| 4.4 | `audit:layer` passes (no `invoke` / `listen` in components) | `pnpm audit:layer` exits 0 |
| 5.1 | A2UI registry has all the component types | `pnpm test` exits 0 |
| 5.2 | Chat streams AG-UI events from a skill | Hand-test; run a skill, see the tokens stream |
| 5.3 | A2UI surfaces render and route actions back | Hand-test; click an A2UI button, see the action in UAR |
| 6.1 | Device identity persists across restarts | Generate identity, restart, see the same node id |
| 6.2 | Pair a second device; see it in the Devices page | Hand-test with two devices |
| 6.3 | Paired device can read the substrate's health | Hand-test; the mobile shows the same dashboard |
| 7.1 | Provider registry lists all known providers | Open Settings → Providers; see the list |
| 7.2 | API key stored in OS secure store | Configure a provider; see the "Configured ✓" badge |
| 7.3 | Test connection works | Click "Test connection" on a configured provider; see the result |
| 8.1 | Full `audit:layer` passes on all surfaces | `pnpm audit:layer` exits 0 |
| 8.2 | Full `audit:naming` passes on all surfaces | `pnpm audit:naming` exits 0 |
| 8.3 | Full `audit:a11y` passes on all surfaces | `pnpm audit:a11y` exits 0 |
| 8.4 | Full `audit:ui` passes (8 shots × N surfaces) | `pnpm audit:ui` exits 0 |
| 8.5 | Full BDD suite passes | `pnpm test:bdd` exits 0 |
| 8.6 | `hybrid-runtime-verification` passes | `bash scripts/runtime-verify.sh` exits 0 |
| 8.7 | macOS DMG is code-signed and notarized | `codesign -dv` and `xcrun notarytool history` |
| 8.8 | Windows MSI is Authenticode-signed | `signtool verify` exits 0 |
| 8.9 | Auto-updater signing key is in CI | `gh secret list` includes the key |
| 8.10 | 0.1.0 release cut | `git tag v0.1.0` is pushed; GitHub Release exists |
| 9.1 | iOS target builds | `pnpm tauri ios build` exits 0 |
| 9.2 | Android target builds | `pnpm tauri android build` exits 0 |
| 9.3 | Mobile feature split on substrate | `cargo check -p prometheus-substrate --no-default-features --features mobile` exits 0 |
| 9.4 | TestFlight internal build installs and runs | Hand-test on a physical iPhone |
| 9.5 | Mobile pair flow works | Hand-test; phone pairs with desktop |
| 10.1 | Mobile `audit:a11y` passes | `pnpm audit:a11y` exits 0 |
| 10.2 | Mobile BDD suite passes | `pnpm test:bdd --target mobile` exits 0 |
| 10.3 | 1.0 release cut | `git tag v1.0.0` is pushed; App Store submission is in flight |

---

*This document is the v0.1.0 implementation plan for the
Prometheus Companion. It is grounded in the source-of-truth
skills at `/Users/gqadonis/Projects/hybrid-mobile-architecture-src/`
and `/Users/gqadonis/Projects/prometheus/prometheus-skill-pack/skills/`,
and it implements the Pillar 5 + Pillar 6 work in the architecture
review at `prometheus-skill-pack/docs/audits/2026-08-20-skill-pack-architecture-review.md`.
Every section ends with a "Skills used" callout so the reader can
audit the grounding. The implementation is sequenced as 10
phases, each with verifiable checkpoints in Appendix D.*

*Generated 2026-08-20 by Mavis (`MiniMax-M3`). The next time this
document is updated, file the diff alongside the previous version
under `docs/architecture-history/`.*
