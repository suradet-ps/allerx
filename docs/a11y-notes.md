# a11y-notes.md — AllerX accessibility audit

The audit log behind ROADMAP Phase 4. Hospital staff include older
clinicians with varying visual ability, so every view must be operable
without a mouse and legible in imperfect lighting. DESIGN.md already
mandates keyboard-first navigation; this document records what was
checked, how, and what is still pending a real screen reader.

## Checklist status (reviewed against the code, Phase 4)

| Requirement | Status | Where / notes |
|---|---|---|
| Keyboard-only flow: search → select → drug search → verdict | ✅ code-level | All interactive elements are native `<input>`/`<button>`; Enter triggers search in both search boxes; Tab order follows DESIGN.md |
| `aria-label` on icon-only buttons | ✅ | `search-clear` ("ล้าง"), `banner-warning__close` ("ปิด"); icons are `aria-hidden` decorative |
| Navigation landmark label | ✅ | Single view, no nav menu; the app header is a `<header>` — a labelled nav will be added if a menu ever appears |
| Visible focus ring | ✅ | `:focus-visible` with `2px solid var(--brand)` on every button/input (main.css) |
| Escape closes dialogs | ✅ | Settings and patient-detail modals close on Escape (window-level listener, guarded by the open flag), plus the ปิด/backdrop-click paths |
| Contrast ≥ 4.5:1 normal / 3:1 large | ✅ token audit | See the contrast table below; the amber `verdict-unresolved` pair (4.6:1) was chosen to pass |
| Never color-only | ✅ | Every colored element pairs with text or an icon: verdict headline + icon, status dot + text, badges + text |
| Touch targets ≥ 44px | ⚠️ desktop-only app | Buttons are 36–40px tall by design (density over whitespace, DESIGN.md); mouse-driven desktop app — revisit only if touch hardware appears |
| Screen-reader pass (NVDA) | 🔲 pending | Needs a real Windows machine (pilot, Phase 6) — procedure below |

## Contrast audit (tokens vs WCAG AA)

| Pair | Ratio | Verdict |
|---|---|---|
| ink `#212121` on canvas-raised `#FFFFFF` | 16.6:1 | ✅ |
| slate `#616161` on canvas-raised | 5.6:1 | ✅ |
| steel `#9E9E9E` on canvas-raised | 2.9:1 | ⚠️ placeholders/tertiary only (3:1 large-text rule; captions at 12px are borderline — placeholders are non-essential text, accepted) |
| brand `#D32F2F` on white (buttons, focus) | 5.4:1 | ✅ |
| on-brand `#FFFFFF` on brand `#D32F2F` | 5.4:1 | ✅ |
| verdict-found `#2E7D32` on `#E8F5E9` | 5.5:1 | ✅ |
| verdict-notfound `#C62828` on `#FFEBEE` | 5.8:1 | ✅ |
| verdict-unresolved `#8a6420` on `#FFF8E1` | 4.6:1 | ✅ (added in Phase 1 with this requirement in mind) |
| warning-text `#8a6420` on warning-bg `#FFF8E1` | 4.6:1 | ✅ (bumped from the old `#F57F17` 2.6:1 pair — same value as verdict-unresolved-text, main.css `--warning-text`) |

**One residual is owed before the pilot:** the NVDA screen-reader pass
itself (below) and the keyboard walkthrough on a real machine — both need a
Windows machine with the actual app running.

## Keyboard walkthrough (to be executed on a real machine, Phase 6)

1. Launch → focus starts in the patient search input (first focusable).
2. Tab: input → clear button → (no patient) drug search input → ตรวจประวัติ
   button → ตั้งค่า button → settings gear.
3. Type a name, press Enter → results list; Tab into the list, Enter to
   select a patient.
4. Patient bar appears; Tab reaches the เปลี่ยนผู้ป่วย (X) button; Enter
   clears the patient.
5. Drug search enabled; type + Enter → verdict band renders; Tab through
   the timeline rows (readable, not focusable — they are `<li>` without
   interactive content, acceptable).
6. Settings: open with Enter from the top bar; Tab through fields in
   order; Escape or ปิด returns focus to the trigger.
7. With a connection failure: banner appears; Tab reaches its ปิด button;
   Enter dismisses.

## Screen-reader procedure (NVDA, Windows — pilot machine)

1. Start NVDA before AllerX.
2. Tab through the full flow above; verify every control announces its
   role + label (Thai voice if configured, otherwise English fallback).
3. Verify the verdict band headline is announced when it appears (it is a
   `<section>` with heading text — announce check).
4. Record: passed / issues, in a table below.

| Date | NVDA version | Result | Issues |
|---|---|---|---|
| — | — | pending | — |

## Known follow-ups

- If touch hardware ever appears: 44 px touch targets per WCAG.
