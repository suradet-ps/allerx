# AllerX

<p align="center">
  <img src="icon-master.svg" width="120" alt="AllerX Logo">
</p>

<p align="center">
  <img src="https://img.shields.io/badge/version-0.3.0-blue" alt="Version">
  <img src="https://img.shields.io/github/license/suradet-ps/allerx" alt="License">
  <img src="https://img.shields.io/badge/PRs-welcome-brightgreen" alt="PRs Welcome">
</p>

<p align="center">
  <a href="https://github.com/suradet-ps/allerx/actions/workflows/ci.yml"><img src="https://github.com/suradet-ps/allerx/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/suradet-ps/allerx/actions/workflows/rust-safety.yml"><img src="https://github.com/suradet-ps/allerx/actions/workflows/rust-safety.yml/badge.svg" alt="Rust Safety (Miri)"></a>
  <a href="https://github.com/suradet-ps/allerx/actions/workflows/test-build.yml"><img src="https://github.com/suradet-ps/allerx/actions/workflows/test-build.yml/badge.svg" alt="Test Build"></a>
  <a href="https://github.com/suradet-ps/allerx/actions/workflows/publish-release.yml"><img src="https://github.com/suradet-ps/allerx/actions/workflows/publish-release.yml/badge.svg" alt="Release"></a>
</p>

---

**AllerX** is a desktop tool for pharmacists and physicians. It searches the
hospital's [HOSxP](https://hosxp.org/) database (MySQL/MariaDB) **strictly
read-only** and answers one question fast: *has this patient ever received this
drug, and when?* — the information needed before a delayed-reaction allergy
assessment.

One window, two panels, one verdict band. Nothing is written to HOSxP, nothing
is cached, and nothing that could identify a patient is ever logged.

## Features

- **One search box, auto-detected input** — 13-digit national ID (CID), hospital
  HN, or patient name (prefix match with contains-match fallback), with a
  250 ms debounce.
- **Honest verdicts, three ways** — "พบประวัติ" (found, with the full
  timeline), "ไม่พบประวัติ" (resolved drug, no dispensing rows), or an amber
  "ไม่สามารถยืนยันได้" when the typed term matches nothing in the formulary —
  it never collapses an unknown term into a false "no history".
- **Batch drug check** — queue several drugs as chips and check them all in one
  pass: one verdict band per drug, merged newest-first timeline.
- **Drug autocomplete & resolution** — generic or trade name, exact hit or
  ranked candidates, straight from `drugitems`.
- **Merged OPD + IPD history** — both visit tracks queried concurrently,
  merged, sorted by date; truncation is shown honestly instead of passing a
  capped list off as complete.
- **Patient detail view** — full CID reveal plus a last-30-days medication
  snapshot (read-only).
- **Printable history sheet** — a Thai patient+verdict sheet for consultation
  notes; nothing is persisted to disk.
