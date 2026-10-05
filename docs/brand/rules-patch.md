# Rule-file patch — branding

`AGENTS.md` and `CLAUDE.md` are generated. Do **not** edit them directly.
Add the blocks below to the templates, then regenerate and verify:

```bash
bash scripts/gen-rule-files.sh
bash scripts/audit-generated.sh
```

---

## A · Append to `templates/rules/agents.md.tmpl`

Insert as a new section after the design-token section (it is the same
"one source, never hand-mirrored" doctrine applied to identity).

```markdown
## §N · Branding (binding)

The visual identity is fixed. `docs/brand/01-branding-guide.md` is the
source of truth; `docs/brand/assets/` holds every mark. Read the guide
before touching an icon, a colour, a font, or any tray artwork.

- **§N.1 — The mark is Node.** A rounded slab plus a detached companion
  square. Never a flame, torch, ember, or spark: Prometheus is a
  CNCF monitoring project with a flame brand and this app is in the
  adjacent category. Do not redraw, rotate, outline, or gradient the
  mark. Use the SVGs in `docs/brand/assets/`.
- **§N.2 — Indigo is identity; the status hues are status.** Brand
  indigo (`[color.accent]`) never indicates health, and success /
  warning / danger / muted never appear as brand. No exceptions.
- **§N.3 — Tray state is shape and alpha, not colour.** macOS template
  images discard colour. Any change to tray artwork must keep the five
  states distinguishable in a single-colour alpha mask, and must ship a
  text label and tooltip alongside. This satisfies the a11y rule that
  status is never colour alone.
- **§N.4 — Three bundled typefaces.** Space Grotesk (display), Inter
  (UI), JetBrains Mono (machine text). Subset WOFF2 files ship with the
  binary; never fetch a font from a CDN at runtime. No fourth family.
  Disable mono ligatures in log tails and ID fields.
- **§N.5 — Values come from `tokens.toml`.** Colour, spacing, radius,
  and type-size literals in component code are a review failure, exactly
  as with the design tokens. Nothing below `[font.size].xs` (12px) ships.
- **§N.6 — Replace the scaffold icons.** `src-tauri/icons/` still holds
  stock Tauri artwork. A signed build with the default icon is a release
  blocker.
```

---

## B · Append to `templates/rules/claude.md.tmpl`

Add these rows to the **§2 skill-trigger table**:

```markdown
| "brand", "branding", "logo", "app icon", "tray icon", "mark", "wordmark", "lockup", "favicon", "typeface", "font", "typography" | read `docs/brand/01-branding-guide.md` first, then `hybrid-design-tokens` |
```

And add to the local rules:

```markdown
### §N · Brand surfaces

Before producing or altering any icon, logo, splash, tray asset, or
type choice, read `docs/brand/01-branding-guide.md` in full. It is
short and binding. Do not generate new brand artwork from a description
in a prompt when a vetted SVG already exists in `docs/brand/assets/`.
Brand changes require the same adversarial review that produced the
current mark: a critic pass on symbolism, icon theory, and colour, and
a judge ruling recorded in the guide's changelog.
```
