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
shift and visual fatigue matters. The accent color used for chrome (primary buttons, focused
inputs, selected rows) is a deep forest green, kept deliberately separate from the semantic
red/green of the verdict so the two never compete for the same attention.

**Key characteristics:**
- Neutral cool paper workspace ({colors.canvas}), calm and low-fatigue for repeated all-shift use
- Flat, hairline-bordered surfaces — structure comes from 1px borders, not shadows or elevation
- One loud element only: the verdict band (tinted alert: green = found, red = not found)
- Deep forest green ({colors.brand-teal}) used with restraint — buttons, focus, selected rows only
- IBM Plex Sans Thai + IBM Plex Sans for bilingual clarity; IBM Plex Mono for HN/CID/drug codes
- Restrained corners ({rounded.md}, 6px) — a clinical tool, not a consumer app
- No decorative chrome: no brand mark, no icon chips, no gradients, no hover animations —
  the information itself is the interface

## Colors

### Brand & Chrome
- **Brand Teal** ({colors.brand-teal}): `#146a46` — deep forest green. Used for primary buttons, focused input borders, selected rows. Never used for the verdict band.
- **Brand Teal Dark** ({colors.brand-teal-dark}): `#0f5538` — hover/pressed state of green elements.
- **Brand Teal Soft** ({colors.brand-teal-soft}): `#e9f4ee` — pale tint for selected-row backgrounds.

### Semantic — Verdict (reserved, used nowhere else)
Treated as a **tinted system alert** (like a hospital status), not a solid banner: light
background, colored text, matching hairline border. Reads clearly at a glance without the
heavy marketing-block look.

- **Found** ({colors.verdict-found}): bg `#e9f6ee`, text `#17724a`, border `#b9e2c8` — history found.
- **Not-Found** ({colors.verdict-notfound}): bg `#fbeded`, text `#b03a2e`, border `#eec4c0` — no history.
- **Neutral Pending** ({colors.verdict-pending}): bg `#f1f3f4`, text `#5b6770`, border `#d9dfe2` — query in flight; green-gray, never implies an answer.

### Surface — neutral cool paper
- **Canvas** ({colors.canvas}): `#f7f9fa` — app background, cool neutral off-white.
- **Canvas Raised** ({colors.canvas-raised}): `#ffffff` — cards, panels, the patient info bar.
- **Surface Muted** ({colors.surface-muted}): `#eef1f3` — hover backgrounds, badges.
- **Hairline** ({colors.hairline}): `#d9dfe2` — 1px default border/divider. The structural
  backbone of the layout — surfaces are separated by hairlines, not shadows.
- **Hairline Strong** ({colors.hairline-strong}): `#b9c3c9` — input borders, top-bar button.

### Text
- **Ink** ({colors.ink}): `#17212b` — primary text, near-black with a cool cast.
- **Slate** ({colors.slate}): `#4b5761` — secondary text — labels, timestamps.
- **Steel** ({colors.steel}): `#77838c` — tertiary text — placeholders, taglines.
- **On Teal** ({colors.on-teal}): `#ffffff` — text on brand-teal buttons.
- **Muted Code** ({colors.muted-code}): `#5c6872` — HN/CID/drug-code text when de-emphasized in lists.

### Status (non-verdict)
- **Warning Background** ({colors.warning-bg}): `#fdf6e3` — pale amber — used only for system states (e.g. "connection to HOSxP lost"), never for clinical meaning.
- **Warning Text** ({colors.warning-text}): `#8a6420` — amber text/icon for the same.

## Typography

### Font Family
- **IBM Plex Sans Thai** (primary, Thai UI text): Humanist, excellent Thai glyph shapes at small sizes, open license.
- **IBM Plex Sans** (primary, Latin/numerals): Pairs directly with Plex Sans Thai — same family, same metrics, so mixed Thai/English/numeral strings (drug names, doctor names) don't visually clash.
- **IBM Plex Mono** (data): HN, CID, drug codes, timestamps — anything that benefits from fixed-width alignment and unambiguous character shapes (0 vs O, 1 vs l matters when the input is a national ID).

### Hierarchy