- **Read-only by construction** — enforced in layers (see
  [Security and privacy](#security-and-privacy)).
- **Encrypted credentials at rest** — HOSxP connection settings stored
  AES-256-GCM-encrypted with the key held in the OS keychain; never plaintext.

## Download

Installers (Windows NSIS/MSI, Linux, macOS ARM) are attached to each
[GitHub release](https://github.com/suradet-ps/allerx/releases). Notes for
clinic PCs:

- Windows needs the WebView2 Runtime — preinstalled on updated Windows 10/11;
  offline machines should install Microsoft's Evergreen offline installer
  first.
- Installers are intentionally **unsigned for v0.x**: expect the SmartScreen
  prompt (*More info → Run anyway*) and coordinate AV whitelisting with
  hospital IT.
- Updates are **manual by design** — hospital IT controls what runs on clinic
  PCs (rationale in [`docs/deployment.md`](docs/deployment.md)).

## Getting started (development)

### Requirements

- Rust **1.85+** (edition 2024) with the `wasm32-unknown-unknown` target
- [Trunk](https://trunkrs.dev) — `cargo install trunk --locked`
- [Tauri CLI](https://tauri.app/start/) — `cargo install tauri-cli --locked`
- Tauri 2 system dependencies for your platform (see the
  [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/))
- A MySQL/MariaDB HOSxP instance with a **SELECT-only** user (see
  [Database access](#database-access))

### Run

```sh
git clone https://github.com/suradet-ps/allerx.git
cd allerx

# Backend test suite — DB-free: search-core runs against a mock repository
cargo test

# Desktop app (trunk serve runs automatically via beforeDevCommand)
cargo tauri dev
```

Notes:

- `src-tauri` is excluded from the root workspace's default members so root
  `cargo test` stays fast and DB-free — check the shell explicitly with
  `cargo test -p allerx-tauri` or `--workspace`.
- `hosxp-connector` integration tests are gated behind
  `--features integration-tests` and must only ever run against a test/staging
  HOSxP instance — never production.

## Tech stack

| Layer | Technology |
|-------|-----------|
| Desktop shell | [Tauri 2](https://tauri.app/) |
| Frontend | [Leptos 0.8](https://leptos.dev/) (Rust CSR → wasm32 via [Trunk](https://trunkrs.dev)) |
| Business logic | `search-core` — pure Rust, tested against a mock repository |
| Database | [sqlx](https://github.com/launchbadge/sqlx) (MySQL/MariaDB) — SELECT-only, compile-time statement constants |
| Settings encryption | [encryptman](https://github.com/suradet-ps/encryptman) + [encryptman-keyring](https://github.com/suradet-ps/encryptman-keyring) (AES-256-GCM, OS keychain) |
| Styling | Plain CSS implementing the token system in [`docs/DESIGN.md`](docs/DESIGN.md) |

## Database access

AllerX reads exactly **eight tables** — `patient`, `drugitems`, `opitemrece`,
`iptitemrece`, `ipt`, `doctor`, `kskdepartment`, `drugusage` — always via
parameterized `SELECT`s naming explicit columns. There is no `SELECT *`, no
write of any kind, ever. The complete column-level matrix plus copy-paste
table-level and column-level `GRANT` templates for the least-privilege account
live in [`docs/deployment.md`](docs/deployment.md).

## Security and privacy

- **Read-only, enforced in layers**: dedicated DB user with `GRANT SELECT`
  only → `SET SESSION TRANSACTION READ ONLY` on every pooled connection → an
  application-level guard rejecting anything but a single read statement →
  parameterized queries everywhere (never SQL string concatenation).
- **No PII in logs** — logs carry query timing and error types, never patient
  names, HN/CID values, or drug-history content; latency instrumentation is
  in-memory and PII-free by construction.
- **CID masking** — national IDs are masked in list views
  (`1-XXXX-XXXXX-XX-1`) and revealed in full only on the detail view.
- **Minimal Tauri capabilities** — no fs/shell scopes beyond what the UI needs.
- The HOSxP password is a `secrecy::SecretString` end to end: zeroized on drop
  and `Debug`-redacted.

## Testing

- `search-core`: unit tests against a mock repository — no live DB required;
  this suite must always pass.
- `hosxp-connector`: every SQL constant has a guard test asserting it is a
  single read statement; integration tests run behind
  `--features integration-tests` against staging only.
- `app`: host-run component tests plus a WASM suite mounted into headless
  Chrome — verdict states, timeline truncation, CID masking, batch flows, and
  the API layer against a fake `invoke` (no Tauri, no database).
- CI gates every change: `fmt` + `clippy -D warnings` + tests (root workspace
  and WASM frontend), `cargo-deny`, clippy + **Miri** on `search-core`, WASM
  tests in headless Chrome, Tauri build (Linux per push/PR, Windows installer
  smoke on demand).

## Documentation

| Doc | What it covers |
|---|---|
| [`AGENTS.md`](AGENTS.md) | Product scope, hard rules, HOSxP schema notes, search flow |
| [`docs/ROADMAP.md`](docs/ROADMAP.md) | Verified current state, gaps, phase plan, schema debt ledger |
| [`docs/DESIGN.md`](docs/DESIGN.md) | Visual/UI design system (tokens, layout, components) |
| [`docs/database.md`](docs/database.md) | HOSxP schema verification log and query patterns |
| [`docs/deployment.md`](docs/deployment.md) | DBA sign-off checklist, GRANT templates, installer runbook |
| [`docs/pilot-notes.md`](docs/pilot-notes.md) | Pilot protocol, scenario script, feedback forms |
| [`docs/perf-baseline.md`](docs/perf-baseline.md) | Performance budgets, measurement protocol, index checklist |
| [`docs/reliability-notes.md`](docs/reliability-notes.md) | Connection health model, failure taxonomy, kill-DB scenario |
| [`docs/a11y-notes.md`](docs/a11y-notes.md) | Accessibility audit, contrast table, keyboard/NVDA walkthrough |

## Roadmap

The core question is answered end to end (patient search → drug resolution →
merged history → verdict), engineering phases 1–5 are implemented (verdict
integrity, performance instrumentation, connection honesty, frontend testing,
clinical depth), and Phase 6 deployment material is shipped while the pilot
with the pharmacy department gets underway. The detailed, honestly-statused
plan lives in [`docs/ROADMAP.md`](docs/ROADMAP.md).

## Contributing

Contributions are welcome! Please open an issue first to discuss what you'd
like to change.

1. Fork the repository
2. Create your feature branch (`git checkout -b feat/amazing-feature`)
3. Commit your changes (`git commit -m 'feat: add amazing feature'`)
4. Push to the branch (`git push origin feat/amazing-feature`)
5. Open a Pull Request

Please read [`AGENTS.md`](AGENTS.md) first — especially the hard rules: HOSxP
is read-only without exceptions, no PII in logs, parameterized queries only.

## License

Distributed under the MIT License. See [LICENSE](LICENSE) for details.

> ⚠️ **Important**: AllerX is designed for hospital-internal use against a
> HOSxP instance the hospital controls. It reports what the record contains;
> clinical interpretation always belongs to the clinician. This software is
> not medical advice.
