# KBD Constraints — Prometheus Companion

Derived from `AGENTS.md` §0 (Constitution), §3 (kebab-case), §4 (layer rule),
§13 (secrets), §15 (pins), §17 (anti-patterns).

Every `check:` below was executed against the tree at init time (2026-08-20) and
returned zero hits unless noted, so each blocking rule starts from a green state.
A constraint that already fails on a clean checkout is a broken gate, not a rule.

**Command environment:** no mandatory wrapper, toolchain selector, or external
`CARGO_TARGET_DIR` applies to this project. Commands run as written from the repo
root. `AGENTS.md` §1 requires all validation to be local — never cite a hosted CI
run as evidence for any check here.

---

## Blocking Constraints (prevent archiving until resolved)

```yaml
constraints:
  - id: build-passes
    severity: blocking
    description: 'Project build must pass without errors'
    command: 'pnpm run build'
    note: 'Verified passing at init (exit 0, 19 modules, 396ms).'

  - id: generated-rule-files-intact
    severity: blocking
    description: 'AGENTS.md and CLAUDE.md must match their templates and carry the GENERATED banner'
    command: 'bash scripts/audit-generated.sh'
    note: >
      This is the ONLY audit script that currently exists. It invokes
      gen-rule-files.sh, which renders templates over the outputs by whole-file
      copy. The prometheus-base managed region therefore lives in
      templates/rules/*.md.tmpl, NOT in AGENTS.md. Editing the region in
      AGENTS.md is silently discarded on the next run. See
      .prometheus/decisions.md (2026-08-20).

  - id: no-console-log
    severity: blocking
    description: 'No console.log in committed TypeScript (AGENTS.md §17)'
    check: "grep -rn 'console\\.log' src/ --include='*.ts' --include='*.tsx'"

  - id: no-any-type
    severity: blocking
    description: 'No `any` type in TypeScript source'
    check: "grep -rn ': any' src/ --include='*.ts' --include='*.tsx'"

  - id: no-radix-imports
    severity: blocking
    description: 'No @radix-ui/* imports — use @base-ui-components/react (AGENTS.md §8, §17)'
    check: "grep -rn '@radix-ui/' src/"

  - id: no-react-query
    severity: blocking
    description: 'No @tanstack/react-query — entity data goes through PEM 3.x (AGENTS.md §9, §17)'
    check: "grep -rn '@tanstack/react-query' src/"

  - id: no-rust-unwrap
    severity: blocking
    description: 'No unwrap() in Rust — use ? and typed errors (AGENTS.md §17)'
    check: "grep -rn 'unwrap()' src-tauri/src/"

  - id: no-wildcard-cargo-versions
    severity: blocking
    description: 'No `*` version specs in Cargo.toml (AGENTS.md §15)'
    check: "grep -nE '^[a-z_-]+ *= *\"\\*\"' src-tauri/Cargo.toml"

  - id: no-hardcoded-secrets
    severity: blocking
    description: 'No API keys, tokens, or passwords in source — OS secure store only (AGENTS.md §13)'
    check: "grep -rn 'sk-\\|api_key\\|API_KEY\\|secret.*=.*[\"\\x27][A-Za-z0-9]' src/ src-tauri/src/"

  - id: no-committed-env
    severity: blocking
    description: 'No .env file committed (AGENTS.md §13)'
    check: "git ls-files --error-unmatch .env 2>/dev/null"
```

---

## Warning Constraints (acknowledge before archiving)

```yaml
warnings:
  - id: kebab-case-file-names
    severity: warning
    description: 'TypeScript files and folders under src/ are kebab-case (AGENTS.md §3)'
    check: "find src -type f \\( -name '*.ts' -o -name '*.tsx' \\) | grep -vE '/(index|__root)\\.(ts|tsx)$' | grep -E '/[^/]*[A-Z][^/]*$'"
    known_violations:
      - 'src/App.tsx — Tauri scaffold default; rename to src/app.tsx'
    note: >
      WARNING, not blocking, because the repo violates this rule today. AGENTS.md
      §3 declares scripts/audit-naming.sh as the enforcement, but that script does
      not exist. Promote to blocking once App.tsx is renamed and the audit script
      is written.

  - id: clean-layer-rule
    severity: warning
    description: 'Dependencies point inward: Domain ← Application ← Presentation; Infrastructure implements Application ports (AGENTS.md §4)'
    note: >
      Cannot be mechanically checked yet — src/ has no domain/, application/,
      infrastructure/, or presentation/ directories. The 4-layer structure is
      declared in AGENTS.md but not yet built. Promote to blocking once the
      layers exist and scripts/audit-layer.sh is written.

  - id: tauri-surface-gating
    severity: warning
    description: 'Every @tauri-apps/* import is wrapped in isTauri() or a sync-gated module body (AGENTS.md §5)'
    check: "grep -rln '@tauri-apps/' src/ | xargs -r grep -L 'isTauri'"
    note: 'Web builds must not crash with "no __TAURI_INTERNALS__ bridge".'

  - id: tests-exist
    severity: warning
    description: 'New features have tests'
    note: >
      No test runner is configured — package.json has no test script and there is
      no vitest/jest/playwright config. project.json test_command is null. Any
      claim that tests passed is unverifiable until a runner is wired.

  - id: a11y-gate
    severity: warning
    description: 'WCAG 2.2 AA: accessible names, keyboard operability, contrast, target size (AGENTS.md §11)'
    note: 'scripts/audit-a11y.sh does not exist. Manual review required per AGENTS.md §12.'
```

---

## Declared-but-absent enforcement

`AGENTS.md` §19 advertises a `pnpm audit` umbrella of ten `audit:*` scripts and
states a pre-push hook refuses to push if any fails. Measured at init:

| Script | Status |
|---|---|
| `audit-generated.sh` | exists |
| `audit-layer.sh` | **missing** |
| `audit-naming.sh` | **missing** |
| `audit-tokens.sh` | **missing** |
| `audit-a11y.sh` | **missing** |
| `audit-deps.sh` | **missing** |
| `audit-secret-leak.sh` | **missing** |
| `audit-skill-desc.sh` | **missing** |
| `audit-progress.sh` | **missing** |
| `audit-ui.sh` | **missing** |

No `audit:*` key exists in `package.json`, so `pnpm audit` does not resolve to
these scripts. The `check:` expressions above are inline substitutes for the
missing ones. They are not a replacement for the real scripts — a grep is a
heuristic, and §4's layer rule in particular cannot be reduced to one.
