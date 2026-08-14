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
inputs, selected rows) is a deep red, aligned with the app icon's palette (`#FF5252` →
`#D32F2F`), kept deliberately separate from the semantic red/green of the verdict so the two
never compete for the same attention.

**Key characteristics:**
- Neutral warm paper workspace ({colors.canvas}), calm and low-fatigue for repeated all-shift use
- Flat, hairline-bordered surfaces — structure comes from 1px borders, not shadows or elevation
- One loud element only: the verdict band (tinted alert: green = found, red = not found)
- Deep red ({colors.brand}) used with restraint — buttons, focus, selected rows only, matches app icon
- IBM Plex Sans Thai + IBM Plex Sans for bilingual clarity; IBM Plex Mono for HN/CID/drug codes
- Restrained corners ({rounded.md}, 6px) — a clinical tool, not a consumer app
- App logo (pill bottle icon) in top bar, red gradient matching brand chrome

### Design Principles (informed by Microsoft UX Guidelines & Desktop Design Systems)

1. **Progressive disclosure** — Show only what's needed at each step. Patient search first,
   drug search only after a patient is selected. Verdict appears only after a query.
2. **Asymmetric information hierarchy** — Patient search is the primary action (wider column);
   drug search is secondary (narrower column). Layout reflects the natural task order.
3. **Density over whitespace** — This is a work tool, not a landing page. Compact spacing
   lets pharmacists scan more information per glance.
4. **Keyboard-first navigation** — Every interactive element must be reachable via Tab/Enter.
   Visible focus rings on all inputs and buttons.
5. **Native feel** — Hover/focus transitions at 200ms, no bounce/elastic animations.
   Respects `prefers-reduced-motion`.
6. **Inverted pyramid** — The verdict (answer) is the most prominent element. Supporting
   details (timeline, patient info) are secondary. System chrome is tertiary.

## Colors

### Brand & Chrome (aligned with app icon palette)
- **Brand** ({colors.brand}): `#D32F2F` — deep red from app icon. Used for primary buttons, focused input borders, selected rows. Matches the icon gradient's darker stop. Never used for the verdict band.
- **Brand Dark** ({colors.brand-dark}): `#B71C1C` — hover/pressed state of red elements.
- **Brand Soft** ({colors.brand-soft}): `#FFEBEE` — pale tint for selected-row backgrounds, patient bar.

### Semantic — Verdict (reserved, used nowhere else)
Treated as a **tinted system alert** (like a hospital status), not a solid banner: light
background, colored text, matching hairline border. Reads clearly at a glance without the
heavy marketing-block look.

- **Found** ({colors.verdict-found}): bg `#E8F5E9`, text `#2E7D32`, border `#A5D6A7` — history found.
- **Not-Found** ({colors.verdict-notfound}): bg `#FFEBEE`, text `#C62828`, border `#FFCDD2` — no history.
- **Neutral Pending** ({colors.verdict-pending}): bg `#F5F5F5`, text `#616161`, border `#E0E0E0` — query in flight; neutral gray, never implies an answer.
- **Unverifiable** ({colors.verdict-unresolved}): bg `#FFF8E1`, text `#8a6420`, border `#FFE082` — the drug term could not be matched to the formulary (`drugitems`). Amber warns "this needs checking", and is deliberately distinct from both the green/red verdicts (never implies found/not-found) and from system-state amber (this is a *clinical* answer: "cannot confirm, choose a drug from the suggestions"). Rendered by the `verdict-unresolved` state (ROADMAP Phase 1, Gap G1).

### Surface — neutral warm paper
- **Canvas** ({colors.canvas}): `#FAFAFA` — app background, warm neutral off-white.
- **Canvas Raised** ({colors.canvas-raised}): `#FFFFFF` — cards, panels, the patient info bar.
- **Surface Muted** ({colors.surface-muted}): `#F5F5F5` — hover backgrounds, badges.
- **Hairline** ({colors.hairline}): `#E0E0E0` — 1px default border/divider. The structural
  backbone of the layout — surfaces are separated by hairlines, not shadows.
