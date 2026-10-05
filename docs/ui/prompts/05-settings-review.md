# Prompt 05 · Settings and Review Policy

> Phase 6, plus the settings sections that Phase 4 stubbed. Read
> `docs/ui/40-adversarial-review-config.md`.

---

Invoke before writing code: `orchestrate-prometheus-application`, `hybrid-design-tokens`,
`a11y-gate`.

Implement the provider/model configuration and the adversarial review policy resolver.

## Providers and models

Two sections, both generated from the registry, not hand-laid.

- **Providers:** base URL, keychain reference, model count, p95 latency, health. Keys live in
  the OS secure store; the app holds a reference and asks the store to sign — it never reads
  a key value back, and never writes one into a TOML, a plist, or a PEM.
- An unhealthy provider leaves **every** routing pool, including the review roles, and the
  UI says why. A dangling keychain reference is a specific, nameable failure — show the
  name, not "error".
- **Models:** the registry with provider, context, price in and out per Mtok, and tags. Price
  is recorded per model because the review policy holds a cost ceiling. A model whose
  provider goes unhealthy stays listed but drops out of every pool, and is **never** silently
  substituted mid-run.
- Routing defaults: chat model, fallback chain, degraded-mode model.

## The review resolver

This is the substance. Implement it exactly:

```
producer      = HARNESS[h].model            # not configurable; it is whatever is writing code
excluded      = { provider(producer) }
if h == codex  → excluded += { openai }
if h == claude → excluded += { anthropic }
if h == local  → excluded += { ollama }

critic = configured, if its provider ∉ excluded
         else first healthy configured model whose provider ∉ excluded
excluded += { provider(critic) }

judge  = configured, if its provider ∉ excluded
         else first healthy configured model whose provider ∉ excluded
```

Defaults: critic `moonshot/kimi-k3`, judge `minimax/minimax-m3` — chosen because they are
unlikely to be the harness. A default that collides with the common case is not a default.

**The exclusion set moves with the harness.** This is not a static setting; it is derived.
If you implement it as a stored provider blacklist, you have built the wrong thing.

**Every resolution emits a numbered trace**, rendered in the UI and stored with the verdict:

```
01  harness = codex → producer openai/gpt-5-codex
02  excluded providers for review roles: openai
03  critic ≠ openai/gpt-5-codex · shares a provider with the producer under harness codex
04  critic = moonshot/kimi-k3 · substituted from moonshot
05  judge = minimax/minimax-m3 · provider minimax clears the exclusion set
```

The trace is the artifact. An assignment without one cannot be audited.

**Substitutions are visible** — an amber warning panel naming what was requested, why it was
rejected, and what replaced it. A silent substitution is a lie about what reviewed the code.

**When no alternative provider is configured:** degrade to same-provider review and record
it in the verdict. Do not skip the pass. A disclosed weak review is actionable — it tells
the operator to configure a second provider — where a skipped one produces a verdict
indistinguishable from a real one. If this project prefers hard-fail, say so and change it
in one place.

## Policy and limits

Toggles: review every mutating fix (on), critic rejection blocks approval (on, overridable
with a recorded reason), cache verdicts by diff hash (off). Rounds 2. Tie-break: judge rules,
a tie escalates to the operator and never to a coin flip. Quorum: single judge, three above
blast radius `multi-unit`. Cost ceiling $0.25 per run.

A blocked approve control is `aria-disabled` with the critic's reason attached — never
removed from the DOM.

## Tests

Every harness value yields a legal assignment. A deliberate collision (harness `codex`,
critic `openai/gpt-5-codex`) yields substitution + trace + visible warning. The
no-alternative case degrades and records. An unhealthy provider is never selected. Judge
provider ≠ critic provider ≠ producer provider.
