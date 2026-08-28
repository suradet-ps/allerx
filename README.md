# AllerX

```
 █████╗ ██╗     ██╗     ███████╗██████╗ ██╗  ██╗
██╔══██╗██║     ██║     ██╔════╝██╔══██╗╚██╗██╔╝
███████║██║     ██║     █████╗  ██████╔╝ ╚███╔╝
██╔══██║██║     ██║     ██╔══╝  ██╔══██╗ ███╔╝
██║  ██║███████╗███████╗███████╗██║  ██║██╔██╗
╚═╝  ╚═╝╚══════╝╚══════╝╚══════╝╚═╝  ╚═╝╚═╝ ╚═╝
```

---

## ◆ PULSE

One question, answered without flinching: **has this patient ever received
this drug, and when?** The moment before a delayed-reaction allergy
assessment - a pharmacist standing mid-shift needs the answer in under a
second. AllerX searches the hospital's [HOSxP](https://hosxp.org/)
database, strictly read-only, and reports what the record contains.
Nothing is written, nothing is cached, nothing that could identify a
patient is ever logged.

| P1-P5 ▣ | P6 docs ▣ | P6 pilot ☐ | P7 ☐ | P8 ☐ |
|---|---|---|---|---|

*Verdict integrity, speed, connection honesty, frontend testing, and
clinical depth are sealed. The deployment runbook is shipped; the pilot
with the pharmacy department is underway. Clinical validation and v1.0
hardening wait ahead.*

> Built with Tauri 2 + Leptos 0.8, answered by `search-core`, read from
> eight HOSxP tables by `hosxp-connector` - never a write, never a lie.
>
> **suradet-ps**, artifact keeper

---

## ◆ IGNITION

Two installs, one test, one launch.

```
⟫ rustup target add wasm32-unknown-unknown
⟫ cargo install trunk --locked
⟫ cargo install tauri-cli --locked
⟫ cargo test            # DB-free: search-core runs against a mock repository
⟫ cargo tauri dev       # desktop app; trunk serves itself
```

The compiled artifact lives in [GitHub releases](https://github.com/suradet-ps/allerx/releases)
as Windows NSIS/MSI, Linux, and macOS ARM installers.

<details>
<summary>Notes for clinic PCs</summary>

- Windows needs the WebView2 Runtime - preinstalled on updated Windows 10/11;
  offline machines should take Microsoft's Evergreen offline installer first.
- Installers are **unsigned for v0.x**: expect the SmartScreen prompt
  (*More info -> Run anyway*) and coordinate AV whitelisting with hospital IT.
- Updates are **manual by design** - hospital IT controls what runs on
  clinic PCs (rationale in `docs/deployment.md`).

</details>

---

## ◆ ANATOMY

Three crates, one hard law: HOSxP is read-only, without exceptions.

- **Answers** - `search-core` is pure Rust with the whole question in it:
  patient match, drug resolution, merged OPD + IPD timeline, and the
  verdict. Tested against a mock repository - no live database in the
  test suite, ever.
- **Reads** - `hosxp-connector` talks to MySQL/MariaDB through `sqlx`:
  parameterized `SELECT`s naming explicit columns across eight tables.
  Every SQL constant carries a guard test asserting it is a single read
  statement - there is no `SELECT *`, and no write of any kind.
- **Asks** - `app` is the Leptos frontend: one search box that
  auto-detects a 13-digit CID, hospital HN, or name, debounced at 250 ms,
  and answers with one verdict band.
- **Guards** - read-only is enforced in layers: a dedicated user with
  `GRANT SELECT` only, `SET SESSION TRANSACTION READ ONLY` on every pooled
  connection, an application-level guard rejecting anything but a single
  read statement, parameterized queries everywhere. Logs carry timings and
  error types, never patient names. CIDs are masked until the detail view.
- **Seals** - HOSxP credentials rest AES-256-GCM-encrypted with the key in
  the OS keychain; the password is a `secrecy::SecretString` end to end,
  zeroized on drop, `Debug`-redacted.

---

## ◆ RITUALS

**The core ceremony** - one search, one verdict:

1. Type a CID, an HN, or a name. The box figures out which.
2. Add drugs as chips - batch check resolves them all against the
   formulary, one verdict band per drug, merged newest-first timeline.
3. Read the band: **พบประวัติ** with the full timeline, **ไม่พบประวัติ**
   when no dispensing row exists, or an amber **ไม่สามารถยืนยันได้**
   when the term matches nothing in the formulary.
4. Open the detail view for the full CID and a last-30-days medication
   snapshot; print the Thai patient + verdict sheet for the consultation
   notes.

**The ceremony of honesty** - an unknown term is never collapsed into a
false "no history." When the tool cannot be sure, the verdict says so.
Truncated timelines say they are truncated. A dead HOSxP connection says
it is dead - the app degrades, it never pretends.

**The ceremony of silence** - the search writes nothing, caches nothing,
and logs nothing that could identify a patient. On shared pharmacy
workstations, the patient's identity is a responsibility, not a
convenience.

---

## ◆ ECHOES

**Where this artifact is heading**

```
P1-P5 ▸ verdict integrity, speed, connection honesty, testing, clinical depth ▸ sealed
P6    ▸ deployment docs + pilot protocol shipped; pilot machines pending     ▸ forging
P7    ▸ clinical validation: zero false verdicts on a 50-100 patient audit   ▸ ahead
P8    ▸ v1.0 hardening: the quiet long tail                                   ▸ ahead
```

**Raising the artifact** - read `AGENTS.md` first, especially the hard
rules: HOSxP read-only without exceptions, no PII in logs, parameterized
queries only. The path is written honestly in `docs/ROADMAP.md`; the DBA
sign-off checklist and GRANT templates live in `docs/deployment.md`.

**Status** - CI gates every change: fmt, `clippy -D warnings`, tests,
`cargo-deny`, **Miri** on `search-core`, WASM tests in headless Chrome,
and a Tauri build. [Watch the gates](.github/workflows).

> ⚠️ AllerX reports what the record contains. Clinical interpretation
> belongs to the clinician - this software is not medical advice.

---

```
  ─────────────────────────────────────────
   A false "ไม่พบประวัติ" is not a bug.
   It is a patient-safety event waiting.
  ─────────────────────────────────────────
```

AllerX is distributed under the [MIT License](LICENSE).