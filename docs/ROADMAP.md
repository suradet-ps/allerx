# ROADMAP.md - AllerX

This roadmap describes what AllerX is, honestly, from reading its own code — and
where it should end up. It follows the architecture in [AGENTS.md](../AGENTS.md),
the conventions in [AGENTS-RUST.md](AGENTS-RUST.md), and the design system in
[DESIGN.md](DESIGN.md). It supersedes the milestone table in AGENTS.md §10 as
the forward-looking plan; the milestone table stays as the historical record of
what each milestone was scoped to.

> **What AllerX is.** A *quiet, precise, honest* lookup tool: a pharmacist
> standing mid-shift needs one answer in under a second — "has this patient
> ever received this drug, and when?" — before an allergy assessment. One
> window, two panels, one verdict band. Strictly read-only against HOSxP,
> nothing written, nothing cached, nothing logged that could identify a
> patient.
>
> **What AllerX is not.** Not an EHR, not an allergy registry, not a CDSS, not
> a reporting system, and never a writer into HOSxP. It answers one question
> and stops. Every feature in this roadmap must survive that test: *does it
> make the one answer faster, more correct, or more trustworthy?* Anything
> that turns AllerX into a general patient-history browser is listed under
> "Out of Scope" so the line is drawn on purpose.

Nothing here is called "done" on intent alone. The repo already has a real CI
pipeline (`.github/workflows/ci.yml`: fmt, clippy `-D warnings`, tests,
cargo-deny; `rust-safety.yml`: Miri on `search-core`; `test-build.yml`: Tauri
build; `publish-release.yml`: 3-platform tag builds); every phase's acceptance
is checked against it.

---

## Design Principles

Every feature in AllerX should reinforce one or more of these principles. When
a new feature is proposed, ask: "which principle does it serve, and does it
violate any other?"

1. **A verdict that never lies is the product.** The whole tool is one answer.
   A "พบประวัติ" that is wrong, or a "ไม่พบประวัติ" that is false, is not a
   bug — it is a patient-safety event waiting for an anaphylactic reaction or
   an unnecessary drug avoidance. Correctness and honesty of the verdict
   outrank every other feature. (This is why Phase 1 exists.)
2. **Read-only is a hard law, not a preference.** HOSxP is the system of
   record. AllerX never writes to it — not data, not schema, not "just a
   temp table." Enforcement is layered (grants, session mode, SQL guard,
   parameterized queries) and never weakened for convenience.
3. **Speed is a safety feature.** The pharmacist is standing, the patient is
   waiting. AGENTS.md's < 300 ms typical-query budget is a clinical
   requirement. A slow tool gets skipped; a skipped check is a missed reaction.
4. **Ambiguity must be visible, never silently resolved.** When the tool
   cannot be sure (drug name not in formulary, ambiguous matches), it must say
   so in the verdict — it must never collapse uncertainty into "no history."
5. **Deterministic and explainable.** Same inputs, same verdict, always. No
   probabilistic ranking, no black-box matching, no AI. Every verdict must be
   traceable to rows in HOSxP.
6. **Quiet UI, one loud element.** DESIGN.md's verdict-band rule — red and
   green exist nowhere else; the interface is paper-quiet so the answer is
   the only thing that shouts.
7. **Privacy by construction.** No PII in logs, CID masked by default, plain
   secrets never on disk. On shared pharmacy workstations, the patient's
   identity is a responsibility, not a convenience.
8. **Local-first.** Works on the hospital LAN with no internet. If HOSxP is
   down, the app says so clearly and degrades gracefully — it never pretends,
   and never holds the pharmacist hostage to a reconnect spinner.

---

## Safety Goals

AllerX exists to make one moment safer: the moment before an allergy
assessment, when a clinician asks "has this patient had this drug?"

The software should help clinicians:

- **Avoid false "no history" conclusions** — the drug may exist under a
  trade name, a variant spelling, a non-`1%` icode, or an IPD table this
  instance doesn't populate the expected way. A false negative here can send
  a patient into an avoidable allergic challenge.
- **Avoid false "history found" conclusions** — same name, different
  strength/presentation should not be silently reported as the same drug
  without the pharmacist seeing the strength.
- **Get the answer before the patient is discharged** — sub-second queries,
  no multi-step wizard, no scrolling through a sea of unrelated rows.
- **See the full timeline, not just the latest hit** — delayed-type reactions
  matter; the timeline is a clinical object, not decoration.
- **Trust the tool's privacy** — so clinicians will actually use it on a
  shared machine without hesitation.

It should **never** decide anything for the clinician. The tool reports what
HOSxP contains; the clinician interprets.

---

## Engineering Goals

- Business rules stay inside `search-core` — pure Rust, no I/O, fully
  testable against `MockRepository`. Any new search/matching/ordering logic
  goes there, not into SQL or components.
- All SQL lives in `hosxp-connector` as compile-time constants, guarded,
  parameterized, SELECT-only — and each statement has a test asserting it
  passes the read-only guard.
- UI contains no search logic — it renders verdicts. Components take signals,
  display states, call `api.rs`. Nothing else.
- Tauri commands remain thin adapters — validate input, call the repository,
  translate errors to Thai at this boundary and nowhere else.
- Every query is schema-verified against the live instance before it ships;
  unverified schema carries a `// SCHEMA-UNVERIFIED:` marker as an explicit
  debt ledger (see the Schema Debt Ledger below).
