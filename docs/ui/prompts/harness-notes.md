# Harness Notes

> The same prompts, run by different agents. What changes and what does not.

The prompts in this folder are harness-agnostic. These are the deltas.

## All harnesses

Read in this order before touching code:

1. The target project's `AGENT_BASE_RULES.md` — it wins over anything in this package.
2. `docs/ui/README.md` — the three invariants.
3. The phase file for the phase you are on, and its gate in `60-acceptance-tests.md`.
4. The skills listed for that phase in `50-skills-manifest.md`.

State assumptions and success criteria **before** writing code (Base Rules 1 and 4). If
two interpretations exist, present both and ask. Do not invent an API, a file, a package,
or a behaviour — if it is not known, say so (Rule 5).

## Claude Code

- Skills self-activate through the `UserPromptSubmit` hook if the project was scaffolded
  with `scripts/add-project-skills.sh`. Verify the hook is wired before trusting it —
  `claude-hooks-reliability` exists because silent hook failure is common.
- `PostToolUse` reminds you to run `a11y-gate` after a UI edit. Advisory, not blocking; it
  does not judge the code. Run the gate anyway.
- Prefer the plan-then-execute mode for phase-sized work. One phase per session; the gate
  is the session's exit condition.
- Screenshot loops (`tauri-ui-review`) work natively. Use them; do not describe a UI you
  have not looked at.

## Codex

- No hook mechanism. **Name the skills explicitly in the prompt** — paste the skill's rules
  section if the skill file is not readable from the sandbox.
- Under harness `codex` the review policy excludes the whole `openai` provider from the
  critic and judge roles. That is deliberate; do not "fix" it by selecting a GPT critic.
- Codex tends to produce more code than asked. Base Rule 2 is the counterweight: if 50 lines
  solves it, do not write 200. Speculative abstraction fails Gate review.
- Verify the generated TypeScript mirror is regenerated, not hand-written. This is the
  single most common Codex failure on this codebase.

## Kimi (kimi-code)

- No hook mechanism; name skills explicitly as with Codex.
- Long-context strength: give it the whole spec folder at once rather than one file per
  turn, then ask for the phase plan before code.
- Under harness `local` or when Kimi is the producer, `moonshot` is excluded from the critic
  and judge roles. Defaults shift to `minimax/minimax-m3` and the next healthy provider.
- Watch for over-eager file creation. Every new file must be named in the phase's `Do` list
  or justified in the plan.

## Any harness: what fails review

- A hand-edited generated file.
- A `default:` case added to a `ContentBlock` switch "to make it compile". That silently
  drops a variant; it is the exact failure the union is shaped to prevent.
- A new `ContentBlock` variant for a catalog component. See ADR-UI-1.
- A threshold changed to make a resolver test pass.
- `TanStack Query`, `react-query`, `SWR`, or an Apollo cache anywhere.
- A capability list typed into a prompt instead of injected from the manifest.
- A colour literal in a component.
- "It works" without a runtime artifact — a screenshot, a passing test, or a device run.
