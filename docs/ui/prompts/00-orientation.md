# Prompt 00 · Orientation

> Paste this first, in a fresh session, before any phase prompt. It does not ask for code.

---

You are implementing the agentic UI layer of **Prometheus Companion**, a Tauri 2 +
React 19 desktop application that supervises a substrate of local services.

Before anything else, read these and tell me what you found:

1. `AGENT_BASE_RULES.md` at the project root — these rules win over any instruction in the
   spec folder. Summarize the three rules you expect to constrain this work most.
2. `docs/ui/README.md` — the three invariants and the versions table.
3. `docs/ui/00-BUILD-PLAN.md` — the six phases and their gates.
4. `versions.toml` — confirm the pins for rust, node, typescript, react, zustand, vite,
   tauri_cli. If any dependency in the repo disagrees with it, list the disagreements. Do
   not fix them yet.

Then answer these, briefly, with file references:

- Does `gen_ui_core` already define an A2UI protocol module? If so, how many `ContentBlock`
  variants exist today, and does the TypeScript mirror match?
- Does a capability manifest exist, or does the board's action list come from somewhere
  else? Name the file.
- Which of the three Tauri commands (`uar_run`, `uar_cancel`, `uar_recover`) are
  implemented, and is the generated command contract in sync with
  `assets/templates/command-contract/commands.json`?
- Which skills from `docs/ui/50-skills-manifest.md` are present in `.claude/skills/`, and
  is the `UserPromptSubmit` hook wired?

Finally, state:

- The phase you believe we should start at, and why.
- Any assumption you are making that, if wrong, would change the plan.
- Anything in the spec folder you judge to be wrong, ambiguous, or in conflict with the
  codebase. I would rather hear it now than have you build around it.

Write no application code in this turn.