- Every phase's acceptance is run through the CI gate: `cargo fmt --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test
  --workspace`, `cargo-deny`, plus the WASM frontend build.
- Docs change in the same commit as the code they describe (AGENTS.md §11).

---

## Current State (verified against the repo, not assumed)

- **Version**: `0.3.0` (pilot-start bump, per Phase 8's version
  discipline) in the root workspace, `app/Cargo.toml`, and
  `tauri.conf.json`; history in `CHANGELOG.md`.
- **Stack**: Tauri 2 + Rust 2024 desktop shell; Leptos 0.8 CSR frontend
  (WASM via Trunk, `app/` is its own workspace); `crates/models` (domain
  types, no I/O), `crates/search-core` (pure logic + `HosxRepository` trait +
  `MockRepository`), `crates/hosxp-connector` (the *only* crate that touches
  MySQL).
- **Search flow implemented end to end**: one patient search box with
  HN/CID/name auto-detection (`detect_query_kind`, 250 ms debounce), drug
  autocomplete from `drugitems`, merged OPD+IPD history
  (`merge_drug_history`, most-recent-first), and the verdict band
  (`Pending` / `Found { records }` / `NotFound`).
- **Data model**: `PatientSummary`, `DrugHistoryRecord` (+ `VisitType`),
  `DrugItem`, `DrugSearchHit`. `trade_name` exists in the model but is always
  `None` in practice — the column is unverified.
- **Security model**: read-only enforced in four layers (SELECT-only DB
  user → `SET SESSION TRANSACTION READ ONLY` on every pooled connection →
  application-level SQL guard that accepts only a single `SELECT`/`WITH` →
  parameterized queries everywhere); AES-256-GCM encrypted connection
  settings at rest via `encryptman`/`encryptman-keyring` (key in the OS
  keychain); password is a `secrecy::SecretString` end to end; typed
  connection fields are zeroized in the settings modal; CID masked in the
  UI; no PII in logs.
- **Backend**: 6 Tauri commands in `src-tauri/src/commands.rs`
  (`connection_status`, `configure_connection`, `test_connection`,
  `search_patients`, `search_drugs`, `fetch_drug_history`). Thai error
  translation lives only here (`dev_log` / `map_repo_error`).
- **Frontend**: 9 components (`top_bar`, `patient_search`, `patient_bar`,
  `drug_search`, `settings_modal`, `verdict_band`, `timeline`,
  `patient_detail_modal`, `print_sheet`) + `api.rs` (the only
  webview→backend bridge) + shared signals in `state.rs`. Plain CSS
  implementing DESIGN.md tokens in `app/style/main.css`.
- **Tests** (counted from the repo): 65 Rust unit tests in the root
  workspace (8 `readonly_guard`, 6 `config`, 3 connector `error`, 1
  `queries` guard-assert, 3 `repository`, 6 `query_kind`, 4 `history`, 7
  `resolution`, 18 `mock`, 1 `error`/core, 4 `commands`, 4 `stats`), 7
  host-run component tests in `app` (`patient_bar` 4, `state` 3), and 30
  wasm tests in headless Chrome (24 component + 6 API). Integration tests
  in `hosxp-connector/tests` are gated behind `--features integration-tests`
  and never part of the default run.
- **CI** (4 workflows, all pinned Actions SHAs):
  - `ci.yml` — root workspace fmt/clippy/test; WASM frontend fmt/test/trunk
    build/clippy; cargo-deny (advisories + licenses).
  - `rust-safety.yml` — clippy job (named "Clippy Pedantic" but runs the
    standard `-D warnings`, not pedantic lints — see Gap G13) + Miri on
    `allerx-search-core`.
  - `test-build.yml` — full Tauri build on ubuntu-24.04 on every push/PR,
    plus a dispatch-only Windows job that builds and uploads the NSIS/MSI
    installers (the pre-tag smoke, Phase 6).
  - `publish-release.yml` — 3-platform (Windows, Linux, macOS ARM) tag
    builds via `tauri-apps/tauri-action`, `releaseDraft: false`.
- **Milestone status** (historical record per AGENTS.md §10):

| Milestone | Scope | Status |
|---|---|---|
| M0 | Workspace, Tauri 2 + Leptos 0.8 CSR scaffold, `SELECT 1` smoke test | ✅ Done |
| M1 | Read-only guard + connection pool; encrypted config | ✅ Done (schema verification items remain — see Debt Ledger) |
| M2 | Patient search (HN/CID/name) end to end | ✅ Done |
| M3 | Drug search + autocomplete from `drugitems` | ✅ Done |
| M4 | Medication history (OPD+IPD merged) + timeline UI | ✅ Done |
| M5 | Performance tuning (debounce, index verification, query timing) | 🔶 Partial — debounce + result limits exist; no timing instrumentation, no index verification → Phase 2 |
| M6 | UI polish (loading/error states, DB-unreachable handling, CID masking) | 🔶 Partial — masking + error messages done; no live connection state, no degraded-mode banner → Phase 3 |
| M7 | Packaging (Tauri bundle), internal pharmacy pilot | 🔶 Release workflow exists, not yet exercised end-to-end; pilot not started → Phase 6 (docs + CI smoke shipped; execution pending) |

---

## Gaps found while reading the repo (these shape the phases below)

1. **G1 — The verdict can lie, in both directions (safety-critical).**
   `fetch_drug_history` resolves the typed drug to an icode (exact icode,
   then *exact* name); if resolution fails it returns an empty list, and the
   frontend renders "ไม่พบประวัติ" (`drug_search.rs` → `VerdictState::NotFound`).
   The pharmacist cannot distinguish *"this patient never received this
   drug"* from *"this name isn't in drugitems — typo, trade-name variant, or
   not in the formulary."* A generic-name search that fails to resolve
   produces a false negative verdict. Also, exact-name-only resolution breaks
   on the mildest variant ("พาราเซตามอล 500" vs "พาราเซตามอล"). **This is
   the single most dangerous gap in the current tool.** (Phase 1)
2. **G2 — "เชื่อมต่อแล้ว" means "a config file exists", not "HOSxP is
   reachable."** The top-bar status dot derives from `state.configured`,
   which is set by `connection_status()` — an `fs::exists` check on the
   encrypted settings file (`commands.rs:96`). A dead database mid-shift
   shows a green dot until the first query fails with a generic error. The
   `.banner-warning` CSS class exists and nothing uses it. (Phase 3)
3. **G3 — Autocomplete and history may include/be poisoned by non-drug
   rows.** The `drugitems` queries have no drug-type filter
   (`istype`/`item_type` from AGENTS.md §6.2 is unverified), so on an
   instance whose `drugitems` holds supplies, autocomplete can offer
   non-drug items. And `trade_name` is never selected — generic→trade
   matching is impossible today. (Phase 1)
4. **G4 — Patient result rows show name + HN only.** DESIGN.md §7.1
   specifies HN / name / birth date — birth date disambiguates duplicate
   names and is already in the model, but not rendered. (Phase 1)
5. **G5 — Timeline truncation is silent.** Each history source is
   `LIMIT 200`; the footer renders "ทั้งหมด N รายการ" with N = fetched
   count, so a truncated result is presented as complete. A pharmacist may
   believe they have seen the whole history when the oldest (or newest)
   rows were cut. (Phase 1)
6. **G6 — No query-timing instrumentation.** Only `test_connection`
   measures latency. The < 300 ms budget cannot be verified, monitored, or
   regression-tested. No cold-start measurement exists. (Phase 2)
7. **G7 — No frontend test infrastructure.** Only 4 host-run component
   tests (`patient_bar.rs`). The verdict state machine — the product — has
   zero tests; neither does the API layer or the debounce logic. No WASM
   test runner, no e2e. (Phase 4)
8. **G8 — HN pattern and remaining schema unverified.** `detect_query_kind`'s
   "5–10 digits = HN" rule is a documented default, not confirmed against
   Sarabos Hospital (AGENTS.md §6). `// SCHEMA-UNVERIFIED` markers remain on
   `drugitems.name`/`strength` and the whole `iptitemrece` query. Charset
   (TIS-620 vs UTF-8) is unconfirmed. See the Schema Debt Ledger. (Phases 1, 5, 6)