| Token | Size | Weight | Line Height | Use |
|---|---|---|---|---|
| `{typography.verdict}` | 24px | 700 | 1.25 | The verdict band text — the loudest thing on screen |
| `{typography.patient-name}` | 20px | 600 | 1.30 | Selected patient's name in the patient bar |
| `{typography.heading}` | 15px | 600 | 1.30 | Section headings ("ประวัติการได้รับยา") |
| `{typography.body}` | 15px | 400 | 1.50 | Primary body text, table cells |
| `{typography.body-medium}` | 15px | 500 | 1.50 | Emphasized body (drug name in a history row) |
| `{typography.label}` | 13px | 600 | 1.40 | Field labels |
| `{typography.caption}` | 12px | 400 | 1.40 | Timestamps, secondary metadata |
| `{typography.code}` | 13px | 400 | 1.45 | HN, CID, drug code (IBM Plex Mono) |
| `{typography.button}` | 14px | 600 | 1.30 | Button labels |

### Principles
- The scale is compact and dense — this is a work tool, not a landing page; the verdict at
  24px is the largest text on screen and it must stay that way.
- Weight does the differentiation, not size: 400 for reading, 500 for emphasis, 600–700
  reserved for headings, buttons, and the verdict.
- Line height stays generous (1.40–1.50) throughout — this is a tool people read carefully,
  under time pressure, often in imperfect lighting.
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

Elevation is almost never used. Surfaces are separated by hairlines (1px borders); a shadow
appears only when something must float above the page (the modal).

| Level | Treatment | Use |
|---|---|---|
| 0 (flat) | No shadow; `{colors.hairline}` border | Panels, cards, patient bar, verdict band, top bar |
| 1 (raised) | — (reserved, unused) | — |
| 2 (band) | — (reserved, unused) | — |
| 3 (modal) | `rgba(23, 33, 43, 0.16) 0px 8px 24px -4px` | Settings dialog — the only floating element |

## Shapes

| Token | Value | Use |
|---|---|---|
| `{rounded.sm}` | 4px | Small inner details |
| `{rounded.md}` | 6px | Inputs, buttons, result lists |
| `{rounded.lg}` | 8px | Panels, patient bar, verdict band |
| `{rounded.full}` | 9999px | Tiny chips only — OPD/IPD badges |

Corners stay tight (4–8px) — restrained enough to read as clinical software, not a consumer
app chasing friendliness. Pill-shaped (`{rounded.full}`) elements are limited to tiny chips
(OPD/IPD badges ≤ 20px tall); buttons and panels are never pill-shaped.

## Components

### Buttons

**`button-primary`** — Main action (search).
- Background `{colors.brand-teal}`, text `{colors.on-teal}`, border 1px `{colors.brand-teal}`, typography `{typography.button}`, padding `9px 18px`, rounded `{rounded.md}`. No shadow.
- Hover/pressed: `{colors.brand-teal-dark}`.

**`button-secondary`** — Clear / reset / change patient.
- Background `{colors.canvas-raised}`, text `{colors.ink}`, border `1px solid {colors.hairline-strong}`, typography `{typography.button}`, padding `9px 18px`, rounded `{rounded.md}`.
- Hover: background `{colors.surface-muted}`.

### Search

**`search-input`** — Patient or drug search field.
- Background `{colors.canvas-raised}`, text `{colors.ink}`, border `1px solid {colors.hairline-strong}`, rounded `{rounded.md}`, height 42px, padding `0 {spacing.md}`.
- Focused: border `2px solid {colors.brand-teal}` — border color change only, no glow/halo.
- Placeholder text `{colors.steel}`.

**`search-result-row`** — One row in the patient/drug autocomplete dropdown.
- Background `{colors.canvas-raised}`, hover/selected background `{colors.surface-muted}`, padding `10px {spacing.md}`, bottom border `1px solid {colors.hairline}`.
- Name in `{typography.body-medium}`, HN/CID in `{typography.code}` `{colors.muted-code}` right-aligned.

### Top Bar

**`top-bar`** — Flat, neutral header, chrome only.
- Background `{colors.canvas-raised}`, bottom border `1px solid {colors.hairline}`. No brand color, no mark — just the wordmark and the settings action.
- **`top-bar__title`** — "AllerX", `18px/700`. **`top-bar__tagline`** — `{typography.caption}` `{colors.steel}`.
- **`top-bar__button`** — small secondary-style button with gear icon; label "ตั้งค่า".

### Patient Bar

