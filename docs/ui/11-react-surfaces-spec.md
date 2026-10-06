# React Surfaces Specification

> React 19.2.7 · Zustand 5.0.14 · Vite 8.1.5 · TypeScript 7.0.2 (pins from `versions.toml`).
> Visual oracle: **Companion Concept B — Operations Board**. Backing skills:
> `reference-ui-fidelity`, `tauri-ui-review`, `mobile-navigation`, `hybrid-design-tokens`,
> `a11y-gate`, `pem-local-first`, `entity-graph-web-shell`.

Treat Concept B as an acceptance oracle, not an inspiration board. Build the screen and
state inventory from it first (`reference-ui-fidelity`), then implement.

---

## File layout

```
src/
  bridge/a2ui/
    types.ts              # GENERATED from Rust. Never hand-edited.
    catalog.ts            # component id → { component, validate, version }
    renderer.tsx          # artifact dispatch + UnknownSurface / MalformedSurface
  stores/
    runStore.ts           # AG-UI projection: events → blocks, per runId
    resolutionStore.ts    # pending resolutions by resolutionId
    serviceStore.ts       # PEM-owned service graph (NOT a thread-local copy)
    settingsStore.ts      # providers, models, review policy, grants
  hooks/
    useRun.ts             # subscribe to a run projection
    useResolution.ts      # open resolution + answer()
    useCapabilities.ts    # the manifest, for both the board and the composer
  surfaces/
    assistant/            # the slideout
    settings/             # the six sections
    board/                # queue + source columns + drawer
  components/a2ui/        # the 12 catalog components, one file each
```

**Prohibited:** TanStack Query, react-query, SWR, Apollo cache. Prometheus Entity
Management 3.x owns this layer (`pem-local-first`). Components read through hooks and never
call `invoke()` or `listen()` themselves.

---

## Surface 1 · Assistant slideout

**Geometry.** 480 px, right-docked, **pushes** the main content rather than overlaying it.
The operator's reason for opening the assistant is usually something they can see on the
board; covering it defeats the purpose. Consequences to implement, not to discover:

- The board reflows: the attention queue goes 3 → 2 columns, the source board 4 → 3.
- The inspector drawer, when both are open, is width-capped as a percentage of the
  remaining area, not a fixed 640 px.
- Below 1100 px total width the slideout becomes an overlay with a scrim — pushing would
  leave the board unreadable. This is a **width** decision, never a platform check
  (`mobile-navigation`).
- The width is a persisted user preference in the 400–620 range.

**Anatomy, top to bottom.** Header: run-state dot, thread id, close. Model chip showing the
resolved chat model, clicking through to Settings → Models — a model the operator cannot
locate is a model they cannot trust. Thread: the block list. Composer: suggestion chips, the
input, and one line stating the boundary — the thread enumerates the same registry the board
does; it cannot author an effect.

**Thread rendering.** One `ContentBlock` switch with a `never` guard. `artifact` blocks
dispatch through the catalog registry. Streaming blocks grow in place. Every catalog surface
shows its provenance strip.

**Scroll behaviour.** Pinned to the bottom while the operator is at the bottom; releases the
moment they scroll up, with a "jump to latest" affordance and an unread count. Never call
`scrollIntoView`; set `scrollTop` directly.

## Surface 2 · Settings

Six sections in a 212 px sub-nav. Each is generated from data, not hand-laid — the panel
kinds are `table`, `kv`, `pick`, `toggles`, `trace`, `warn`, `note`, and a new section
should need no new panel kind.

| Section | Content | Notes |
|---|---|---|
| Providers | The liter-llm provider registry: base URL, keychain reference, model count, p95, health | Keys live in the OS secure store; the app holds a reference and asks the store to sign. An unhealthy provider leaves every routing pool, including the review roles, and says why |
| Models | The model registry, routing defaults, fallback chain, degraded-mode model | Price per model is recorded because the review policy holds a cost ceiling. A model whose provider goes unhealthy stays listed but is never silently substituted mid-run |
| Adversarial review | Harness, resolved producer/critic/judge, resolution trace, model pickers, policy toggles, limits | See `40-adversarial-review-config.md` |
| Sources | Registered manifest directories | A source is any directory with a `companion.manifest.toml`; registering one adds a board column with no release |
| Capability grants | The manifest as a table: capability, effect class, gate, callers | This is the one registry all four callers enumerate. Read-only here; edits go through policy |
| Pairing | This node's identity, paired peers, per-peer grants | Grants are chosen before the code is shown, never after |

**Rule for every settings surface:** show the *resolved* value and the reason. "Critic:
kimi-k3" is a setting. "Critic: kimi-k3 — clears the exclusion set for harness=claude" is a
setting an operator can reason about.

## Surface 3 · Board integration

The board already exists. The assistant must integrate with it, not shadow it.

- One source of truth for service state: `serviceStore`. The thread's `ServiceRow`
  resolves status from the store by id.
- Approving a plan in the thread changes the board — the row turns, the header count drops,
  the queue card leaves. If approving in the thread does not visibly change the board, the
  shared-registry claim is decoration.
- "Ask about this" in the drawer opens the assistant seeded with that service's context and
  closes the drawer. The operator asked to change modes, not to stack two panels.

---

## State rules

1. **Events in, projection out.** `runStore` is a reducer over AG-UI events. No component
   mutates a block.
2. **Idempotent reducers.** Applying the same event twice is a no-op. Test it directly.
3. **Persist the projection**, keyed by runId, so `uar_recover` can rehydrate after a
   restart and open resolutions come back.
4. **No derived state in components.** Compute in a selector; components receive values.
5. **One open resolution per run**, enforced in the store, not in the UI.

## Verification

`tauri-ui-review` after every surface change: screenshots at 320 / 768 / 1024 / 1440 in
both themes. `a11y-gate` on every UI edit. Both are gates, not suggestions — a surface
without screenshot evidence is not done, and compiling is not evidence
(`hybrid-runtime-verification`).

Specific things to check that a generic pass misses:

- The slideout push at each breakpoint, including the overlay crossover at 1100 px.
- Streaming text with no layout jump — measure, do not eyeball.
- Every catalog component in each of its six action states.
- Colour contrast on **dark** surfaces specifically. A muted token that is correct in light
  mode can fail at 2.6:1 on a dark panel; that failure has already happened here twice, and
  both times it was hiding in a shared value map rather than in the markup.