- **Hairline Strong** ({colors.hairline-strong}): `#BDBDBD` — input borders, top-bar button.

### Text
- **Ink** ({colors.ink}): `#212121` — primary text, near-black.
- **Slate** ({colors.slate}): `#616161` — secondary text — labels, timestamps.
- **Steel** ({colors.steel}): `#9E9E9E` — tertiary text — placeholders, taglines.
- **On Brand** ({colors.on-brand}): `#FFFFFF` — text on brand-red buttons.
- **Muted Code** ({colors.muted-code}): `#757575` — HN/CID/drug-code text when de-emphasized in lists.

### Status (non-verdict)
- **Warning Background** ({colors.warning-bg}): `#FFF8E1` — pale amber — used only for system states (e.g. "connection to HOSxP lost"), never for clinical meaning.
- **Warning Text** ({colors.warning-text}): `#8a6420` — amber text/icon for the same — shares the value with {colors.verdict-unresolved-text} so the warning pair passes WCAG AA normal-text contrast (4.6:1).

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

### Structure — Two-Panel Desktop Layout

```
┌─────────────────────────────────────────────────────────────────┐
│ Top bar (40px: app name · connection dot · settings)            │
├───────────────────────┬─────────────────────────────────────────┤
│                       │                                         │
│  SIDEBAR (360px)      │  MAIN CANVAS (remaining)               │
│                       │                                         │
│  ┌─────────────────┐  │  ┌───────────────────────────────────┐  │
│  │ Patient search  │  │  │                                   │  │
│  │ [input]         │  │  │  VERDICT BAND (full width)        │  │
│  │ [results]       │  │  │  พบประวัติ / ไม่พบประวัติ        │  │
│  ├─────────────────┤  │  │                                   │  │
│  │ Patient bar     │  │  └───────────────────────────────────┘  │
│  │ (when selected) │  │                                         │
│  ├─────────────────┤  │  ┌───────────────────────────────────┐  │
│  │ Drug search     │  │  │                                   │  │
│  │ [input]         │  │  │  TIMELINE (full width)             │  │
│  │ [results]       │  │  │  ประวัติการได้รับยา               │  │
│  │ [button]        │  │  │  ┌─ row ──────────────────────┐   │  │
│  └─────────────────┘  │  │  │ date · badge · drug · meta │   │  │
│                       │  │  └────────────────────────────┘   │  │
│                       │  │  ┌─ row ──────────────────────┐   │  │
│                       │  │  │ date · badge · drug · meta │   │  │
│                       │  │  └────────────────────────────┘   │  │
│                       │  │                                   │  │
│                       │  └───────────────────────────────────┘  │
│                       │                                         │
└───────────────────────┴─────────────────────────────────────────┘
```

**Two-panel rationale:** This is a classic desktop application pattern (VS Code, TablePlus,
medical record systems). The sidebar is the **input panel** — all user actions happen here.
The main canvas is the **output panel** — results appear here. This spatial separation maps
directly to the pharmacist's mental model: "I tell it who + what on the left, I see the
answer on the right."

**Sidebar (360px fixed):**
- Contains the complete search workflow: patient search → patient bar → drug search.
- Stays fixed while the main canvas scrolls (important for long timelines).
- 1px right border (`{colors.hairline}`) separates it from the canvas.
- Background `{colors.canvas}` (same as app background) — the sidebar is part of the
  workspace, not a separate surface.

**Main Canvas (fluid):**
- Takes remaining width after sidebar.
- Scrollable vertically when timeline exceeds viewport height.
- Verdict band sits at the top — always visible without scrolling.
- Timeline rows fill the rest, dense and scrollable.
- Background `{colors.canvas-raised}` (white) — the canvas is the "paper" where results live.

**Why not bento/grid:** Bento grids work when all cells are equal inputs. Here the two
sides have fundamentally different roles (input vs output) and different scroll behaviors
(sidebar fixed, canvas scrollable). A two-panel layout handles this correctly.

## Elevation & Depth

Elevation is minimal. The sidebar and canvas are distinguished by background color
(canvas = white, sidebar = off-white), not shadows. Shadows float only the modal.

