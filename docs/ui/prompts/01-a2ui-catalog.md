# Prompt 01 · The A2UI Catalog

> Phase 1. Requires Gate 0 closed. Read `docs/ui/10-a2ui-component-spec.md` first.

---

Invoke these skills before writing code: `content-block-ui`, `a2ui-surface-contract`,
`hybrid-design-tokens`, `a11y-gate`, `reference-ui-fidelity`.

Build the client-owned A2UI component catalog for the React surface.

**Read `docs/ui/10-a2ui-component-spec.md` in full, including ADR-UI-1.** The decision it
records is not negotiable: the `ContentBlock` union stays frozen at 11 variants, and catalog
components arrive as `artifact` blocks whose `artifactType` is the component id. If you find
yourself wanting a 12th variant, you have misread it — stop and tell me instead.

Deliver:

1. **The registry** — `src/bridge/a2ui/catalog.ts`. Maps component id to
   `{ component, validate, version }`. The renderer resolves through this and nothing else.
   A grep for a switch or an if-chain on `artifactType` outside this file must come back
   empty; that is a CI gate.

2. **Twelve components** in `src/components/a2ui/`, one file each, from the two tables in
   the spec. Eight projection components, four resolution components. Props-in only: no
   `invoke()`, no `fetch`, no `listen`, no store access beyond the hooks the spec names.

3. **The two failure surfaces** — `UnknownSurface` for an unregistered id, `MalformedSurface`
   for prop-validation failure. Both render the raw payload in a disclosure. Neither marks
   the run failed or complete. `MalformedSurface` emits `surface.malformed`; it never throws
   into the tree.

4. **The dispatcher** — the `ContentBlock` switch with a `never`-typed exhaustiveness guard
   in `default`. It switches and delegates; no rendering logic inline.

Constraints I will check:

- Every value comes from the token set. No hex literal, no raw px, in any component.
- Every component renders all six action states: pending approval, denied, cancelled,
  failed, resumed, completed.
- `ServiceRow` takes service **ids** and resolves current status from the store. It must not
  render a status the agent asserted — that status is stale the moment it was serialized.
- `CandidateChoice` shows the matched evidence per candidate (trigger phrase, glossary
  synonym, or retrieval span). A candidate list without evidence is a guess wearing a number.
- `ClarifyPrompt` must not assert a candidate. It restates and offers the nearest three,
  unranked.
- Accessibility: radiogroup semantics with arrow traversal on both choice components;
  `aria-disabled` with a reason rather than removal on a blocked approve; a text summary on
  `MetricSpark`; a definition list, not a table, for `ReceiptBlock`.
- Provenance strip on every catalog surface — component id and catalog version, visible.

Before you call this done: screenshots at 320 / 768 / 1024 / 1440 in both themes for every
component, the a11y checklist, and a prop-rejection test per component. Compiling is not
evidence.

Start by listing the twelve files you will create and the token references you expect to
need. I will confirm before you write them.
