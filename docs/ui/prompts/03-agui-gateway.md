# Prompt 03 · AG-UI Stream and Gateway

> Phases 2 and 5. Read `docs/ui/20-agui-agent-spec.md`.

---

Invoke before writing code: `agui-event-contract`, `axum-agent-gateway`,
`agent-runtime-security`, `pem-local-first`.

## Part A · The client stream (Phase 2)

Implement the AG-UI client and the run projection.

The envelope is fixed: `protocolVersion`, `eventId`, `runId`, `seq`, `ts`, `type`,
`payload`, optional `causalParent`. `eventId` is stable across retries — that stability is
the entire reliability story, so if you find a code path that mints a new id on retry, that
path is the bug.

Requirements:

- Idempotent application keyed by `eventId`, with a bounded seen-set persisted alongside the
  projection.
- Sequence enforcement with a 2 s gap buffer, then a **replay request** from the last
  committed seq — never an out-of-order apply.
- Resume from the last **committed** event id, where committed means the reducer ran and the
  projection persisted. Not the last received id.
- Zustand store holding the projection. Components read through `useRun`; no component
  touches the transport.
- Explicit rendered states for approval, denial, cancellation, timeout, and failure. One
  spinner may not mean five things.
- Unknown event types: preserved, displayed as an unknown-event artifact, **not** success.
  Unknown protocol *minor* accepted with fields preserved; unknown *major* rejected.
- Only `run.completed` completes a run. A model writing "all done" is text.

## Part B · The gateway and A2A (Phase 5)

Axum. The request sequence is ordered and no step may be skipped:

1. Validate shape and size.
2. Resolve `VerifiedSession` by server-side validation — signature, issuer, audience,
   expiry, revocation. Decoded token hints are display-only.
3. Derive tenant, actor, allowed persona **from the verified session**, never the body.
4. Authorize action and resource with the policy engine.
5. Submit an idempotent run command to UAR with bounded budgets — tokens, wall clock, tool
   calls, output bytes.
6. Stream events with correlation and resume ids.
7. Propagate disconnect or explicit cancel to UAR.
8. Persist an immutable, redacted outcome.

Never accept a caller-selected tenant, trusted role, provider route, or raw tool permission.
Never execute an MCP transport from the gateway.

For A2A: build a **gateway, not an endpoint**. A loopback listener is reachable by every
process running as the user, including a compromised `npm postinstall` — design as if one is
already running.

- The Agent Card is **generated from the capability manifest**, signed as a JWS over the
  JCS-canonicalized card. It can never disagree with what the board renders.
- One A2A skill per capability whose `callers` includes `a2a`. Privileged-class capabilities
  are excluded by default and need an explicit per-peer grant chosen before pairing
  completes, never after.
- Map to A2A's own task states — `submitted`, `working`, `input-required`, `completed`,
  `canceled`, `failed`. Do not invent a parallel vocabulary.
- Per-peer budget, rate limit, and capability subset. A peer's cancel propagates. A peer
  that disappears mid-task has its task cancelled, not left `working`.
- Receipts carry `caller` and the peer node id.

## Tests, all of them

Forged, expired, wrong-audience, and revoked credentials. Cross-tenant access. Idempotent
retries. Cancellation mid-tool-call. Slow consumers. Unknown events and unknown minor
versions. Policy denial. Restart and resume. Output flooding. Approval races — two clients
deciding one `approvalId`. And prompt injection embedded in a tool name, a service
description, and a log line: tool names and descriptions are untrusted data.

Tell me which of these already have coverage before you write new ones.