| Level | Treatment | Use |
|---|---|---|
| 0 (flat) | No shadow; `{colors.hairline}` border | Sidebar, top bar, verdict band, timeline rows |
| 1 (canvas) | `background: {colors.canvas-raised}` | Main canvas surface (white on off-white bg) |
| 2 (band) | — (reserved, unused) | — |
| 3 (modal) | `0 8px 24px rgba(23,33,43,0.16)` | Settings dialog — the only floating element |

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

### Top Bar

**`top-bar`** — Thin (44px), flat, neutral header spanning full width.
- Background `{colors.canvas-raised}`, bottom border `1px solid {colors.hairline}`.
- **`top-bar__left`** — App logo (inline SVG pill bottle, 28×28, red gradient) + title.
- **`top-bar__title`** — "AllerX", `16px/700`, left-aligned. No tagline on this layout.
- **`top-bar__status`** — Connection indicator: 8px dot + text, right-aligned next to settings.
- **`top-bar__button`** — Ghost button with gear icon, tooltip "ตั้งค่า".

### Sidebar

**`sidebar`** — Fixed-width (360px) left panel, full height below top bar.
- Background `{colors.canvas}` (off-white, same as app bg), right border `1px solid {colors.hairline}`.
- Contains three stacked sections separated by hairlines:
  1. Patient search
  2. Patient bar (when patient selected)
  3. Drug search
- **Stays fixed** while main canvas scrolls.

### Patient Search (sidebar)

**`sidebar__section`** — Section inside sidebar with heading.
- Padding `{spacing.md}`, border-bottom `1px solid {colors.hairline}`.
- Heading: `{typography.label}` with icon, e.g. "ค้นหาผู้ป่วย".

**`search-input`** — Full-width input inside sidebar section.
- Height 40px (compact for sidebar), border `{colors.hairline-strong}`, rounded `{rounded-md}`.
- Focused: border `2px solid {colors.brand}`.

**`search-result-row`** — Autocomplete dropdown row.
- Compact: padding `8px {spacing.md}`, two lines (name + HN/CID code).

### Patient Bar (sidebar)

**`patient-bar`** — Compact patient context strip inside sidebar.
- Background `{colors.brand-soft}`, border `1px solid {colors.verdict-found-border}`, rounded `{rounded-md}`, padding `{spacing.sm} {spacing.md}`.
- **Layout:** Icon (16px) | Name (15px/600) + meta (12px: HN · CID masked · DOB · sex) | Change button (ghost, X icon).
- CID always masked (`1-XXXX-XXXXX-XX-1`).
- **Change patient button:** Ghost button at top-right, tooltip "เปลี่ยนผู้ป่วย". Clears patient + resets verdict.

### Drug Search (sidebar)

**`drug-search`** — Same input style as patient search, but disabled until patient selected.
- Placeholder: "ชื่อยา (สามัญ / การค้า)".
- **Disabled state:** `opacity: 0.5`, `cursor: default`, no pointer events.
- Autocomplete dropdown appears below input.
- **Chip queue (Phase 5):** pressing Enter (or clicking a suggestion) queues the drug as a `chip` — pill-shaped (≤20px tall, `{rounded.full}`), removable via `chip__remove`, deduped by icode/label. "ตรวจประวัติ" checks the whole queue (a single drug is a batch of one); "ล้างทั้งหมด" clears the queue and verdict.

### Main Canvas

**`main-canvas`** — Fluid right panel, scrollable.
- Background `{colors.canvas-raised}` (white), left border `1px solid {colors.hairline}`.
- Padding `{spacing-lg}`.
- Scrollable via `overflow-y: auto`.

### Verdict Band (main canvas, top)

