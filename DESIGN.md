# DESIGN.md - AllerX

> Visual design system for AllerX, a read-only medication-history lookup tool for allergy
> assessment. This file defines color, type, layout, and component tokens for the Leptos
> frontend (`app/style/`). For product scope, architecture, and workflow, see `AGENTS.md`.

## Overview

AllerX is used standing up, mid-shift, by a pharmacist who needs one thing: an unambiguous
answer to "has this patient had this drug before, and when." It is not a marketing surface —
it has no hero, no CTA funnel, no course catalog. The whole design exists to serve one
decisive moment: the verdict.

**The signature element is the verdict band** — a full-width, high-contrast strip that appears
the instant a drug search resolves, reading either "พบประวัติ" (found, led by a check-circle
icon) or "ไม่พบประวัติ" (not found, led by an x-circle icon) in a size that can be read from
arm's length without focusing on it. Everything else in the interface is deliberately quiet
so this band is the only loud thing on screen.

Outside that one moment, the palette stays calm and paper-like — closer to a well-typeset
clinical reference than to a product homepage — because the app is read constantly during a
shift and visual fatigue matters. The accent color used for chrome (search focus, active
states, links) is a muted clinical teal-navy, kept deliberately separate from the semantic
red/green of the verdict so the two never compete for the same attention.

**Key characteristics:**
- Paper-white workspace ({colors.canvas}), calm and low-fatigue for repeated all-shift use
- One loud element only: the full-width verdict band (green = found, red = not found)
- Muted teal-navy ({colors.brand-teal}) for chrome — never used for the verdict itself
- IBM Plex Sans Thai + IBM Plex Sans for bilingual clarity; IBM Plex Mono for HN/CID/drug codes
- Rounded-but-restrained corners ({rounded.md}, 8px) — a clinical tool, not a consumer app
- No decorative imagery, no illustration, no gradients — every pixel is legible information

## Colors

### Brand & Chrome
- **Brand Teal** ({colors.brand-teal}): Primary chrome accent — top bar, active tab, focused input border, links. Never used for the verdict band.
- **Brand Teal Dark** ({colors.brand-teal-dark}): Pressed/active state of teal elements.
- **Brand Teal Soft** ({colors.brand-teal-soft}): Pale tint for selected-row background (e.g. selected patient in a result list).

### Semantic — Verdict (reserved, high-contrast, used nowhere else)
- **Found Green** ({colors.verdict-found}): Verdict band background when history is found. Saturated, not pastel — this is the one place saturation is allowed.
- **Found Green Text** ({colors.verdict-found-text}): Text/icon on the found band.
- **Not-Found Red** ({colors.verdict-notfound}): Verdict band background when no history is found.
- **Not-Found Red Text** ({colors.verdict-notfound-text}): Text/icon on the not-found band.
- **Neutral Pending** ({colors.verdict-pending}): Band state while a query is in flight (gray, not red or green — must never imply an answer before one exists).

### Surface
- **Canvas** ({colors.canvas}): App background — warm-neutral off-white, not stark white (reduces glare on long shifts).
- **Canvas Raised** ({colors.canvas-raised}): Cards, panels, the patient info bar.
- **Surface Muted** ({colors.surface-muted}): Table row stripe, secondary panel backgrounds.
- **Hairline** ({colors.hairline}): 1px default border/divider.
- **Hairline Strong** ({colors.hairline-strong}): Input borders, table header rule.

