# Changelog

All notable changes to AllerX are documented here. Versions follow the
discipline in [docs/ROADMAP.md](docs/ROADMAP.md) §Phase 8: v0.2.0 marked
Phase 1 completion, v0.3.0 marks the Phase 6 pilot start.

## [Unreleased]

## [0.3.1] — pilot connectivity

- Fixed: the pilot HOSxP instance has TLS disabled, so `ssl-mode=REQUIRED`
  made AllerX unable to connect. The connector defaults to `Preferred`
  now and reads `ALLERX_HOSXP_SSL_MODE` to raise the posture to
  `required`/`verify_ca`/`verify_identity` once the DBA enables TLS;
  `docs/deployment.md` A5 is an advisory checklist and the residual is
  recorded there.
- Security: the pilot hospital's name was removed from the public docs
  and history; README and deployment notes updated.
- Dependencies: `rustls` 0.23.43 → 0.23.45 (RUSTSEC-2026-0285);
  `encryptman` 0.2.2 → 0.3.0 and `encryptman-keyring` 0.1.2 → 0.1.3
  (fallible master-key generation, `#![forbid(unsafe_code)]`, formats
  unchanged, no migration).
- Documented the complete HOSxP table/column access surface with
  table-level and column-level GRANT templates (`docs/deployment.md`);
  a CI test fails the build when a statement touches an undocumented
  table.

## [0.3.0] — pilot start (Phase 6)

- `docs/deployment.md`: DBA sign-off checklist (SELECT-only user,
  charset, Debt-Ledger schema confirmations, index inspection) and the
  installer/release runbook with the two posture decisions — unsigned
  installer for v0.x, manual updates.
- `docs/pilot-notes.md`: 2–4 week pharmacy pilot protocol, machine
  hygiene checks, twelve-scenario script, feedback form, and an
  anonymized false-verdict report channel feeding Phase 7.
- CI: dispatch-only Windows job builds the NSIS/MSI installers so a
  release tag is never the first Windows bundle (Gap G9).

## [0.2.0] — verdict integrity through clinical depth (Phases 1–5)

Shipped across Phases 1–5 while the version stayed at 0.1.0; recorded
here together so the history is honest:

- **Phase 1 — Verdict Integrity:** three-state verdict
  (`Found`/`NotFound`/`Unresolved`) — an unresolvable drug term can no
  longer produce a false "ไม่พบประวัติ"; robust resolution
  (exact icode → exact name → ranked candidates), trade-name tier and
  drug-type filter behind runtime-tolerant query tiers, birth date in
  patient rows, honest truncation footer.
- **Phase 2 — Speed & Measurement:** PII-free in-memory timing buffer
  (`query_stats`/`clear_query_stats`), warm pool on startup, server-side
  SELECT timeout; budgets + DBA index checklist in
  `docs/perf-baseline.md`.
- **Phase 3 — Reliability:** live connection health (startup warm-up +
  30 s monitor) replacing the config-file-exists dot, degraded-mode
  warning banner, failure taxonomy mapped at the command boundary;
  kill-DB procedure in `docs/reliability-notes.md`.
- **Phase 4 — Frontend Testing & CI:** wasm-bindgen test runner with 30
  headless-Chrome tests over the clinical surface, API-layer tests,
  honest CI job naming, cargo audit on the release path, accessibility
  audit log (`docs/a11y-notes.md`).
- **Phase 5 — Clinical Depth:** multi-drug batch check (`check_drugs`,
  chip queue, compact per-drug verdict bands), patient detail modal
  (full CID reveal + recent-meds snapshot), print sheet, all documented
  in DESIGN.md and this roadmap.

[Unreleased]: https://github.com/suradet-ps/allerx/compare/v0.3.1...HEAD
[0.3.1]: https://github.com/suradet-ps/allerx/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/suradet-ps/allerx/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/suradet-ps/allerx/compare/v0.1.0...v0.2.0