9. **G9 — No deployment/pilot materials.** No DBA checklist (SELECT-only
   user creation, grant template), no Windows installer verification
   (`test-build.yml` is ubuntu-only; the Windows artifact has never been
   produced by CI), no pilot protocol, no feedback mechanism. (Phase 6)
10. **G10 — Docs debt.** No `architecture.md`, `database.md` (schema
    verification log), `security.md`, `perf-baseline.md`, or validation
    report. AGENTS.md's "To confirm with DBA" list has no living home.
    (Throughout — see Documentation Plan)
11. **G11 — No cross-check against documented allergies.** HOSxP instances
    commonly hold allergy/adverse-reaction records (table name varies —
    verify). AllerX never reads them. A patient with a *documented allergy*
    to the searched drug plus a "ไม่พบประวัติ" verdict is the exact
    scenario the tool exists to prevent, and today it would silently pass.
    (Phase 7)
12. **G12 — No detail view / full CID reveal.** DESIGN.md says CID is shown
    in full only on a detail view; no detail view exists. Minor today,
    but a verdict-anchored detail view is the natural home for Phase 5/7
    additions. (Phase 5)
13. **G13 — CI naming/scope drift.** The `rust-safety.yml` job is named
    "Clippy Pedantic" but runs the standard `-D warnings` lint set (no
    `-W clippy::pedantic`), and Miri covers only `search-core`. Honest
    naming and deliberate scope beats aspirational names. (Phase 4)
14. **G14 — No multi-drug checking.** DESIGN.md's `status-dot` token "exists
    partly in anticipation of" batch checks but has no use. A pharmacist
    often checks 2–5 drugs per patient per assessment; today that is N
    round trips. (Phase 5, scope-gated)
15. **G15 — No measurement of "was the answer wrong."** No process for
    recording false verdicts found in the field, which is the raw material
    for Phase 7 validation. (Phase 6/7)

---

## Phases

### Phase 1: Verdict Integrity — the search must never lie

The thing AllerX sells is one answer. Phase 1 makes that answer honest.
Everything else can wait; this cannot. Corresponds to the unresolved
correctness residue of M2–M4 plus G1, G3, G4, G5.

**Status: COMPLETE** (implementation) — pending live-instance schema
confirmation for the trade-name column and the drug-type filter (tracked in
`docs/database.md`; `SCHEMA-UNVERIFIED` markers stay until then).

