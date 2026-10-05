# A2UI Component Specification

> Backing skills: `a2ui-surface-contract`, `content-block-ui`, `hybrid-design-tokens`,
> `a11y-gate`. Read all four before writing a component.

A2UI describes a projection. It does not grant authority. The agent composes; the client
renders what it already trusts.

---

## ADR-UI-1 · Catalog components ride inside `artifact`

**Context.** `ContentBlock` is a frozen 11-variant union and the single cross-platform
contract between `gen_ui_core` and every surface. Adding a variant touches Rust, Dart, and
TypeScript, requires codegen on both surfaces, and is deliberately expensive. But the
agentic surface needs an *open* set of rich components, and it needs to grow without a
release.

**Decision.** The catalog is open; the union stays frozen. A catalog component arrives as
an `artifact` block whose `artifactType` is the catalog component id and whose `payload`
is the component's props:

```ts
{ type: "artifact", artifactType: "FixPlan", artifactVersion: 1, payload: { … } }
```

The `artifact` renderer resolves `artifactType` through the catalog registry. Adding a
component is a registry entry plus a React component plus a schema — no Rust union change,
no Dart codegen, no mirror edit.

**Consequences.**
- Never add a 12th `ContentBlock` variant for a catalog component. If you believe you need
  one, you have misread this ADR.
- The exhaustiveness rule still holds at the `ContentBlock` switch. The registry lookup is
  a second, separate dispatch with its own unknown path.
- Flutter gets the same components for free by registering its own catalog against the
  same ids. Parity is a test, not a hope.

---

## The catalog

Twelve components in two groups. Ids are stable and versioned; a prop change that is not
backward compatible bumps `artifactVersion`.

### Group A · Projection components (8)

The agent selects these to *show* the operator something it found.

| id | Props (required) | Renders | Interaction |
|---|---|---|---|
| `ServiceRow` | `services: ServiceRef[]` | Agent-selected live status rows | Row click opens that service in the operator view |
| `LogTail` | `path`, `lines: LogLine[]`, `filter` | A filtered log excerpt with the filter stated | None; the full tail stays in the operator view |
| `MetricSpark` | `series: number[]`, `axisLeft`, `axisRight` | Small bar timeseries | None |
| `TableView` | `columns: Column[]`, `rows: Cell[][]` | Arbitrary tabular result | Optional row action by capability id |
| `ChoiceCards` | `choices: Choice[]`, `selected?` | Pick-one among agent options | Emits `surface.action` with the chosen id |
| `FixPlan` | `fixes: RankedFix[]`, `gate` | Ranked fixes with rank, confidence, blast radius | Approve emits `approval.decided`; nothing runs before it |
| `ConfirmForm` | `fields: Field[]`, `submitLabel` | Agent-composed input request | Submit emits `resolution.answered` or `approval.decided` |
| `ReceiptBlock` | `lines: ReceiptLine[]`, `verified: boolean` | Signed execution receipt | Click-through to the run record |

`ServiceRef` carries the service **id only**. The component resolves current status from
the store — it never renders a status the agent asserted, because that status is stale the
moment it was serialized.

### Group B · Resolution components (4)

The agent, or rather the resolver, uses these when a prompt did not become one task. See
`30-task-resolution-spec.md` for when each is emitted.

| id | Props (required) | Emitted for verdict | Answer event |
|---|---|---|---|
| `CandidateChoice` | `resolutionId`, `candidates: Candidate[]`, `prompt` | `ambiguous` | `resolution.answered` with `candidateId` |
| `ParamForm` | `resolutionId`, `capability`, `fields: Field[]` | `underspecified` | `resolution.answered` with `params` |
| `ClarifyPrompt` | `resolutionId`, `restated`, `nearest: Candidate[]` | `low_confidence` | `resolution.answered` with `text` or `candidateId` |
| `ResolutionTrace` | `resolutionId`, `steps: TraceStep[]`, `thresholds` | any, on request | None; inspectable only |

`CandidateChoice` shows, per candidate: capability id, one-line description, the effect
class and gate, the confidence, and **what evidence matched** — the trigger phrase, the
glossary synonym, or the retrieval span. A candidate list without the matched evidence is
a guess wearing a number.

`ClarifyPrompt` must not assert a candidate. It restates what the resolver understood and
offers the nearest three as unranked suggestions. Presenting a low-confidence top hit as
the answer is the failure mode this component exists to prevent.

`ResolutionTrace` renders the thresholds in force alongside the scores. An operator who
disagrees with a verdict needs to see the constant that produced it.

---

## Component rules

1. **Registered types only.** The renderer resolves `artifactType` through the registry. An
   unregistered id renders `UnknownSurface`: the id, the raw props in a disclosure, and a
   note that the run is neither failed nor complete. Never guess a component from a shape.
2. **Validated props, always.** Each entry carries a validator generated from its schema.
   Validation failure renders `MalformedSurface` with the validation errors, and emits a
   `surface.malformed` telemetry event. It never throws into the tree and never silently
   drops the block.
3. **No authority in the component.** No `invoke()`, no `fetch`, no `listen`, no provider
   routing, no policy decision. Actions emit an event upward; the gateway decides.
4. **Every action carries `runId`, `eventId`, and an idempotency key.** A double click
   produces one effect. This is the component's job as much as the gateway's.
5. **All six action states are renderable.** Pending approval, denied, cancelled, failed,
   resumed, and completed. A component that can only draw the happy path is unfinished.
6. **Sanitize.** Markdown is sanitized; external URLs are validated and open through the
   shell's own opener, never a raw `href` to an agent-supplied target.
7. **Streaming variants never jump.** `text` and `thinking` grow in place with a caret.
   Animate opacity and transform only.
8. **Tokens, not values.** No hex, no px literal that is not a token reference. One token
   source feeds the Tailwind theme and the Flutter theme.
9. **`image` needs explicit `width`/`height` and a mandatory `alt`.**
10. **Provenance is visible.** Every catalog surface renders its id and catalog version in
    a header strip. The operator should always be able to see that a block was composed,
    not authored, and which version composed it.

## Accessibility per component

Beyond the `a11y-gate` checklist, these are the component-specific requirements that get
missed:

- `CandidateChoice` and `ChoiceCards` are a radiogroup with a labelled group name, arrow-key
  traversal, and an accessible name per option that includes the effect class.
- `FixPlan`'s approve control is disabled with `aria-disabled` and a reason, never removed,
  when policy blocks it.
- Streaming text announces through a polite live region on completion, not per token.
- `ReceiptBlock` is a definition list, not a table, and the verified state is text as well
  as colour.
- `MetricSpark` needs a text summary — the peak and the window — because a bar row is
  invisible to a screen reader.

## Parity

The React and Flutter catalogs render equivalent semantics for the same props. Equivalent
does not mean identical pixels; it means the same information, the same states, the same
affordances, and the same accessible names. This is verified by a shared fixture set: each
component has a props fixture in `reference/`, and both surfaces render it in a
screenshot/golden test.
