# Prometheus Companion — Brand Guide

> **Status:** v1.0 — adopted 2026-08-20
> **Mark:** Node (option 2a), selected by adversarial review over four alternatives
> **Scope:** desktop app icon, tray icon set, wordmark, typography, colour, voice
> **Source of colour truth:** `assets/templates/design-tokens/tokens.toml` (§8 of the architecture plan). This guide never restates a value the TOML owns; it states the *rules* for using them.

---

## 1 · The mark

**Node** — a rounded slab with a smaller companion square detached at its
upper-right corner.

The slab is the substrate: one in-process binary. The satellite is the
Companion: the small resident thing that watches it. That is the entire
product in two rectangles, and it is the only reading the mark supports.

```
slab       rect x=5  y=21 w=38 h=38 rx=10   (64-unit grid)
satellite  rect x=44 y=5  w=16 h=16 rx=5
```

**Why not a flame.** Prometheus is a CNCF-graduated monitoring system and
time-series database, and its brand is a flame-torch. The Companion
supervises services, aggregates health, and tails logs — the adjacent
category. A flame plus the word Prometheus plus a red/amber/green status
light is read as *that* product. The mythology is the name's etymology,
not this product's idea. Fire is retired from the identity permanently.

**Why two elements.** The satellite is not decoration. It is the state
carrier: it dims, hollows, slashes, or orbits while the slab never moves.
Because state lives in a detachable element, the tray icon stays legible
in a macOS template image, where colour does not exist (§3).

### Construction rules

| Rule | Value |
|---|---|
| Clear space | 0.5 × slab width on all sides |
| Minimum size, mark alone | 16 px |
| Minimum size, lockup | 120 px wide |
| App-icon plate corner radius | 15.6% of plate (rx 80 on 512) |
| Mark share of plate | 54% of plate width |
| Rotation, skew, outline, gradient, shadow on the mark | never |
| Recolouring the mark to a status hue | never (see §2) |

---

## 2 · Colour

### The one rule

**Indigo is identity. The four status hues are status. Neither crosses
into the other's job.**

| Role | Light | Dark | Used for |
|---|---|---|---|
| Brand indigo | `#5B6CFF` | `#7C8BFF` | mark, lockup, focus ring, primary action |
| Success | `#2DA44E` | `#3FB950` | `up` only |
| Warning | `#D98E2B` | `#D29922` | `degraded` only |
| Danger | `#D14343` | `#F85149` | `down` only |
| Muted | `#5B6470` | `#8B949E` | `paused`, secondary text |
| Canvas | `#FAFAF7` | `#0B0F14` | app background |
| Chrome | `#FFFFFF` | `#11151B` | title bar, tray popover |

All values are read from `tokens.toml`. Do not hand-mirror them.

### Why indigo, defended

Infrastructure tooling reads blue for trust and geometric precision, and
category convention says match the neighbourhood then shift the shade.
Indigo is that shift — and it is the only hue family in this palette that
cannot be mistaken for one of the four status colours. Ember or amber
would have collided with `degraded` **and** with the monitoring project's
own brand. An operator whose entire relationship with this app is a small
coloured shape in a menu bar must never have to ask whether a colour means
"brand" or "broken".

### Where colour is allowed to carry meaning

- Dashboard status chips, popover header, service rows — yes.
- Windows and Linux tray icons — yes.
- macOS tray icons — **no**. See §3.

---

## 3 · The tray icon set

macOS template images discard colour entirely: the system paints the
alpha channel to match the menu bar, light or dark. `tray.rs` sets
`.icon_as_template(true)`, so on the primary platform a colour-coded tray
icon is not possible. §11.1 of the architecture plan (five colour states)
and §11.2 (template mode) contradict each other. This guide resolves the
contradiction in favour of the platform.

**State is carried by shape and alpha. Colour is a bonus where the OS
allows it.**

| State | Shape | Alpha | Colour (Win/Linux only) |
|---|---|---|---|
| `up` | slab + solid satellite | 100% | success |
| `degraded` | slab + hollow satellite | 100% | warning |
| `down` | slab + satellite, diagonal knockout | 100% | danger |
| `paused` | slab + satellite | 35% | muted |
| `init` | slab + satellite orbiting the slab | 100% | indigo |

The 35% dim for `paused` follows Apple's own treatment of a disabled menu
bar extra. Every state also ships a tooltip string and a text label in the
popover — the a11y checklist forbids status by colour alone, and here
colour is not even available.

**Assets:** `docs/brand/assets/tray-<state>-{template,light,dark}.svg`.
`scripts/gen-tray-icons.sh` rasterises these to 16/22/32/44/88/176 px.
Only the `-template` set is fed to macOS.

---

## 4 · Typography

Three families ship inside the binary. A fourth is web-only.

| Role | Family | Weights | Where |
|---|---|---|---|
| Display | **Space Grotesk** | 600, 700 | wordmark, window titles, empty-state headlines, dashboard section titles |
| UI | **Inter** | 400, 500, 600 | every control, label, table cell, menu item, button |
| Mono | **JetBrains Mono** | 500, 600 | log tails, receipts, device IDs, queue counters, version tags, `// eyebrows` |
| Long-form | Roboto | 400 | docs site and marketing only — never bundled in the app |

