# AllerX

> Check a patient's medication history against HOSxP before an allergy assessment — "has this patient ever received this drug, and when?"

AllerX is a desktop tool for pharmacists and physicians. It searches the hospital's
HOSxP database (MySQL/MariaDB) **strictly read-only** and answers one question
fast: whether a patient has previously received a given drug, and when — the
information needed for a delayed-reaction allergy assessment.

[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust 1.85+](https://img.shields.io/badge/rust-1.85+-dea584.svg)](#requirements)
[![CI](https://github.com/suradet-ps/allerx/actions/workflows/ci.yml/badge.svg)](https://github.com/suradet-ps/allerx/actions/workflows/ci.yml)
[![Rust Safety](https://github.com/suradet-ps/allerx/actions/workflows/rust-safety.yml/badge.svg)](https://github.com/suradet-ps/allerx/actions/workflows/rust-safety.yml)
[![Test Build](https://github.com/suradet-ps/allerx/actions/workflows/test-build.yml/badge.svg)](https://github.com/suradet-ps/allerx/actions/workflows/test-build.yml)
[![Release](https://github.com/suradet-ps/allerx/actions/workflows/publish-release.yml/badge.svg)](https://github.com/suradet-ps/allerx/actions/workflows/publish-release.yml)

## Features

- **One search box, auto-detected input** — 13-digit national ID (CID), hospital
  HN, or patient name (prefix match with contains-match fallback), 250 ms debounce.
- **Drug autocomplete** — generic or trade name, straight from `drugitems`.
- **Merged OPD + IPD history** — queries both visit tracks concurrently, merges
  and sorts by date, and shows the full timeline (not just the latest hit).
- **Unambiguous verdict** — "received, most recently on [date] at [OPD/IPD] by
  [doctor]" or "no history found".
- **Read-only by construction** — the app can only ever `SELECT` from HOSxP,
  enforced in layers (§ [Security](#security-and-privacy)).
- **Encrypted credentials at rest** — HOSxP connection settings are stored
  AES-256-GCM-encrypted (key held in the OS keychain), never in plaintext files.

## Requirements

- Rust **1.85+** (edition 2024)
- Node/Trunk: frontend builds with [Trunk](https://trunkrs.dev) (`trunk serve` / `trunk build`)
- Tauri 2 system dependencies for your platform (see the
  [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/))
- A MySQL/MariaDB instance with a **SELECT-only** HOSxP database user

## Getting started (development)

```sh
# 1. Run the backend crates' test suite (DB-free — search-core tests run against a mock repository)
cargo test

# 2. Build/serve the WASM frontend
cd app
trunk serve            # dev server for the Tauri shell

# 3. Run the desktop shell (separate terminal)
cargo tauri dev
```

Notes:

- `src-tauri` is **not** in the root workspace's default members, so `cargo test`
  at the root stays fast and DB-free. Check the shell explicitly with
  `cargo test -p allerx-tauri` or `cargo test --workspace`.
- `hosxp-connector` integration tests are gated behind `--features integration-tests`
  and must only ever run against a test/staging HOSxP instance — never production.

## Security and privacy

- **Read-only, enforced in layers**: dedicated DB user with `GRANT SELECT` only →
  `SET SESSION TRANSACTION READ ONLY` on connect → an application-level guard that
  rejects any statement not starting with `SELECT`/`WITH` → parameterized queries
  everywhere (never SQL string concatenation).
- **No PII in logs** — logs carry query timing and error types, never patient
  names, HN/CID values, or drug history content.
- **CID masking** — national IDs are masked in list views (`1-XXXX-XXXXX-XX-1`)
  and shown in full only on the detail view.
- **Minimal Tauri capabilities** — no fs/shell scopes beyond what the UI needs.
- The HOSxP password is a `secrecy::SecretString` end to end: zeroized on drop
  and `Debug`-redacted.

## Testing

- `search-core`: unit tests against a mock repository — no live DB required.
  This is the default suite and must always pass.
- `hosxp-connector`: integration tests behind a feature flag, run only against a
  test/staging instance. Every query in the crate has a test asserting it is a
  `SELECT`/`WITH` statement.
- CI runs: `fmt` + `clippy -D warnings` + tests (root workspace and WASM
  frontend), `cargo deny` dependency checks, pedantic clippy + **Miri** on
  `search-core`, and a full Tauri build on tag pushes.

## Documentation

| Doc | What it covers |
|---|---|
| [`AGENTS.md`](AGENTS.md) | Product scope, hard rules, HOSxP schema notes, search flow, milestones |
| [`docs/DESIGN.md`](docs/DESIGN.md) | Visual/UI design system (tokens, layout, components) |
| [`docs/AGENTS-RUST.md`](docs/AGENTS-RUST.md) | Rust-specific style and workflow rules |
| [`docs/ROADMAP.md`](docs/ROADMAP.md) | Verified current state, gaps, and the detailed phase plan |
| [`docs/database.md`](docs/database.md) | HOSxP schema verification log and query patterns |
| [`docs/perf-baseline.md`](docs/perf-baseline.md) | Performance budgets, measurement protocol, DBA index checklist |

## Roadmap

Milestones M0–M4 (workspace, read-only connector, patient search, drug search,
medication history) are implemented; M5–M7 are partially complete. The
forward-looking plan — verdict integrity, performance measurement, connection
honesty, frontend testing, deployment & pilot, and clinical validation — is
detailed in [`docs/ROADMAP.md`](docs/ROADMAP.md).

## License

[MIT](LICENSE) — see the `LICENSE` file.

> ⚠️ **Important**: This project is designed for hospital-internal use against
> a HOSxP instance the hospital controls. It is not medical advice and does not
> replace a clinician's judgment.
