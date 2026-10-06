# AG-UI Agent Specification

> The runtime-to-application boundary, the agent that drives it, and the A2A surface that
> exposes it. Backing skills: `agui-event-contract`, `axum-agent-gateway`,
> `agent-runtime-security`, `persona-scoped-agent`.

AG-UI is the event boundary. UAR emits authoritative run events; clients maintain
projections. A2UI is what some of those events carry. A2A is how another agent asks for a
run. Four protocols, four jobs — do not collapse them.

---

## Envelope

Every event, no exceptions:

```ts
interface AgUiEvent<T = unknown> {
  protocolVersion: string;   // "1.0" — reject an unknown MAJOR, preserve an unknown MINOR
  eventId: string;           // stable across retries; the dedupe key
  runId: string;
  seq: number;               // monotonic per run, gap-detectable
  ts: string;                // RFC 3339, server clock
  type: string;              // see the union below
  payload: T;                // typed per event type
  causalParent?: string;     // eventId of the cause, for tool-call correlation
}
```

`eventId` stable across retries is the whole reliability story. A retried event with a new
id is a duplicate the client cannot detect, and the projection silently doubles.

## Event union

| Type | Payload | Notes |
|---|---|---|
| `run.started` | `{ plan, caller, budgets }` | `caller` is one of `ui \| chat \| a2a \| mobile` |
| `text.delta` | `{ blockId, text }` | Append-only; never a replacement |
| `thinking.delta` | `{ blockId, text }` | Same, collapsible on the surface |
| `tool.call.start` | `{ toolCallId, capability, gate }` | Arguments not yet complete |
| `tool.call.args.delta` | `{ toolCallId, text }` | Partial JSON; render as text, parse only when complete |
| `tool.call.result` | `{ toolCallId, ok, artifact? }` | `artifact` is a catalog surface (ADR-UI-1) |
| `surface.render` | `{ blockId, artifactType, artifactVersion, props }` | Agent-composed A2UI projection |
| `surface.action` | `{ blockId, actionId, payload, idempotencyKey }` | Client → server |
| `resolution.required` | `{ resolutionId, artifactType, props, verdict }` | Task state becomes `input-required` |
| `resolution.answered` | `{ resolutionId, candidateId? , params?, text? }` | Client → server |
| `resolution.stale` | `{ resolutionId, reason }` | Answer rejected; never re-runs |
| `resolution.expired` | `{ resolutionId }` | TTL elapsed |
| `approval.required` | `{ approvalId, plan, gate, diff? }` | Renders the gate, not a toast |
| `approval.decided` | `{ approvalId, decision, reason? }` | Client → server; `reason` required on override |
| `run.cancelled` | `{ by, atSeq, sideEffects }` | `sideEffects` is explicit, never assumed empty |
| `run.failed` | `{ code, message, retryable }` | |
| `run.completed` | `{ receipt }` | The **only** completion signal |

Anything else: preserve it, display it as an unknown event artifact, and do not treat it as
success. Never infer completion from prose — a model saying "all done" is text, not a
`run.completed`.

## Client semantics

1. **Idempotent by `eventId`.** Keep a seen-set per run, bounded and persisted with the
   projection.
2. **Ordered by `seq`.** Buffer a gap for up to `SEQ_GAP_TIMEOUT` (2 s), then request a
   replay from the last committed seq rather than applying out of order.
3. **Resume from the last committed event id**, not the last received one. Commit is after
   the projection reducer has run and persisted.
4. **Explicit states.** Approval, denial, cancellation, timeout, and failure each have a
   rendered state. No spinner that means five things.
5. **Redaction is server-side.** The client renders what it is given; it never un-redacts,
   and it never logs a payload it was told to redact.

## Gateway request sequence

Axum, per `axum-agent-gateway`. In order, no steps skipped:

1. Validate request shape and size.
2. Resolve `VerifiedSession` by server-side validation — signature, issuer, audience,
   expiry, revocation. Decoded token hints are display-only.
3. Derive tenant, actor, and allowed persona **from the verified session**. Never from the
   request body.
4. Authorize action and resource with the policy engine.
5. Submit an idempotent run command to UAR with bounded budgets (tokens, wall clock, tool
   calls, output bytes).
6. Stream AG-UI/A2UI events with correlation and resume ids.
7. Propagate client disconnect or explicit cancel to UAR.
8. Persist an immutable, redacted outcome.

Never accept a caller-selected tenant, trusted role, provider route, or raw tool
permission. Never execute an MCP transport from the gateway — that is UAR's interior.

## The agent

The agent is the composer between the resolver and the surface. Its contract is narrow on
purpose.

**It may:** call read-class capabilities to gather evidence; choose which catalog
components to project and with what props; rank fixes; compose a `ParamForm`'s fields;
write prose.

**It may not:** author a component; execute a mutating or privileged capability without the
gate; choose its own provider or model; set a confidence threshold; declare a run complete.

**Prompt structure.** System prompt carries: the persona and its capability subset, the
catalog component ids with their prop schemas, the current projection summary (not the raw
log volume), and the standing instruction that unknown means ask. The capability list is
injected from the manifest — never typed into the prompt, or it drifts the first time a
capability is added.

**Evidence discipline.** A projection must cite what it read. `LogTail` carries the path
and the filter; `TableView` carries the query; `FixPlan` carries `resolvedVia`. A block
with no provenance is not renderable.

---

## A2A boundary

A gateway, not an endpoint. A loopback listener is reachable by every process running as
the user, including a compromised `npm postinstall`.

**Agent Card.** Generated from the capability manifest, so it can never disagree with what
the board renders. Published at the gateway, signed as a JWS over the JCS-canonicalized
card. The spec does not mandate that peers verify cards; we verify ours anyway, and we
verify theirs.

**Skills.** One A2A skill per capability whose `callers` includes `a2a`. Privileged-class
capabilities are excluded by default and require an explicit per-peer grant chosen before
pairing completes, never after.

**Task lifecycle mapping.** A2A's states are ours; do not invent a parallel vocabulary.

| A2A state | Companion meaning |
|---|---|
| `submitted` | Plan accepted, not yet authorized |
| `working` | Authorized, running |
| `input-required` | An open resolution or approval |
| `completed` | `run.completed` with a signed receipt |
| `canceled` | Explicit cancel or peer disconnect |
| `failed` | Policy denial, budget exhaustion, or execution error |

**Per-peer controls.** Every peer gets its own budget, rate limit, and capability subset.
A peer's cancel propagates to UAR. A peer that disappears mid-task has its task cancelled,
not left `working` forever.

**Receipts carry the caller.** `caller: a2a` plus the peer's node id sits in the receipt
envelope. When someone asks why a service restarted at 03:00, the answer is a signed record
naming which peer asked.

## Tests that must exist

Forged, expired, and wrong-audience credentials. Cross-tenant access. Idempotent retries.
Cancellation mid-tool-call. Slow consumers. Unknown event types and unknown protocol minor
versions. Policy denial. Restart and resume from the last event id. Prompt injection
embedded in a tool name, a service description, and a log line. Output flooding. Approval
races — two clients deciding the same `approvalId`. Revoked keys.
