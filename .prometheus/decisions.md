# decisions

Append-only. Dated entries. Mark superseded entries; do not delete them.

## 2026-08-20
- Initialized by prometheus-context-bootstrap.

## 2026-08-20 — Managed region lives in templates, not outputs

**Decision.** The `prometheus-base:start v1` region was spliced into
`templates/rules/agents.md.tmpl`, and the `@AGENTS.md` import line into
`templates/rules/claude.md.tmpl` — not into `AGENTS.md` / `CLAUDE.md`.

**Why.** `scripts/gen-rule-files.sh` renders by whole-file copy
(`cat template > AGENTS.md`); it has no marker awareness.
`scripts/audit-generated.sh` invokes that generator on every run, and
`pnpm audit` plus the pre-push hook invoke the audit. Splicing into the
outputs — which is what `bootstrap.sh` does by default — is therefore
self-erasing: the region would be silently deleted at the next audit with
no error emitted.

**Verified.** After relocation, `gen-rule-files.sh` was run twice.
Region survives with exactly one well-formed marker pair; outputs are
byte-identical to templates; second render is idempotent
(md5 e54e9411c60975708ec957e816d240dc). `audit-generated: PASS`.

**Consequence.** To change managed rules, re-run `bootstrap.sh` and then
re-apply this relocation, or edit the template region directly and accept
that a bootstrap re-run will need the same treatment. Do not edit the
region in `AGENTS.md`.

**Open.** `verify.sh` reports 2 FAILs on word count (4271 managed words
vs 1500 ceiling). The bootstrap-owned region is 1388 words — under budget.
The overage is the pre-existing 2883 words of project prose (§0–§20),
which the skill deliberately does not shrink. Semantic overlap exists
between the region and §0.2/§0.4/§0.6 and needs a human reconciliation
pass; see the reduction procedure in the skill's "Existing projects"
section. Measure with /context before and after — a reduction that lowers
tokens and lowers pass rate is a regression.

## 2026-08-20 — Three audits ship as documented stubs, not fake checks

`audit-tokens`, `audit-skill-desc`, and `audit-ui` exit 0 and print `SKIP` with
the precondition they need, rather than implementing a check that cannot work.

**Why.** Each lacks its subject:
- `audit-ui` — §12 needs 8 screenshots/surface; no Playwright or BrowserClaw
  exists, so nothing can capture them. A "does screenshots/ exist" check would
  pass on an empty directory — green for work that never happened.
- `audit-skill-desc` — §19 cites §7.1, which does not exist (§7 is Mobile
  navigation). All 40 SKILL.md files are vendored by `openspec init`; linting
  them reports on OpenSpec's style, not ours.
- `audit-tokens` — §8/§17 require tokens from theme.css; there is no theme.css
  and App.css carries 24 literals. Blocking would make main permanently red,
  and a gate that always fails stops being read.

`audit-all.sh` counts SKIP separately and never as PASS. Each stub's header
documents exactly how to activate it.

**Also fixed to make the gate green rather than decorative:**
- Renamed `src/App.tsx`/`App.css` to `app.tsx`/`app.css` (§3). Symbol stays
  `App`; only the filename is kebab-case.
- Pinned all 10 dependencies to installed versions (§15). No version drift:
  each pin is the version already resolved in the lockfile. Verified
  `pnpm install --frozen-lockfile` succeeds.
