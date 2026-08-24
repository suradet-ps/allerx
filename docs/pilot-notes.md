# pilot-notes.md — AllerX pilot protocol (pharmacy department)

The Phase 6 pilot protocol (ROADMAP): 2–4 weeks of real use by the
pharmacy department, structured so the feedback answers exactly the
questions the roadmap needs — including the raw material for the Phase 7
validation. Install and machine prep come from
[deployment.md](deployment.md) Part B; this document starts once AllerX
runs on the pilot PC.

## Pilot shape

- **Where:** pharmacy department, one workstation, against the live
  HOSxP instance (read-only user from deployment.md A1).
- **Who:** 2–4 pharmacists who do allergy-related checks; one named
  contact for questions.
- **How long:** 2 weeks minimum, extend to 4 if sessions are sparse.
- **Success question:** would a pharmacist reach for AllerX *instead of*
  a raw HOSxP screen during an allergy assessment — and did any verdict
  ever look wrong?

## Machine hygiene (confirm during week 1, log results below)

| # | Check | Procedure | Pass? |
|---|---|---|---|
| 1 | Keyring works on the locked-down domain login | Save connection settings, close app, relaunch — settings must still be present. If the Thai keychain-unavailable message appears ([reliability-notes.md](reliability-notes.md)), stop and escalate to IT: the app will not store credentials on this profile | ☐ |
| 2 | Window sizing / DPI legible on the clinic display | Default window at the clinic's resolution/scaling; verdict band and timeline rows readable at arm's length; resizing down to the minimum keeps everything usable | ☐ |
| 3 | Crash / hang logging procedure agreed | On any crash or hang: note date+time, what was clicked, and app behavior — **never** the searched term or patient identity (AGENTS.md §2). Report via the same channel as the feedback form | ☐ |
| 4 | Printer reachable for the print sheet | One test print of a history sheet; layout intact | ☐ |

## Scenario script (walk each pharmacist through all of these in week 1)

1. Patient search by HN — expect instant list, select.
2. Patient search by CID — 13 digits auto-detected.
3. Patient search by name — duplicate names disambiguated by birth date
   in the result rows.
4. Drug by generic name (e.g. พาราเซตามอล) — resolved verdict with
   strength shown.
5. Drug by trade name only — same verdict quality expected.
6. Deliberately misspelled / non-formulary term — expect the amber
   "ไม่สามารถยืนยันได้" band with candidate suggestions, never a green/
   red answer.
7. Batch check 2–5 drugs at once — one verdict band per drug, merged
   timeline.
8. An OPD-heavy patient and an IPD patient (in-stay + take-home) —
   timeline shows both visit types.
9. A patient with long history — truncation footer appears honestly.
10. Detail modal ("ดูข้อมูลผู้ป่วย") — full CID reveal + recent-meds
    snapshot.
11. Print sheet for one verdict.
12. One deliberate network break mid-session (kill-DB scenario,
    [reliability-notes.md](reliability-notes.md)) — banner, no freeze,
    recovery without restart.

## Feedback form (paper or hospital-internal form — one per notable event)

Questions that matter, in this order:

1. **Did a verdict ever look wrong?** If yes: what was typed, what the
   band said, what manual chart review showed.
2. **Did the unverifiable state appear — and was it clear** that it means
   "term not found in the drug registry", not "no history"?
3. **Would you use this daily?** What is missing for that?
4. Speed: ever felt slow enough to skip using it?
5. Anything confusing, missing, or scary?
6. Was the print sheet useful in real consultations?
7. Was the "ยาที่ได้รับล่าสุด" snapshot useful? *(pilot feedback decides
   whether Phase 5's concurrent-meds feature stays)*

## False-verdict reports (the Phase 7 raw material)

Every question-1 "yes" becomes an anonymized row in a hospital-controlled
tracking doc:

| Date | Term typed (as typed) | Verdict shown | Strength shown? | Manual chart-review ground truth | Pharmacist initials |
|---|---|---|---|---|---|

Rules:

- **No patient identifiers on the shared form** — no name, HN, or CID.
  The root cause almost never needs them; if a case truly requires
  follow-up, the pharmacist records it privately and tells only the
  pilot contact it exists.
- The term typed and the ground truth are the payload — they feed the
  retrospective audit and root-cause analysis of
  [ROADMAP](ROADMAP.md) Phase 7 (`docs/validation-report.md`).
- A false "ไม่พบประวัติ" is a safety finding even when nobody was harmed;
  report it anyway.

## Weekly cadence

- 15-minute check-in with the pilot contact: collect forms, walk through
  anything odd, confirm the machine-hygiene table is complete.
- End of pilot: summarize into `docs/validation-report.md` inputs
  (Phase 7) — false-verdict rows verbatim, question-3/7 tallies, and a
  keep/drop recommendation per Phase 5 item.

## Pilot session log (fill in as sessions happen)

| Date | Pharmacists present | Scenarios covered | Notable events |
|---|---|---|---|
| — | — | — | — |