**`verdict-band`** — Full-width result banner at top of canvas.
- Rounded `{rounded-lg}`, padding `{spacing-lg}`, border `{colors.hairline}`.
- **Found:** bg `{colors.verdict-found}`, text `{colors.verdict-found-text}`, border `{colors.verdict-found-border}`. Check-circle icon (32px) + headline (24px/700) + detail line.
- **Not-Found:** bg `{colors.verdict-notfound}`, text `{colors.verdict-notfound-text}`, border `{colors.verdict-notfound-border}`. X-circle icon + headline + detail.
- **Pending:** bg `{colors.verdict-pending}`, text `{colors.verdict-pending-text}`. Clock icon + headline + contextual hint.
- **Unverifiable:** bg `{colors.verdict-unresolved}`, text `{colors.verdict-unresolved-text}`, border `{colors.verdict-unresolved-border}`. X-circle icon + headline + detail — never the "ไม่พบประวัติ" text (Phase 1).
- Only one verdict on screen at a time — a single-drug check is one band; a batch check is **`verdict-batch`**, a grid of **`verdict-band--compact`** bands (one per checked drug), each term-labelled (`verdict-band__term`, 14px/600) above a condensed detail line. The grid is **2 columns at ≥960px**, stacked 1 column below (the canvas scrolls for many drugs). Compact bands keep the same three semantic palettes; the unresolved compact band may embed `candidate-button` chips that queue the drug for re-check. Never fade between states.

### Print Sheet (Phase 5)

**`print-sheet`** — a printable Thai patient+history sheet, invisible on screen (`display: none`), the only content rendered in `@media print`:
- Header: app name + print timestamp; patient block (name/HN/CID/birth date); verdict table (drug term | status | detail); history table (date | OPD/IPD | drug | prescriber | department); footer disclaimer ("HOSxP ยังคงเป็นแหล่งข้อมูลหลัก").
- Print tokens: `@page` margin 14mm, black-on-white, 12px body / 11px table text, 1px `#999` table borders — designed to survive greyscale and hospital printers. The app chrome is hidden by `display: none !important` on its **siblings** (`.top-bar`, `.app__body`, `.modal-backdrop`) — never on `.app` itself, because an ancestor `display: none` would swallow the sheet's whole subtree (the empty-print bug). `html/body/.app` drop their screen `height`/`overflow` constraints so long sheets paginate correctly.

### Timeline (main canvas, below verdict)

**`timeline`** — Dense scrollable list of medication history records.
- List-style: no bullets, no outer padding.
- **`timeline-filter`** — shown for multi-drug checks: "ทั้งหมด" + one chip per checked drug (neutral `timeline-filter__chip`, `--active` state in brand-soft/brand). Clicking a chip isolates that drug's rows; clicking again (or ทั้งหมด) restores the merged view. Filters reset automatically on the next check.
- **`timeline-row`** — One record per row. Two-line layout:
  - Line 1: `{typography.code}` date (80px min-width) | OPD/IPD badge | drug name + strength (`{typography.body-medium}`)
  - Line 2: prescriber @ department (`{typography.caption}`, `{colors.slate}`)
- Row padding `8px {spacing.md}`, border-bottom `1px solid {hairline}`.
- Hover: background `{colors.surface-muted}`.
- Most recent first — order carries clinical meaning.
- **`timeline-footer`** — "แสดงทั้งหมด N รายการ" in `{typography.caption}`, centered.

### Patient Detail Modal (Phase 5)

**`patient-detail-modal`** — elevation-3 modal opened from the patient bar
("ดูข้อมูลผู้ป่วย"). The DESIGN.md-mandated detail view where the **full
CID is revealed** (the only place in the app). Contents: demographics grid
(name / HN / CID / birth date / sex) plus the "ยาที่ได้รับล่าสุด (30 วัน)"
snapshot (`med-row` list, monospace dates, trade names in parentheses).
Empty state: "ไม่มีรายการจ่ายยาใน 30 วันที่ผ่านมา".

## Do's and Don'ts

### Do
- Reserve the verdict colors for the verdict band exclusively. No other element may use them, ever — including future badges, buttons, or charts.
- Structure surfaces with hairlines and flat color, not shadows — a panel is a white box with a 1px border, and nothing more.
- Keep the type scale compact (15px body, 13px labels) — density is what makes a work tool feel professional.
- Use `{typography.code}` (monospace) for every HN, CID, and drug code — never render these in the body sans-serif.
- Mask CID by default everywhere; require an explicit action to reveal it in full.
- Add `font-variant-numeric: tabular-nums` on all numeric/code displays for column alignment.
- Provide visible focus rings on every interactive element for keyboard navigation.
- Use `200ms ease-out` for hover transitions, `150ms ease-out` for focus transitions.