- [x] **Three-state verdict.** Extend the verdict model from
  `Found / NotFound / Pending` to:
  - **พบประวัติ** — drug resolved to icode(s), dispensing rows found.
  - **ไม่พบประวัติ** — drug resolved to icode(s), no dispensing rows found.
  - **ไม่สามารถยืนยันได้ (ไม่พบยานี้ในทะเบียนยา)** — the drug term could not
    be resolved to any `drugitems` entry. Distinct visual treatment
    (verdict-pending palette is NOT acceptable — that implies waiting; use
    the warning/amber system state, which DESIGN.md already reserves for
    system states and never for clinical meaning, so add a documented
    `verdict-unverifiable` token pair and update DESIGN.md).
  - The frontend must never render "ไม่พบประวัติ" when resolution failed.
    `fetch_drug_history` gains a discriminated result type (e.g.
    `HistoryVerdict { Resolved { records }, Unresolved }` in `models`),
    `search-core` gets a pure `verdict_from_history` mapping, and
    `drug_search.rs` routes on it.
- [x] **Robust drug resolution.** Resolution order becomes: exact icode →
  exact name → *prefix/contains shortlist* (top ~10) presented as a
  disambiguation choice when more than one candidate exists (or when the
  typed term is not an exact hit). Never resolve silently when ambiguous,
  and never collapse to empty. All resolution logic is a pure function in
  `search-core` (unit-testable against `MockRepository`); `hosxp-connector`
  only supplies the candidate rows.
- [x] **Trade-name search.** Verify the trade-name column on the live
  instance (candidate: `drugitems.trade_name` — confirm, remove
  `SCHEMA-UNVERIFIED`); select it in `search_drugs` and `HISTORY_*` so the
  model's `trade_name` field is actually populated, and resolve on generic
  OR trade name. A pharmacist who only knows the trade name must get the
  same verdict quality.
- [x] **Drug-type filter.** Verify which of `istype`/`item_type` this
  instance uses (AGENTS.md §6.2); apply the drug-category filter to
  `search_drugs` (autocomplete) and confirm the icode-category assumption
  (`'1%'`) per instance. Document the finding in `docs/database.md`.
- [x] **Birth date in patient rows.** Render `birth_date` in
  `search-result-row` (DESIGN.md §7.1 already promises it).
- [x] **Truncation honesty.** Change the timeline footer to
  "แสดง 200 รายการแรก — มีประวัติมากกว่านี้" when any source hit its
  `LIMIT 200` (track truncation through `HistoryVerdict`), or raise the
  limit with a count query — pick the cheapest correct option after
  measuring (Phase 2).
- [x] **New-query guard tests.** Every new/changed SQL statement gets a
  read-only-guard assertion test (AGENTS.md §12), and every new pure
  function in `search-core` gets mock-based unit tests, including the
  "resolution failed ⇒ never NotFound" invariant.

**Acceptance:** no code path produces a "ไม่พบประวัติ" verdict for an
unresolvable drug term; a generic-name/trade-name search resolves or offers
disambiguation instead of failing silently; autocomplete shows only drug
items; the CI gate passes with the new tests; every schema finding is
recorded in `docs/database.md` — the `SCHEMA-UNVERIFIED` markers stay until
the live-instance confirmation in Phase 6 (the tiered-query fallback keeps
the app working either way).

### Phase 2: Speed & Measurement (completes M5)

A budget you cannot measure is a wish. AGENTS.md's < 300 ms target needs
instrumentation, a baseline, and a debt list for the DBA.

**Status: COMPLETE** (implementation) — the ring buffer, warm pool, and
SELECT timeout are shipped; `docs/perf-baseline.md` defines the protocol,
budgets, and the DBA index checklist. Actual numbers are collected at the
staging/pilot sessions (Phase 6) — this repo has no live database to
measure against, and no fake numbers go into the baseline.

- [x] **Per-query latency instrumentation.** A PII-free in-memory ring
  buffer of `(command, source, elapsed_ms, outcome)` in the connector (or
  `src-tauri` state), surfaced via a `query_stats` command (dev/ops only —
  never rendered in the normal UI, never persisted, never containing
  parameter values). This is logging of timing, not patient data, so it
  respects AGENTS.md §2 as written — but keep it in-memory to stay
  conservative.
- [x] **Cold-start and page-load baseline.** Measure and document in
  `docs/perf-baseline.md`: app launch → interactive, patient search
  response, drug autocomplete response, full history lookup (OPD+IPD),
  verdict render time. Reference hardware: mid-range clinic PC (i5, 8 GB,
  HDD), per Phase 9 of the model roadmap this repo learns from.
- [x] **Index verification.** Walk the AGENTS.md §6 "To confirm with DBA"
  list into a written checklist: indexes on `patient(hn)`, `patient(cid)`,
  `patient(fname)/lname`, `drugitems(icode)`, `drugitems(name)`,
  `opitemrece(hn, icode, vstdate)`, `opitemrece.an`, `iptitemrece(an)`.
  Missing indexes → documented DBA request (safe under read-only).