**`patient-bar`** — Persistent strip showing the selected patient.
- Background `{colors.canvas-raised}`, border `1px solid {colors.hairline}`, rounded `{rounded.lg}`, padding `{spacing.md} {spacing.lg}`.
- **`patient-bar__icon`** — plain 20px user icon in `{colors.slate}`; no chip, no tinted square.
- Name: `{typography.patient-name}`. HN/CID/DOB/sex as a `{typography.caption}` row beneath, HN/CID in `{typography.code}`.
- CID always rendered masked (`1-XXXX-XXXXX-XX-1`) here; full value never shown outside an explicit "show full ID" toggle.

### Verdict Band (signature component)

**`verdict-found`**
- Background `{colors.verdict-found}`, text `{colors.verdict-found-text}`, border `1px solid {colors.verdict-found-border}`, typography `{typography.verdict}`, rounded `{rounded.lg}`, padding `{spacing.md} {spacing.lg}`.
- Content: check-circle icon (26px, `{colors.verdict-found-text}`) + "พบประวัติการได้รับยานี้" + most recent date/location inline, in `{typography.body}` beneath the headline.

**`verdict-notfound`**
- Background `{colors.verdict-notfound}`, text `{colors.verdict-notfound-text}`, border `1px solid {colors.verdict-notfound-border}`, typography `{typography.verdict}`, rounded `{rounded.lg}`, padding `{spacing.md} {spacing.lg}`.
- Content: x-circle icon (26px, `{colors.verdict-notfound-text}`) + "ไม่พบประวัติการได้รับยานี้".

**`verdict-pending`**
- Background `{colors.verdict-pending}`, text `{colors.verdict-pending-text}`, border `1px solid {colors.verdict-pending-border}`, same shape as the above two.
- Content: clock icon (26px, `{colors.verdict-pending-text}`) + "รอการค้นหา".
- Shown only while a query is in flight — must never be styled to suggest an answer.

Rule: only one verdict band exists on screen at a time, and it always fully replaces the
previous one — never stack or fade between states in a way that leaves both partially visible.

### Timeline (history list)

**`timeline-row`** — One prior administration of the searched drug.
- Background `{colors.canvas-raised}`, bottom border `1px solid {colors.hairline}`, padding `{spacing.sm} {spacing.md}`, hover background `{colors.surface-muted}`.
- Date in `{typography.code}` (mono, tabular) `{colors.ink}`, visit type (OPD/IPD) as a small `badge`, prescriber/department in `{typography.caption}` `{colors.slate}`.
- Rows are read top-to-bottom, most recent first — order itself carries clinical meaning, so this is the one place ordering is treated as information, not decoration.

### Badges

**`badge`** — Visit-type tag (OPD/IPD).
- Background `{colors.surface-muted}`, text `#47545c`, typography `{typography.label}`, pill-shaped (`{rounded.full}`), padding `2px 8px`.
- OPD/IPD are distinguished by label text only, not color — color is reserved for the verdict.

### Icons

All icons are lucide-style stroke SVGs, one per component in `app/src/components/icons.rs`:
- 24×24 viewBox, 2px stroke, round caps/joins, `fill: none`, `stroke: currentColor` — the
  icon inherits its color from surrounding text, never hardcoded.
- Sized by CSS via the `.icon` class (16px default; 15px inside buttons, 26px verdict band,
  16px panel headings, 20px patient bar).
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
- Reserve the verdict colors for the verdict band exclusively. No other element may use them, ever — including future badges, buttons, or charts.
- Structure surfaces with hairlines and flat color, not shadows — a panel is a white box with a 1px border, and nothing more.
- Use the forest green sparingly: primary buttons, input focus, selected rows. If a new state needs attention, reach for weight/border/spacing first.
- Keep the type scale compact (15px body, 13px labels) — density is what makes a work tool feel professional.
- Use `{typography.code}` (monospace) for every HN, CID, and drug code — never render these in the body sans-serif.
- Mask CID by default everywhere; require an explicit action to reveal it in full.

### Don't
- Don't use red or green anywhere except the verdict band — not for buttons, not for links, not for hover states.
- Don't add decorative chrome: no brand marks, no icon chips/squares, no gradient or tinted hero areas, no marketing copy.
- Don't add hover animations, glow/focus rings, or elevation to flat surfaces — interaction feedback is a background color change, nothing more.
- Don't add a hero, marketing copy, or promotional banner — there is no acquisition funnel here.
- Don't apply pill-shaped (`{rounded.full}`) buttons — that shape is limited to tiny chips (OPD/IPD badges ≤ 20px tall), never buttons or panels.
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