### Don't
- Don't use red or green anywhere except the verdict band — not for buttons, not for links, not for hover states.
- Don't add decorative chrome beyond the logo: no icon chips/squares, no gradient or tinted hero areas, no marketing copy.
- Don't add hover animations, glow/focus rings, or elevation to flat surfaces — interaction feedback is a background color change, nothing more.
- Don't add a hero, marketing copy, or promotional banner — there is no acquisition funnel here.
- Don't apply pill-shaped (`{rounded.full}`) buttons — that shape is limited to tiny chips (OPD/IPD badges ≤ 20px tall), never buttons or panels.
- Don't animate the verdict band in a way that delays reading it (no fade-ins longer than ~120ms) — the whole point is instant legibility.
- Don't introduce a second accent color beyond `{colors.brand}` for chrome; new UI needs should be solved with weight/size/spacing, not new hues.
- Don't use bounce/elastic easing for any UI element.
- Don't hide keyboard focus indicators — this is a clinical tool used by professionals who may prefer keyboard navigation.

## Window & Responsive Behavior

AllerX is a fixed-purpose desktop app (Tauri), not a responsive web page, but the window is
still resizable by the user:

| Window width | Behavior |
|---|---|
| < 720px | **Stacked mode:** sidebar collapses to full-width, main canvas below it. Sidebar shows patient search + drug search stacked. Verdict + timeline below. |
| 720–959px | **Two-panel:** sidebar 300px, canvas fills rest. Verdict text drops to 20px. |
| ≥ 960px | **Two-panel standard:** sidebar 360px, canvas fills rest. Full verdict text. |

Minimum supported window size: 480×600px.

### Focus Management

- **Visible focus ring:** All interactive elements display `2px solid {colors.brand}` on `:focus-visible`.
- **Tab order:** Patient search → result list → patient bar (change button) → drug search → drug results → search button → timeline rows.
- **Escape key:** Closes any open dropdown/modal. Returns focus to last active input.

### Motion

- **Hover transitions:** `background-color 200ms ease-out` on buttons, rows.
- **Focus transitions:** `border-color 150ms ease-out` on inputs.
- **Reduced motion:** `@media (prefers-reduced-motion: reduce)` disables all transitions.
- **Never:** bounce/elastic easing, fade-ins longer than 120ms.

## Iteration Guide

1. Any new component must declare which existing token set it uses — no ad-hoc hex values in component CSS.
2. If a new state seems to need a new color, check first whether it can be expressed as a weight/border/spacing change instead — this system intentionally has very few named colors.
3. The verdict band's exclusivity rule (red/green nowhere else) is the one rule that should never be relaxed, even under UI pressure to "add a bit of color" elsewhere.
4. Default to `{typography.body}` for body text, `{typography.code}` for anything that is HN/CID/drug-code shaped.
5. Re-check every new screen against the "quiet except for the verdict" principle before shipping it.

## Known Gaps

- Dark mode is out of scope for M0–M6 (see `AGENTS.md` milestones); token names above should support a future dark variant without renaming, but dark values are not yet defined.
- The `status-dot` token (anticipated for multi-drug checking) is now used by the batch verdict bands — no live gap remains there.

## Future: Dark Mode Tokens

When dark mode is implemented, the following tokens need dark variants:

```
--canvas:        #FAFAFA  → #1a1d21
--canvas-raised: #FFFFFF  → #242830
--surface-muted: #F5F5F5  → #2d3239
--hairline:      #E0E0E0  → #3a4049
--ink:           #212121  → #e8eaed
--slate:         #616161  → #9aa0a6
--steel:         #9E9E9E  → #6b7280
--brand:         #D32F2F  → #FF6659 (lighter for dark bg contrast)
```
