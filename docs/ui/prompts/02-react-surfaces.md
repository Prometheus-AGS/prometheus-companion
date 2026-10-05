# Prompt 02 · React Surfaces

> Phase 4. Requires Gates 1, 2, 3 closed. Read `docs/ui/11-react-surfaces-spec.md`.

---

Invoke before writing code: `reference-ui-fidelity`, `mobile-navigation`,
`hybrid-design-tokens`, `a11y-gate`, `pem-local-first`, `tauri-ui-review`.

Build the assistant slideout and the settings surfaces.

**The reference is authoritative.** `Companion Concept B — Operations Board` is a visual
acceptance oracle, not inspiration. Start with `reference-ui-fidelity`: produce the complete
screen and state inventory from it, show me the inventory, and only then implement.

## The slideout

480 px, right-docked, **pushes** the main content. It does not overlay above 1100 px total
width, and the consequences are yours to implement, not to discover:

- The board reflows — attention queue 3 → 2 columns, source board 4 → 3.
- With both open, the inspector drawer is width-capped as a percentage of the remaining
  area, not a fixed 640 px.
- Below 1100 px it becomes an overlay with a scrim. This is a **width** decision. If you
  write a `Platform.isIOS` or `navigator.userAgent` check in navigation or layout code, that
  check is wrong — see `mobile-navigation`.
- Width is a persisted preference, 400–620.

Anatomy: header with run-state dot, thread id, close; a model chip that opens
Settings → Models; the thread; a composer with suggestion chips and one line stating the
boundary — the thread enumerates the same registry the board does and cannot author an
effect.

Scroll: pinned to bottom while the operator is at the bottom, releases on manual scroll up
with a jump-to-latest affordance. **Never call `scrollIntoView`** — set `scrollTop`.

## The settings surfaces

Six sections in a 212 px sub-nav, per the table in the spec: Providers, Models, Adversarial
review, Sources, Capability grants, Pairing.

Generate each section from data using the panel kinds the spec lists (`table`, `kv`, `pick`,
`toggles`, `trace`, `warn`, `note`). Adding a seventh section must need no new panel kind —
if it does, the panel model is wrong.

For every setting, show the **resolved** value and the reason. "Critic: kimi-k3" is a
setting; "Critic: kimi-k3 — clears the exclusion set for harness=claude" is a setting an
operator can reason about.

## State

`pem-local-first` governs this layer. TanStack Query, react-query, SWR, and Apollo cache are
prohibited. Components read through hooks and never call `invoke()` or `listen()`.

One source of truth for service state. The thread projects; it does not keep a parallel
service list. Approving a plan in the thread must visibly change the board — the row turns,
the header count drops, the queue card leaves. If it does not, the shared-registry claim is
decoration and the gate stays open.

## Before you call it done

Screenshots at 320 / 768 / 1024 / 1440 in both themes, including the overlay crossover at
1100 px. Measure that streaming text causes no layout shift — measure it, do not eyeball it.
Audit contrast on dark surfaces specifically: a muted token that passes in light mode can
land at 2.6:1 on a dark panel.