### Text
- **Ink** ({colors.ink}): Primary text — near-black, warm-neutral (not pure #000, matches canvas warmth).
- **Slate** ({colors.slate}): Secondary text — labels, timestamps.
- **Steel** ({colors.steel}): Tertiary text — placeholders, disabled states.
- **On Teal** ({colors.on-teal}): Text on brand-teal surfaces (top bar).
- **Muted Code** ({colors.muted-code}): HN/CID/drug-code text color when de-emphasized in lists.

### Status (non-verdict)
- **Warning Background** ({colors.warning-bg}): Pale amber — used only for system states (e.g. "connection to HOSxP lost"), never for clinical meaning.
- **Warning Text** ({colors.warning-text}): Amber text/icon for the same.

## Typography

### Font Family
- **IBM Plex Sans Thai** (primary, Thai UI text): Humanist, excellent Thai glyph shapes at small sizes, open license.
- **IBM Plex Sans** (primary, Latin/numerals): Pairs directly with Plex Sans Thai — same family, same metrics, so mixed Thai/English/numeral strings (drug names, doctor names) don't visually clash.
- **IBM Plex Mono** (data): HN, CID, drug codes, timestamps — anything that benefits from fixed-width alignment and unambiguous character shapes (0 vs O, 1 vs l matters when the input is a national ID).

### Hierarchy

| Token | Size | Weight | Line Height | Use |
|---|---|---|---|---|
| `{typography.verdict}` | 32px | 600 | 1.20 | The verdict band text — the loudest thing on screen |
| `{typography.patient-name}` | 24px | 600 | 1.25 | Selected patient's name in the patient bar |
| `{typography.heading}` | 18px | 600 | 1.30 | Section headings ("ประวัติการได้รับยา") |
| `{typography.body}` | 16px | 400 | 1.50 | Primary body text, table cells |
| `{typography.body-medium}` | 16px | 500 | 1.50 | Emphasized body (drug name in a history row) |
| `{typography.label}` | 13px | 500 | 1.40 | Field labels, table column headers |
| `{typography.caption}` | 12px | 400 | 1.40 | Timestamps, secondary metadata |
| `{typography.code}` | 14px | 400 | 1.45 | HN, CID, drug code (IBM Plex Mono) |
| `{typography.button}` | 14px | 600 | 1.30 | Button labels |

### Principles
- No display/hero sizes — the largest text on screen is the verdict, at 32px, not a marketing headline.
- Weight does the differentiation, not size: 400 for reading, 500 for emphasis, 600 reserved for the verdict, patient name, and headings.
- Line height stays generous (1.40–1.50) throughout — this is a tool people read carefully, under time pressure, often in imperfect lighting.
- Numerals in HN/CID always render in `{typography.code}` (tabular figures) so columns of numbers align.

## Layout

### Spacing
- Base unit 4px, primary increment 8px.
- Tokens: `{spacing.xs}` (4px) through `{spacing.xxl}` (48px). No `hero`-scale token exists in this system — there is no hero.
- Single-window app: content max-width 960px, centered, with `{spacing.lg}` (24px) side padding on smaller windows.

### Structure (single page, top to bottom)
```
┌───────────────────────────────────────────┐
│ Top bar (brand-teal, app name)             │
├───────────────────────────────────────────┤
│ Patient search (or) Patient bar if selected│
├───────────────────────────────────────────┤
│ Drug search                                │
├───────────────────────────────────────────┤
│ VERDICT BAND (appears only after search)   │
├───────────────────────────────────────────┤
│ Timeline list (only if found)              │
└───────────────────────────────────────────┘
```
There is no sidebar, no multi-panel layout, no navigation beyond this single flow — matching the one-task-at-a-time nature of the work.

## Elevation & Depth

| Level | Treatment | Use |
|---|---|---|
| 0 (flat) | No shadow; `{colors.hairline}` border | Default cards, table rows |
| 1 (raised) | `rgba(20, 30, 28, 0.06) 0px 1px 3px 0px` | Patient bar, search input on focus |
| 2 (band) | `rgba(20, 30, 28, 0.10) 0px 2px 8px 0px` | Verdict band — the only element allowed level-2 elevation |
| 3 (modal) | `rgba(20, 30, 28, 0.16) 0px 8px 24px -4px` | Error/connection-lost dialog |

Elevation is used sparingly and points at meaning: the verdict band is the most elevated
static element on the page precisely because it is the answer the whole screen exists to give.

## Shapes

| Token | Value | Use |
|---|---|---|
| `{rounded.sm}` | 4px | Badges, code chips (HN/CID pills) |
| `{rounded.md}` | 8px | Inputs, buttons, table container, cards |
| `{rounded.lg}` | 12px | Patient bar, verdict band |
| `{rounded.full}` | 9999px | Status dots only (reserved; no live component uses it yet) |

Corners stay modest throughout (8–12px) — restrained enough to read as clinical software,
not a consumer app chasing friendliness. Pills (`{rounded.full}`) are reserved for the tiny
status dot, not applied to buttons the way a marketing site would.

## Components

### Buttons

**`button-primary`** — Main action (search).
- Background `{colors.brand-teal}`, text `{colors.on-teal}`, typography `{typography.button}`, padding `10px 20px`, rounded `{rounded.md}`.
- Pressed: `{colors.brand-teal-dark}`.

**`button-secondary`** — Clear / reset / change patient.
- Background transparent, text `{colors.ink}`, border `1px solid {colors.hairline-strong}`, typography `{typography.button}`, padding `10px 20px`, rounded `{rounded.md}`.

### Search

**`search-input`** — Patient or drug search field.
- Background `{colors.canvas-raised}`, text `{colors.ink}`, border `1px solid {colors.hairline-strong}`, rounded `{rounded.md}`, height 48px, padding `0 {spacing.md}`.
- Focused: border `2px solid {colors.brand-teal}`, elevation level 1.
- Placeholder text `{colors.steel}`.

**`search-result-row`** — One row in the patient/drug autocomplete dropdown.
- Background `{colors.canvas-raised}`, hover/selected background `{colors.brand-teal-soft}`, padding `{spacing.sm} {spacing.md}`, bottom border `1px solid {colors.hairline}`.
- Name in `{typography.body-medium}`, HN/CID in `{typography.code}` `{colors.muted-code}` right-aligned.

### Patient Bar

**`patient-bar`** — Persistent strip showing the selected patient.
- Background `{colors.canvas-raised}`, rounded `{rounded.lg}`, padding `{spacing.lg}`, elevation level 1.
- Name: `{typography.patient-name}`. HN/CID/DOB/sex as a `{typography.caption}` row beneath, HN/CID in `{typography.code}`.
- CID always rendered masked (`1-XXXX-XXXXX-XX-1`) here; full value never shown outside an explicit "show full ID" toggle.

### Verdict Band (signature component)

**`verdict-found`**
- Background `{colors.verdict-found}`, text `{colors.verdict-found-text}`, typography `{typography.verdict}`, rounded `{rounded.lg}`, padding `{spacing.lg} {spacing.xl}`, elevation level 2.
- Content: check-circle icon (44px, `{colors.verdict-found-text}`) + "พบประวัติการได้รับยานี้" + most recent date/location inline, in `{typography.body}` beneath the headline.

**`verdict-notfound`**
- Background `{colors.verdict-notfound}`, text `{colors.verdict-notfound-text}`, typography `{typography.verdict}`, rounded `{rounded.lg}`, padding `{spacing.lg} {spacing.xl}`, elevation level 2.
- Content: x-circle icon (44px, `{colors.verdict-notfound-text}`) + "ไม่พบประวัติการได้รับยานี้".

**`verdict-pending`**
- Background `{colors.verdict-pending}` (neutral gray, never red/green), text `{colors.slate}`, same shape as the above two.
- Content: clock icon (44px, `{colors.slate}`) + "รอการค้นหา".
- Shown only while a query is in flight — must never be styled to suggest an answer.

Rule: only one verdict band exists on screen at a time, and it always fully replaces the
previous one — never stack or fade between states in a way that leaves both partially visible.

### Timeline (history list)

**`timeline-row`** — One prior administration of the searched drug.
- Background `{colors.canvas-raised}`, bottom border `1px solid {colors.hairline}`, padding `{spacing.md} {spacing.lg}`.
- Date in `{typography.body-medium}`, visit type (OPD/IPD) as a small `badge`, prescriber/department in `{typography.caption}` `{colors.slate}`.
- Rows are read top-to-bottom, most recent first — order itself carries clinical meaning, so this is the one place ordering is treated as information, not decoration.

### Badges

**`badge-opd`** / **`badge-ipd`** — Visit-type tag.
- Background `{colors.surface-muted}`, text `{colors.slate}`, typography `{typography.label}`, rounded `{rounded.sm}`, padding `2px 8px`.
- OPD/IPD are distinguished by label text only, not color — color is reserved for the verdict.

### Icons

All icons are lucide-style stroke SVGs, one per component in `app/src/components/icons.rs`:
- 24×24 viewBox, 2px stroke, round caps/joins, `fill: none`, `stroke: currentColor` — the
  icon inherits its color from surrounding text, never hardcoded.
- Sized by CSS via the `.icon` class (18px default; 16px inside buttons, 26px panel headings,
  44px verdict band, 36px patient bar).
- Decorative only (`aria-hidden`) — icons never carry meaning without adjacent text.
- No icon is used where color is the only differentiator; verdict icons duplicate the band's
  headline text, and OPD/IPD remain text-only badges.

**`status-dot`** — Small found/not-found/pending indicator (used in compact contexts, e.g. a future multi-drug list).
- 8px circle, rounded `{rounded.full}`, fill matches the corresponding verdict color.
- Currently superseded by the verdict-band icons; kept as a spec for compact lists.

### System States

**`banner-warning`** — Non-clinical system messages (e.g. "HOSxP connection lost, retrying…").
- Background `{colors.warning-bg}`, text `{colors.warning-text}`, rounded `{rounded.md}`, padding `{spacing.md}`.
- Deliberately amber, not red — must never be visually confused with the not-found verdict.

## Do's and Don'ts

### Do
- Reserve `{colors.verdict-found}` / `{colors.verdict-notfound}` exclusively for the verdict band. No other element may use these colors, ever — including future badges, buttons, or charts.
- Keep the rest of the interface quiet (`{colors.canvas}`, `{colors.brand-teal}`, grays) so the verdict is the only loud moment.
- Use `{typography.code}` (monospace) for every HN, CID, and drug code — never render these in the body sans-serif.
- Mask CID by default everywhere; require an explicit action to reveal it in full.
- Keep corner radii modest (`{rounded.md}`/`{rounded.lg}`) — this reads as trustworthy clinical software, not a consumer app.

### Don't
- Don't use red or green anywhere except the verdict band and its matching status dot — not for buttons, not for links, not for hover states.
- Don't add a hero, marketing copy, or promotional banner — there is no acquisition funnel here.
- Don't apply pill-shaped (`{rounded.full}`) buttons — that shape is reserved for the tiny status dot only.
- Don't animate the verdict band in a way that delays reading it (no fade-ins longer than ~120ms) — the whole point is instant legibility.
- Don't introduce a second accent color beyond `{colors.brand-teal}` for chrome; new UI needs should be solved with weight/size/spacing, not new hues.

## Window & Responsive Behavior

AllerX is a fixed-purpose desktop app (Tauri), not a responsive web page, but the window is
still resizable by the user:

| Window width | Behavior |
|---|---|
| < 720px | Single-column, patient bar and drug search stack fully; verdict band text drops to 24px |
| 720–959px | Content area fills width up to `{spacing.xxl}` side padding |
| ≥ 960px | Content area caps at 960px max-width, centered — prevents line lengths from becoming unreadable on wide monitors |

Minimum supported window size: 480×600px. Below that, the app should still be usable
(no cut-off verdict band), even if cramped.

## Iteration Guide

1. Any new component must declare which existing token set it uses — no ad-hoc hex values in component CSS.
2. If a new state seems to need a new color, check first whether it can be expressed as a weight/border/spacing change instead — this system intentionally has very few named colors.
3. The verdict band's exclusivity rule (red/green nowhere else) is the one rule that should never be relaxed, even under UI pressure to "add a bit of color" elsewhere.
4. Default to `{typography.body}` for body text, `{typography.code}` for anything that is HN/CID/drug-code shaped.
5. Re-check every new screen against the "quiet except for the verdict" principle before shipping it.

## Known Gaps

- Dark mode is out of scope for M0–M6 (see `AGENTS.md` milestones); token names above should support a future dark variant without renaming, but dark values are not yet defined.
- Print/export styling (e.g. printing a patient's drug history) not yet designed.
- Multi-drug batch search (checking several drugs at once) is not yet in scope; `status-dot` exists partly in anticipation of this but has no live use case yet.
