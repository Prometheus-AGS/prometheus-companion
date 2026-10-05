# Adversarial Review Configuration

> Which models argue with the producer, and under what rule. Backing skills:
> `orchestrate-prometheus-application`, `realtime-skill-refiner`, `persona-scoped-agent`.

A model does not reliably catch its own family's failure modes. So the critic and the judge
must each come from a different provider than the producer, and from each other. The
producer is whichever harness is driving, which means **the exclusion set moves with the
harness** — it is not a static config value.

## Roles

| Role | Chosen by | Job |
|---|---|---|
| Producer | The harness. Not configurable — it is whatever is writing the code | Produce the diff or the plan |
| Critic | Policy, from the model registry | Attack it: correctness, security, layering, pins |
| Judge | Policy, from the model registry | Rule between producer and critic; record the verdict |

## Resolution

```
producer      = HARNESS[h].model
excluded      = { provider(producer) }
if h == codex  → excluded += { openai }
if h == claude → excluded += { anthropic }
if h == local  → excluded += { ollama }

critic = configured critic, if provider(critic) ∉ excluded
         else the first healthy configured model whose provider ∉ excluded
excluded += { provider(critic) }

judge  = configured judge, if provider(judge) ∉ excluded
         else the first healthy configured model whose provider ∉ excluded
```

Defaults: **critic `moonshot/kimi-k3`**, **judge `minimax/minimax-m3`**. Chosen precisely
because they are unlikely to be the harness — a default that collides with the common case
is not a default, it is a trap. They resolve cleanly under every harness value.

An unhealthy provider is excluded before selection, not after. A substitution is recorded
in the trace and surfaced in the UI; a silent substitution is a lie about what reviewed the
code.

### When nothing else is configured

Degrade to same-provider review and **record it in the verdict**. Do not skip the pass.

This is a judgement call and it is deliberate: a disclosed weak review is worth more than
no review, because the disclosure is itself actionable — it tells the operator to configure
a second provider. Silently skipping produces a verdict that looks identical to a real one.
If your project prefers hard-fail, change it here and say so in the changelog; do not leave
both behaviours possible.

## Resolution trace

Every resolution emits a numbered trace, rendered in the settings surface and stored with
the verdict:

```
01  harness = codex → producer openai/gpt-5-codex
02  excluded providers for review roles: openai
03  critic ≠ openai/gpt-5-codex · shares a provider with the producer under harness codex
04  critic = moonshot/kimi-k3 · substituted from moonshot
05  judge = minimax/minimax-m3 · provider minimax clears the exclusion set
```

The trace is the artifact. An assignment without one cannot be audited, and an operator who
disagrees with a substitution needs to see which rule produced it.

## Policy

| Setting | Default | Why |
|---|---|---|
| Review every mutating fix before it is offered | on | An unreviewed ranking must be marked as such in the thread |
| A critic rejection blocks approval | on | Overridable with a recorded reason, never silently |
| Cache verdicts by diff hash | off | Saves cost; risks a stale verdict when surrounding config moved |
| Rounds | 2 | Producer, critic, judge, then stop |
| Tie-break | judge rules | A tie escalates to the operator, never to a coin flip |
| Quorum | single judge | Three-judge quorum above blast radius `multi-unit` |
| Cost ceiling | $0.25 / run | Per-model price comes from the model registry |

## Integration points

- `review.adversarial` is a capability in the manifest like any other, with `callers`
  including `chat` and `a2a`. It is reviewed by the same registry it belongs to.
- The review run streams as a normal AG-UI run — `run.started` with the three roles in its
  plan, the verdict landing as a `tool.call.result` carrying a catalog artifact.
- A blocked approval renders `FixPlan`'s approve control as `aria-disabled` with the
  critic's reason attached, never removed from the DOM.

## Tests

Every harness value produces a legal assignment. A deliberate collision (harness `codex`,
critic `openai/gpt-5-codex`) produces a substitution plus a trace plus a visible warning.
The no-alternative case degrades and records rather than skipping. An unhealthy provider is
never selected. The judge is never the same provider as the critic.