- [x] **Warm pool on startup.** On app launch, acquire the pool and ping
  (feeds Phase 3's connection honesty); the first real query should not pay
  connect latency.
- [x] **Tuning experiments.** Pool size (current 5), acquire timeout
  (current 5 s), per-query statement timeout if sqlx supports it for the
  MySQL backend — record results in `perf-baseline.md`.

**Acceptance:** `docs/perf-baseline.md` exists with numbers on reference
hardware; typical queries are < 300 ms or the DBA debt list names exactly
what index blocks that; CI is unchanged (instrumentation is PII-free and
in-memory).

### Phase 3: Reliability & Connection Honesty (completes M6)

G2 is a trust bug: a green dot that means "a file exists." Phase 3 makes the
app honest about its own health and graceful in failure.

**Status: COMPLETE** (implementation) — live health, banner, and failure
taxonomy shipped; the kill-DB procedure is documented in
`docs/reliability-notes.md` and will be executed on the pilot machine
(Phase 6) before sign-off.

- [x] **Live health state.** Replace the config-file-exists status with a
  real health signal: on startup (and after Phase 2's warm ping), and on a
  slow timer (e.g. every 30 s while idle), run `ping()`; the status dot and
  text reflect actual reachability ("เชื่อมต่อ HOSxP แล้ว" / "HOSxP ไม่พร้อมใช้งาน").
  The health check reuses the existing pool and never interrupts a query in
  flight.
- [x] **Degraded-mode banner.** When a query fails with
  `RepositoryError::Connection`, show the long-reserved `.banner-warning`
  ("ไม่สามารถเชื่อมต่อ HOSxP ได้ — ตรวจสอบเครือข่าย") above the search
  sections (or on the main canvas), keep the UI fully interactive, and
  auto-retry the next query as usual. No raw error text ever reaches the
  UI (already true — keep it true).
- [x] **Preflight on launch.** Attempt a warm connection at startup with
  bounded retries/backoff; if the stored config is missing or corrupt, the
  existing settings-modal first-run flow already handles it — extend it to
  also handle *keyring-unavailable* (headless/CI/locked-down workstation)
  with a clear Thai message instead of a generic failure.
- [x] **Failure taxonomy in the UI.** Three user-visible failure classes,
  mapped in `commands.rs`: not configured (settings modal), unreachable
  (banner + retry), query failed (generic per-action error, as today).
  Add tests for each translation in `commands.rs`.
- [x] **Manual kill-DB scenario.** Document a test procedure (stop MySQL,
  launch app, search, restart MySQL, search again) in `docs/reliability-notes.md`
  and run it on the pilot machine during Phase 6.

**Acceptance:** the status dot is driven by a real ping, not file existence;
a dead DB produces a clear Thai banner and a usable app (no freeze, no raw
errors); recovery after DB restart requires no app restart; the kill-DB
scenario is documented and passed manually.

### Phase 4: Frontend Testing & CI Hardening (G7, G13)

The Rust core is well tested; the verdict state machine — the product — is
not.

**Status: COMPLETE** (implementation) — wasm test runner + 30 tests in
headless Chrome (24 component + 6 API), honest CI job names, cargo audit on
the release path, and the a11y audit log. The NVDA pass and the on-machine
keyboard walkthrough run during Phase 6 (documented procedures in
`docs/a11y-notes.md`).

- [x] **WASM test runner.** Add `wasm-bindgen-test` to `app/` with a CI job
  (Chromium headless via `wasm-pack test --headless` or the equivalent
  pinned runner). Host-run `cargo test -p allerx-app` stays as-is for pure
  logic.
- [x] **Component tests for the clinical surface:**
  - `verdict_band.rs` — renders all four verdict states (Pending, Found,
    NotFound, Unresolved from Phase 1) with correct class + text; latest
    record detail line correct; never two verdicts.
  - `patient_bar.rs` — existing masking tests, plus DOB/sex edge cases.
  - `timeline.rs` — truncation footer from Phase 1; row rendering.
  - `drug_search.rs` — resolution-failure routing (Phase 1), disabled
    states, debounce wiring via injected timer.
  - `patient_search.rs` — debounce, kind detection end-to-end via a fake
    `invoke`.
- [x] **API-layer tests.** `api.rs` with an injectable fake `invoke`
  (the extern `__TAURI_INTERNALS__` binding becomes an injected function):
  arg-shape correctness, error-string passthrough, deserialization.
- [x] **CI: honest naming + new jobs.** Rename the `rust-safety.yml` job
  (`clippy-pedantic` → `clippy-workspace`) or actually enable
  `-W clippy::pedantic` (prefer honesty: keep the standard lints, rename the
  job); add the WASM test job to `ci.yml`; add `cargo audit` to the release
  workflow (deny covers advisories, but a dedicated audit on tag pushes is
  cheap and loud).
- [x] **Accessibility audit.** Per DESIGN.md's keyboard-first principle,
  document in `docs/a11y-notes.md`: keyboard-only walkthrough of the full
  flow (search → select → drug search → verdict), `aria-label`s on all
  interactive elements, visible `:focus-visible` rings (already styled),
  contrast check (WCAG AA), and one screen-reader pass (NVDA) with results
  logged. Escape-closes behavior for dropdowns and the settings modal.

**Acceptance:** the WASM test job runs in CI and passes; the four
must-not-break components have tests; the audit checklist is complete with
logged results; no existing feature regresses without a test catching it.

### Phase 5: Clinical Depth — scope-checked additions (G12, G14)

Each item below must pass the principle test *before* implementation: "does
it make the one answer faster, more correct, or more trustworthy?" and "does
it violate read-only, privacy, or quiet-UI?" Items that fail the test go to
"Out of Scope" instead. Doc-first per AGENTS.md §11 — DESIGN.md and this
roadmap are updated in the same commit as the code.

**Status: COMPLETE** — all four items implemented, tested (30 wasm tests +
search-core/connector unit tests), and documented. The concurrent-meds
snapshot ships behind the detail view; whether the pilot clinic finds it
useful is a Phase 6/7 evaluation, not an engineering gate.

- [x] **Multi-drug batch check (proposal — requires scope decision).**
  Check 2–5 drugs in one search round trip; verdict band per drug, with the
  `status-dot` token DESIGN.md anticipated. The single-question core stays:
  it is the same question asked N times in one pass. If scoped in, the
  repository trait gains `fetch_drug_history_batch` (concurrent fan-out in
  the connector, pure aggregation in `search-core`) and the UI keeps the
  two-panel layout with a multi-verdict list on the canvas.
  **Shipped as:** `check_drugs` (trait default sequential; connector
  overrides with a `JoinSet` fan-out — one task per drug, each with its own
  OPD+IPD concurrency); chip queue in the sidebar (Enter/suggestion adds,
  dedupe by icode/label, removable, ล้างทั้งหมด); `VerdictState::Results`
  renders one full-size band for a single drug and stacked term-labelled
  compact bands for a batch; unresolved bands embed candidate buttons that
  re-queue the drug; timelines merge across drugs newest-first.
- [x] **Verdict-anchored patient detail view.** The DESIGN.md-mandated
  "detail view" that shows the full CID, and is the future home of Phase 7's
  allergy cross-check. Opened from the patient bar ("ดูข้อมูลผู้ป่วย"),
  keeps the sidebar for the search flow. Scope-limited: demographics from
  the already-fetched `PatientSummary` + full CID reveal + link back. No
  new HOSxP tables yet.
  **Shipped as:** `PatientDetailModal` — full CID (the only place it is
  unmasked), demographics grid, and the recent-meds snapshot loaded on open.
- [x] **Print/export drug history (proposal — requires scope decision).**
  DESIGN.md explicitly lists "print/export styling" as an undesignated gap.
  A printable patient+history sheet (Thai) is the natural artifact a
  pharmacist attaches to a consultation note. Requires DESIGN.md print
  tokens first. Stays read-only and ephemeral (no persisted files).
  **Shipped as:** `PrintSheet` (hidden on screen, sole content in
  `@media print`; print tokens documented in DESIGN.md) + พิมพ์ประวัติ
  button calling `window.print()`. No files are written.
- [x] **Concurrent-medications snapshot (proposal — requires clinical
  sign-off).** A read-only "ยาที่ได้รับล่าสุด" list (recent
  `opitemrece` rows for the HN, filtered to drugs, most recent visit
  window) to give context for cross-reactivity thinking. This is the item
  most at risk of scope creep — the decision belongs to the pilot clinic,
  not to engineering (Phase 6 gates it).
  **Shipped as:** `fetch_concurrent_medications` — last-30-days dispensing
  deduped per icode (GROUP BY), drug category only (`icode LIKE '1%'` per
  AGENTS.md §6.2, SCHEMA-UNVERIFIED pending live confirmation), trade-name
  tier with fallback; shown inside the detail modal. Pilot feedback decides
  whether it stays.

**Acceptance:** each accepted item has a scoped DESIGN.md + roadmap update,
implements in the correct layer (pure logic in `search-core`, SQL in
`hosxp-connector`), ships with tests, and passes the CI gate; rejected items
are recorded in "Out of Scope" with the reason.

### Phase 6: Deployment & Pilot (completes M7; G9)

The tool is only worth anything if it runs on the clinic PC with a real
HOSxP user.

**Status: DOCS COMPLETE** — `docs/deployment.md` and `docs/pilot-notes.md`
are shipped, both posture decisions are made (unsigned installer for v0.x,
manual updates), CI now produces Windows installers on demand (Gap G9's
"never built" is closed), and v0.3.0 is bumped with a CHANGELOG. The rest
executes off-repo by necessity: the smoke-build dispatch, the `v*` tag
exercise, install verification, keyring/DPI confirmation, and the pilot
itself — checked off here only after they happen on the real machines.

- [x] **DBA deployment checklist** (`docs/deployment.md`): SQL template for
  the dedicated SELECT-only user (`GRANT SELECT ON <schema>.* TO
  'allerx_ro'@...` — nothing else), charset verification (TIS-620 vs UTF-8,
  AGENTS.md §6), confirmation of the remaining schema items from the Debt
  Ledger, and a documented index request list from Phase 2.
- [ ] **Windows installer verification** *(runbook + decisions shipped;
  execution pending)*. Exercise `publish-release.yml`
  on a real `v*` tag; verify the NSIS/MSI artifact installs on the pilot
  PC (Windows is the clinic reality). Decided and documented in
  `docs/deployment.md`: code-signing posture (**unsigned for the pilot**)
  and auto-update (**manual installs for v0.x** — hospital IT controls
  rollout).
- [x] **Pilot protocol** (`docs/pilot-notes.md`): 2–4 weeks with the
  pharmacy department; defined scenarios (search by HN, CID, name; trade
  vs generic names; OPD + IPD patients); a feedback form with exactly the
  questions that matter: "did a verdict ever look wrong?", "did the
  unverifiable state appear, and was it clear?", "would you use this daily?";
  and a false-verdict report channel (feeds Phase 7).
- [ ] **Pilot machine hygiene** *(procedure documented in
  `docs/pilot-notes.md`; confirmation needs the locked-down clinic PC)*.
  Confirm keychain access works on the hospital's locked-down Windows login
  (keyring is a Phase 3 dependency);
  confirm window sizing/DPI on the clinic's display; log any crashes with
  timestamps (PII-free).

**Acceptance:** a signed (or documented unsigned) Windows installer exists
and installs on the pilot machine; `docs/deployment.md` and
`docs/pilot-notes.md` exist; the DBA sign-off list is fully checked;
pilot starts.

### Phase 7: Clinical Validation (G11, G15)

Unit tests prove the code does what it says. Validation proves what it says
matters in the clinic.

- [ ] **False-verdict capture during pilot.** Every "verdict looked wrong"
  report from Phase 6 becomes a row in a private, hospital-controlled
  tracking doc (anonymized): searched term, verdict shown, what the
  pharmacist found by manual chart review. This is the raw material for
  root-cause analysis.
- [ ] **Retrospective verdict audit.** With pharmacy cooperation, take
  50–100 patients with known drug-allergy records; run the tool's flow for
  each and compare against manual chart review. Measure:
  - false "พบประวัติ" rate (must be 0),
  - false "ไม่พบประวัติ" rate (must be 0 — every miss gets a root cause:
    name variant, trade name, non-`1%` icode, IPD table variation, etc.),
  - unverifiable rate (should shrink as Phase 1 matures).
- [ ] **Documented-allergy cross-check (proposal — requires schema
  verification + clinical sign-off).** Verify HOSxP's allergy table on the
  live instance; if present and reliable, surface a read-only note in the
  verdict area: "บันทึกแพ้ยานี้ใน HOSxP" when a documented allergy matches
  the searched drug — and, critically, show it even when dispensing history
  is absent. This directly closes G11. Pure logic in `search-core`, new
  guarded SELECT in `hosxp-connector`, verdict-band variant in the UI.
- [ ] **Validation report** (`docs/validation-report.md`): rates above,
  root-cause analysis of every false verdict, DBA findings, pilot feedback
  summary, and a go/no-go recommendation for v1.0.0.

**Acceptance:** zero false "พบประวัติ" and zero false "ไม่พบประวัติ" on the
validated sample (or every miss has a documented root cause and a fix
landed); the allergy cross-check, if accepted, ships through the normal
CI gate; the validation report is published and reviewed with the pilot
clinic.

### Phase 8: 1.0 Hardening & the Quiet Long Tail

The last mile before v1.0.0 and the modest, honest post-1.0 surface.

- [ ] **Dark mode.** DESIGN.md already defines the dark tokens; implement
  behind the OS setting, verify verdict contrast in dark, ship with an
  a11y note. (Low priority — pilot feedback decides.)
- [ ] **Offline formulary fallback (proposal — scope decision).** A
  pre-loaded, read-only snapshot of `drugitems` for resolution when HOSxP
  is down would keep the "unverifiable vs not-found" distinction usable —
  but AGENTS.md's "no persistent patient data" rule applies to *patient*
  data, not the drug catalog; still, this needs an explicit AGENTS.md §11
  doc-first decision before any code.
- [ ] **Version discipline.** v0.1.0 → v0.2.0 at Phase 1 completion →
  v0.3.0 at Phase 6 pilot start → v1.0.0 only after Phase 7 sign-off.
  Each bump updates `CHANGELOG.md` (new), `cargo-deny` review, and a real
  `v*` tag through the existing publish workflow.
- [ ] **Documentation completion.** `docs/architecture.md` (module
  dependency diagram, data flow), `docs/security.md` (threat model:
  shared-workstation threats, credential-at-rest, read-only enforcement —
  largely written in AGENTS.md §5/§9, formalize it), and the docs table
  below closed out.

**Acceptance:** v1.0.0 is tagged only after Phase 7's report; the docs table
has no missing cells; every "proposal" item above is either scoped in with
code+docs or written into "Out of Scope" with its reason.

---

## How the phases relate

```
Phase 1 (Verdict Integrity)   -- foundation, do first: the tool must not lie
        |
        +---> Phase 2 (Speed & Measurement)   -- needs Phase 1's resolution work measured
        |
        +---> Phase 3 (Reliability)           -- needs Phase 2's warm pool/ping
        |
        +---> Phase 4 (Frontend Testing)      -- parallel track, any time (do before 1.0)
        |
        +---> Phase 5 (Clinical Depth)        -- each item gated by the principle test
        |
        v
Phase 6 (Deployment & Pilot)  -- needs Phases 1–4 stable; gates Phases 5 & 7
        |
        v
Phase 7 (Clinical Validation) -- needs the pilot running + Phase 5 items that add data
        |
        v
Phase 8 (1.0 Hardening)       -- v1.0.0 gate
```

Phase 1 comes first on purpose: a verdict that silently lies is the only
feature that can actively harm. Phases 2–4 make the tool measurable, honest
about its own health, and testable — the engineering backbone for everything
after. Phase 6 is the moment of truth (real HOSxP user, real clinic PC, real
pharmacists); Phase 7 is the gate before v1.0.0.

---

## Out of Scope (drawn on purpose, to stay a focused lookup tool)

Each of these is valuable *for a different product*. AllerX stays the single-
question tool:

- **Any write to HOSxP** — data, schema, or otherwise. Hard rule, no
  exceptions, no "sync back", no "just a temp table". If a task seems to
  require it, stop and flag it (AGENTS.md §2).
- **Allergy registry / registration** — recording allergies is the allergy
  clinic's job in its own system. AllerX may *read* documented allergies
  (Phase 7 proposal) but never writes them.
- **EHR replacement** — HOSxP is the system of record; demographics,
  diagnoses, billing stay there.
- **Real-time CDSS at prescribing time** — a separate system; AGENTS.md's
  non-goals say so. AllerX is a pre-assessment lookup, not an alert engine.
- **Patient-facing app / patient portal** — there is no patient story in a
  read-only lookup tool.
- **AI/LLM interpretation of histories** — hallucination risk in a clinical
  path is disqualifying; determinism is a principle, not a preference.
- **Persistent patient-data caching** — AGENTS.md non-goal; the offline
  formulary snapshot (Phase 8 proposal) is explicitly *not* patient data.
- **Web/SaaS version** — desktop-only; hospital LAN deployment model.
- **Multi-language (i18n)** — Thai-only, per clinic reality.
- **Billing, insurance, controlled-substance tracking** — nothing to do with
  the question.
- **General patient-history browser** — AllerX shows *one drug's* timeline,
  not the patient's whole record. The concurrent-medications snapshot
  (Phase 5) is the one item on the boundary — it is listed as a proposal
  precisely because it must earn its place.

---

## Schema Debt Ledger (living document — the `SCHEMA-UNVERIFIED` tracker)

From AGENTS.md §6 and the `// SCHEMA-UNVERIFIED:` markers in the repo.
Everything here is "likely correct, confirm against the live instance" until
checked; once confirmed, the marker is removed and the finding lands in
`docs/database.md`.

| Item | Where it lives | Status |
|---|---|---|
| `drugitems.trade_name` (matching + display) | `queries.rs` DRUG_SEARCH_*/HISTORY_* tiers, Phase 1 | ❌ unverified (runtime fallback in place) |
| `drugitems` drug-type field (`istype` vs `item_type`) | AGENTS.md §6.2, Phase 1 typed tier | ❌ unverified (runtime fallback in place) |
| `iptitemrece` (table name), `idate`/`itime`, `ipt.hn` | `queries.rs` HISTORY_IPD_STAY | ❌ unverified (missing-table tolerated at runtime) |
| `kskdepartment.depcode` | `queries.rs` HISTORY_OPD/IPD | ❌ unverified |
| `opitemrece` IPD take-home branch (`an IS NOT NULL`) | `queries.rs` HISTORY_IPD_TAKEHOME | ❌ unverified |
| Sarabos HN pattern (drives `detect_query_kind`) | `query_kind.rs` | ❌ unverified |
| Database charset (TIS-620 vs UTF-8) | AGENTS.md §6 | ❌ unverified |
| `patient.birthday` (vs `birthdate`) | `queries.rs` | ✅ confirmed on live instance |
| `opitemrece.dep_code` (vs `depcode`) | `queries.rs` | ✅ confirmed on live instance |
| HOSxP allergy/adverse-reaction table (Phase 7) | — | ❌ not yet located |

---

## Documentation Plan

| Document | Content | Status |
|---|---|---|
| `AGENTS.md` | Architecture, hard rules, schema notes, search flow | ✅ exists |
| `docs/DESIGN.md` | Design system, tokens, components | ✅ exists (needs verdict-unverifiable + print tokens when Phases 1/5 land) |
| `docs/AGENTS-RUST.md` | Rust workspace rules + project overrides | ✅ exists |
| `docs/ROADMAP.md` | This document | ✅ now |
| `docs/database.md` | Schema verification log, query patterns, DBA findings | ✅ exists |
| `docs/perf-baseline.md` | Latency measurements, budgets, regression thresholds | ✅ exists (protocol + budgets; numbers filled at staging/pilot) |
| `docs/reliability-notes.md` | Kill-DB scenario, recovery procedure, health-check notes | ✅ exists |
| `docs/a11y-notes.md` | Accessibility audit results, NVDA log | ✅ exists (audit + contrast done; NVDA pass scheduled for Phase 6) |
| `docs/deployment.md` | DBA checklist, installer, keyring/DPI notes | ✅ exists (execution pending pilot machine) |
| `docs/pilot-notes.md` | Pilot protocol, feedback form, field reports | ✅ exists (protocol ready; pilot not started) |
| `docs/validation-report.md` | False-verdict rates, root causes, go/no-go | Phase 7 |
| `docs/architecture.md` | Module dependencies, data flow diagrams | Phase 8 |
| `docs/security.md` | Threat model, credential-at-rest, shared-workstation posture | Phase 8 |
| `CHANGELOG.md` | Version history | ✅ exists (started at v0.3.0) |

---

## Future / Ecosystem (post-1.0, if they stay focused)

- **Allergy cross-check** (Phase 7 proposal) matures into a standard part of
  the verdict surface: "พบประวัติ" / "ไม่พบประวัติ" / "ไม่พบประวัติ แต่มี
  บันทึกแพ้ยานี้" — the third state being the highest-value line in the tool.
- **Same-class awareness**: when the searched drug is unresolvable, check
  whether the *class* is present in the patient's history (requires ATC or
  equivalent column — verify first; likely instance-dependent).
- **Multi-site deployment**: a schema-profile per hospital (HOSxP column
  variations are per-instance, not global) so one binary serves multiple
  hospitals without forked queries.
- **Hospital-launch pack**: co-branded build with the hospital logo, icon
  variants, and an IT handover page — a packaging nicety, not a feature.
- **Integration with the pharmacy dispensing screen** is explicitly NOT on
  this list — that is a different product's job.

---

*This roadmap is a living document. It is rewritten when the code is
rewritten, and its claims are only as good as the last `cargo test`. If you
find it lying — about the current state or about what the next phase should
be — fix it in the same commit as the fix it describes.*