### The defence

**Inter for UI.** It was drawn for computer screens at small sizes: a tall
x-height, open apertures, and a disambiguated glyph set that keeps `1`,
`l`, `I` and `0`, `O` apart. In this app that is not a nicety — a service
row reads `liter-llm` next to `Iroh` and an operator must not squint.
Inter also ships tabular figures, which every queue-depth, latency, and
uptime column in the dashboard needs to stop numbers from dancing on
refresh. It is already the `[font.family].sans` token, so adopting it
costs nothing.

**JetBrains Mono for machine text.** The Companion's whole reason to exist
is showing machine output to a human: signed receipts, Ed25519 node IDs,
log tails, PromQL-ish queries, error strings. JetBrains Mono was drawn for
long code-reading sessions — taller lowercase for the same cap height, so
a wall of monospace stays scannable, and unambiguous `0/O`, `1/l/I`,
`5/S`, `8/B` at 12 px, which is the difference between reading a
fingerprint correctly and pairing the wrong device. **Disable its code
ligatures in log tails and ID fields**: `->` must stay two characters when
the operator is copying a literal. Keep them in code blocks only.

**Space Grotesk for display.** Inter is excellent and anonymous; a product
needs one voice that is not. Space Grotesk is a geometric grotesque with
tight apertures and slightly odd terminals — it reads *engineered* rather
than *corporate*, which matches a brand whose stated metaphor is polished
concrete. Critically it shares a neo-grotesque skeleton with Inter, so the
two pair without arguing: same underlying proportions, different
personality. Set it at `-0.03em` for hero, `-0.02em` for h2/h3, never
uppercase, never below 17 px. Restricting it to display keeps the
personality from becoming noise across a dense operator UI.

**Why not fewer.** Two families (Inter + mono) would work and would be
duller; the app would look like a generated admin panel. Why not more:
a fourth bundled family is dead weight in a signed desktop binary for no
communicative gain, which is why Roboto stays on the docs site.

**Licensing.** Space Grotesk, Inter, and JetBrains Mono are all
open-licensed (SIL OFL / Apache 2.0) and may be embedded in a
redistributed, code-signed desktop application. Bundle the WOFF2 subsets
locally — the app must render correctly offline and must not phone a font
CDN on launch.

### Type scale

Sizes come from `[font.size]` in `tokens.toml`. Do not invent one.

**The floor is `xs = 12px` for all product interface text** — every label,
caption, annotation, table cell, tooltip, and status string in the app and
in brand documentation. Nothing smaller ships.

**One exemption: lockup artwork.** The `PROMETHEUS` eyebrow inside the
wordmark lockup is drawn type, not interface text — it is sized at 0.22×
the cap height of `Companion` and scales with the lockup rather than with
the type scale. It carries no information the adjacent word does not, so
it never needs to be read at a size the floor would govern. This is the
only place type may fall below 12 px, and it may not be used as precedent
for captions, badges, or metadata rows.

---

## 5 · Voice

Short. Declarative. Second-person imperative for actions ("Pair a
device"), impersonal statements for product facts ("Every run emits a
signed receipt"). Metaphors from concrete, steel, light, and server
rooms. Commas over em-dashes. Periods over exclamation points. No emoji
in any shipped surface — the icons carry meaning.

**Write:** precision-engineered, zero-trust, production-hardened,
supervised, paired, degraded.
**Do not write:** seamless, unlock, supercharge, revolutionary,
AI-powered, delight.

Status copy is always literal and always paired with a shape: "Degraded —
skill-index is up but not ready." Never "Something went wrong."

---

## 6 · Asset index

```
docs/brand/
├── 01-branding-guide.md              this file
├── brand-template.html               living visual guide (open in a browser)
├── mood-board.html                   the intended feel, with sources
├── rules-patch.md                    the block to add to the rule templates
└── assets/
    ├── companion-mark-indigo.svg          mark, light surfaces
    ├── companion-mark-indigo-dark.svg     mark, dark surfaces
    ├── companion-mark-mono-white.svg      overlays
    ├── companion-mark-mono-dark.svg       overlays
    ├── companion-appicon-light.svg        512 squircle, indigo plate
    ├── companion-appicon-dark.svg         512 squircle, charcoal plate
    ├── companion-lockup-light.svg         mark + PROMETHEUS / Companion
    ├── companion-lockup-dark.svg
    ├── favicon.svg
    └── tray-{up,degraded,down,paused,init}-{template,light,dark}.svg
```

**Not yet generated:** `.icns`, `.ico`, and the Windows Square*Logo PNG
set. Run `pnpm tauri icon docs/brand/assets/companion-appicon-light.svg`
to produce `src-tauri/icons/`, and `scripts/gen-tray-icons.sh` for the
tray rasters. The scaffold icons currently in `src-tauri/icons/` are the
stock Tauri artwork and must be replaced before any signed build.
