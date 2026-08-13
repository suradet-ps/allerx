# AGENTS.md - AllerX

This file gives coding agents (and future contributors) the context needed to work on this repo correctly and safely. Read this **and** `DESIGN.md` (visual/UI design system) before writing any code. If this project has an `AGENTS-RUST.md`, follow it for all Rust-specific style and workflow rules - this file covers product context, architecture, and process; `DESIGN.md` covers visual design only.

## 1. What this project is

AllerX is a desktop app that helps pharmacists/physicians check a patient's medication history before an allergy assessment. It searches HOSxP (hospital MySQL/MariaDB database) **read-only** and answers one question fast: *"Has this patient ever received this drug, and when?"*

**Design goals:** simple, no unnecessary features; fast (< 300ms for typical queries); strict read-only against HOSxP with no exceptions; UI is not required to work fully offline, but the frontend (SPA/WASM) only ever talks to the database through the Tauri backend.

**Non-goals:**
- Not a system for writing/editing/deleting anything in HOSxP.
- Not an allergy *registration* system it's a lookup aid before a decision is made.
- Not a real-time CDSS/alert system at prescribing time (a separate system; may integrate later).
- Does not persist patient data to disk as a permanent cache.

## 2. Hard rules (non-negotiable)

1. **Never write to the HOSxP database.** No `INSERT`/`UPDATE`/`DELETE`/`ALTER`/DDL of any kind, ever - not even in tests, migrations, or "just to try something." All DB access goes through `crates/hosxp-connector`, and that crate must remain SELECT-only end to end (DB user grants, session mode, query guard, parameterized queries - see §5 below). If a task seems to require writing to HOSxP, stop and flag it instead of implementing it.
2. **Never log PII.** No patient name, CID, HN, or drug history content in persisted logs. Log query timing and error types, not parameter values.
3. **Parameterized queries only.** Never build SQL by string concatenation, even for values that "look safe" like an HN.
4. **No schema guessing without a flag.** HOSxP table/column names in this repo are best-effort until verified against the real instance. Any query touching a table listed as "assumption" in §6 must be marked with a `// SCHEMA-UNVERIFIED:` comment until confirmed.

## 3. Workspace layout (ports-lite)

```
allerx/
├── src-tauri/            # Tauri 2 shell — thin command adapters only, no business logic
├── crates/
│   ├── models/           # Domain types. No sqlx, no I/O.
│   ├── hosxp-connector/  # The only crate allowed to talk to MySQL. Read-only guard lives here.
│   └── search-core/      # Business logic (ranking, matching). Depends on a repository trait, not sqlx directly.
└── app/                  # Leptos 0.8 CSR/WASM frontend
    └── style/            # CSS implementing DESIGN.md tokens
```

Dependency direction: `app` → `search-core` → `models`, and `hosxp-connector` implements a repository trait defined in `search-core` (or `models`). `search-core` must be testable with a mock repository, without a real database.

## 4. Domain model (`crates/models`)

```rust
pub struct PatientSummary {
    pub hn: String,
    pub cid: Option<String>,      // national ID — masked on display, see DESIGN.md
    pub full_name_th: String,
    pub birth_date: Option<NaiveDate>,
    pub sex: Option<String>,
}

pub struct DrugHistoryRecord {
    pub visit_date: NaiveDate,
    pub visit_type: VisitType,     // Opd | Ipd
    pub drug_code: String,
    pub drug_name: String,         // generic name
    pub trade_name: Option<String>,
    pub prescriber: Option<String>,
    pub department: Option<String>,
    pub quantity: Option<String>,
    pub route: Option<String>,
}

pub enum VisitType { Opd, Ipd }

pub struct DrugSearchHit {
    pub patient: PatientSummary,
    pub found: bool,
    pub records: Vec<DrugHistoryRecord>,  // most recent first
}
```

## 5. Strict read-only - enforcement mechanism

This is a hard requirement, so it's enforced in layers:

1. **DB user level:** a MySQL user dedicated to AllerX, `GRANT SELECT` only on the relevant schema/tables. This is the real security boundary, not the application code.
2. **Connection level:** `SET SESSION TRANSACTION READ ONLY;` immediately after connect, so the session itself rejects DML even if a bad query slips through.
3. **Application level:** `readonly_guard.rs` in `hosxp-connector` parses the outgoing SQL string and rejects anything not starting with `SELECT`/`WITH` — defense in depth, not the primary boundary.
4. **Query builder:** `sqlx::query_as!` with parameterized queries only. Never concatenate HN/name/CID into SQL directly.
5. **Tauri capabilities:** no fs/shell scope beyond what's strictly needed — minimizes attack surface on the frontend side.

## 6. HOSxP integration — reviewed schema notes (⚠️ still confirm against the live instance before M1)

This layout has been reviewed against standard HOSxP v3/v4 conventions and is more reliable
than a cold guess, but it has **not** been checked against Sarabos Hospital's actual instance
yet. Treat table/column names below as "likely correct, confirm with `SHOW COLUMNS`," and treat
the filtering/join nuances as load-bearing — they're the parts most likely to cause silent bugs
(wrong rows, not errors) if skipped.

### 6.1 Patient search (`patient`)
- Fields: `hn`, `pname`, `fname`, `lname`, `cid`, `birthday`, `sex`.
- ✅ **Verified against the live instance:** the birthday column on this
  instance is `birthday` (some other instances call it `birthdate` — keep
  `birthday` here; flagged via error 1054 if a different instance is used).
- `pid` in `patient` is usually the internal **Person ID** (links to the `person`/civil-registry
  subsystem), **not** the national ID. For the 13-digit national ID, use `cid`.

### 6.2 OPD medication history (`opitemrece` ⋈ `patient` ⋈ `drugitems`, optionally ⋈ `ovst`)
- `opitemrece` has its own `vstdate`/`vsttime` — joining to `ovst` for the date is often
  unnecessary and slower; only join `ovst` if you need visit-level fields `opitemrece` doesn't carry.
- **`opitemrece` stores every billed item, not just drugs** — services, labs, X-rays, etc. all
  live in the same table. Every query against it must filter to the drug category, e.g.
  `WHERE icode LIKE '1%'` (category `1` = drugs/medical supplies in standard HOSxP coding) and/or
  join `drugitems` and filter on its type field (commonly `istype = '1'` or `item_type = 'MED'`
  — confirm which one this instance uses). Skipping this filter will silently return non-drug
  rows mixed into a "medication history."
- Also filter `qty > 0` — zero/negative-quantity rows exist for billing adjustments and are not
  real dispensing events.

### 6.3 IPD medication history (`ipitemrece`/`iptitemrece` ⋈ `ipt` ⋈ `drugitems`)
- The IPD key is **`an`** (Admission Number), not `vn`.
- Two tables can hold IPD drug data depending on hospital configuration:
  1. `opitemrece` — some hospitals log discharge (D/C) or take-home IPD medication here, with `an` populated instead of `vn`.
  2. `iptitemrece` (or `ipitemrece`, naming varies) — the in-stay dispensing table.
  Both should be checked; relying on only one may miss discharge medications.

### 6.4 Prescribing doctor (`doctor`)
- Primary key is `doctor.code` (e.g. `"0001"`).
- `opitemrece`/`ovst` store the prescriber as `opitemrece.doctor`, joined as `opitemrece.doctor = doctor.code`.

### 6.5 Department / clinic
Three tables exist at different granularities — pick based on what the UI needs to show, not by default:
- `clinic` — treatment clinic category (e.g. "อายุรกรรม"), reached via `ovst.cur_dep_busy` or `ovstdiag.clinic`.
- `spclty` — medical specialty, reached via `ovst.spclty`.
- `kskdepartment` — the actual physical service point/room, reached via `ovst.main_dep` or `opitemrece.dep_code`. This is usually the most useful one for "where was this dispensed" in AllerX's UI. ⚠️ **On this instance `opitemrece` uses `dep_code`** (some others call it `depcode` — keep `dep_code` here; flagged via error 1054 on other instances).

### 6.6 Reference join (OPD, single day) — starting point for M2/M3 query work
```sql
SELECT
    o.vn,
    o.hn,
    CONCAT(p.pname, p.fname, ' ', p.lname) AS patient_name,
    o.icode,
    d.name AS drug_name,
    o.qty,
    o.drugusage,
    u.name1 AS usage_line1,
    doc.name AS doctor_name,
    dep.department AS department_name
FROM opitemrece o
INNER JOIN patient p ON o.hn = p.hn
INNER JOIN drugitems d ON o.icode = d.icode
LEFT JOIN drugusage u ON o.drugusage = u.drugusage
LEFT JOIN doctor doc ON o.doctor = doc.code
LEFT JOIN kskdepartment dep ON o.dep_code = dep.depcode
WHERE o.vstdate = CURDATE()
  AND o.qty > 0
  AND d.icode LIKE '1%'
ORDER BY o.vn, o.item_no;
```
AllerX's actual query replaces the `vstdate = CURDATE()` filter with `hn = ? AND icode/drug_name
matches the searched drug`, and adds the IPD-side equivalent (§6.3) run concurrently — see §7.2.

**To confirm with DBA before M1 (still open):**
- Real table/column names on this specific instance (each hospital customizes slightly), and which of `istype`/`item_type` the drug-type filter actually uses.
- Existing indexes on `hn`, `cid`, `pname`, `icode`, `an` — missing indexes will make search slow; may need to request new indexes (safe under read-only — indexes don't touch data).
- Database character encoding (older instances may be TIS-620 rather than UTF-8) — affects Thai name search.
- Whether this instance logs IPD take-home meds in `opitemrece`, `iptitemrece`, or both (§6.3).

## 7. Search flow

### 7.1 Patient search (name / HN / CID)
- Single search box, auto-detects input type: 13-digit numeric → CID; hospital HN pattern → HN; anything else → name (prefix-match `LIKE` first to use indexes, fallback to contains-match).
- 250ms debounce on the frontend before querying.
- Show up to 20 results (HN / name / birth date) for selection.

### 7.2 Drug search + history
- After a patient is selected, type a drug name (generic or trade) with autocomplete from `drugitems`.
- **Batch check (Phase 5):** drugs queue as chips; "ตรวจประวัติ" checks the whole queue at once — the backend fans each drug out concurrently (`check_drugs`), returns one verdict per term, and the frontend renders one verdict band per drug (compact in a batch). A single drug is a batch of one.
- Query OPD + IPD history concurrently (`tokio::join!`), merge, sort by date descending.
- Result must be unambiguous: "received, most recently on [date] at [OPD/IPD] by [doctor]" (check-circle verdict band) or "no history found" (x-circle verdict band).
- Show the full timeline, not just the latest hit — important for delayed-reaction allergy assessment. In a batch, all found drugs merge into one chronological timeline (each row shows its drug name).
- The patient detail view (from the patient bar) reveals the full CID and lists the patient's concurrent medications from the last 30 days (read-only).

## 8. Performance

- Connection pool via sqlx (`MySqlPoolOptions`), pool size ~5 is enough for a single-hospital desktop app.
- Patient search and drug autocomplete depend on indexes existing (see §6); if they don't, lean on debounce + result limits as a stopgap.
- No heavy ORM mapping — `sqlx::query_as` with explicit structs is standard.

## 9. Security & privacy

- Mask CID partially in list views (`1-XXXX-XXXXX-XX-1`), show in full only on the detail view.
- No PII in persisted logs — log query timing/error type, not parameter values.
- Keep the DB connection string out of source control (`.env` + `.gitignore`, or the same encrypted-credential pattern used in `encryptman` + `encryptman-keyring`(https://docs.rs/encryptman/0.2.2/encryptman/)if client-side credential storage is needed).
- The DB password is a `secrecy::SecretString` end to end in Rust: zeroized on drop, `Debug`-redacted (`ConnectionInput` and `HosxConfig`). Two plaintext windows are documented residuals, not bugs: (1) the webview's JS heap while the operator types/confirms — inherent to a desktop IPC flow; (2) sqlx's internal copy inside `MySqlPool`, retained for reconnects for the pool's lifetime. Never log or Debug-print `ConnectionInput`/`HosxConfig` values.
- Optional (M6+): local application-level audit trail (who searched what, when) for internal hospital compliance.

## 10. Milestones

| Milestone | Scope |
|---|---|
| **M0** | Workspace setup, `AGENTS-RUST.md`, Tauri 2 + Leptos 0.8 CSR scaffold, DB connection smoke test (`SELECT 1`) |
| **M1** | `hosxp-connector`: read-only guard + connection pool; verify real schema with DBA/HOSxP instance |
| **M2** | Patient search (HN/CID/name) end-to-end with list UI |
| **M3** | Drug search + autocomplete from `drugitems` |
| **M4** | Medication history query (OPD+IPD merged) + timeline UI |
| **M5** | Performance tuning (debounce, index verification, query timing) |
| **M6** | UI polish (loading/error states, DB-unreachable handling, CID masking) |
| **M7** | Packaging (Tauri bundle), internal pilot with the pharmacy department, feedback |

## 11. Documentation-first workflow

Before implementing a milestone:
1. Confirm the relevant section of this file (and `DESIGN.md` for anything visual) is current. If a task changes the architecture or design, update the doc first, in the same commit/PR as the code — don't let docs drift.
2. For anything touching HOSxP schema, verify column/table names against the real instance before writing the query, and remove the `SCHEMA-UNVERIFIED` marker once confirmed.
3. Keep changes scoped to one milestone at a time.

## 12. Testing expectations

- `search-core`: unit tests against a mock repository, no live DB required. This is the default suite that must always pass.
- `hosxp-connector`: integration tests gated behind a feature flag (e.g. `--features integration-tests`), run only against a test/staging HOSxP instance, never production. Not part of the default `cargo test` run.
- Any new query added to `hosxp-connector` needs a test asserting it is a `SELECT`/`WITH` statement (the read-only guard should catch violations, but test it anyway).

## 13. Style and conventions

- Rust style, error handling, general conventions: follow `AGENTS-RUST.md`.
- **Error handling:** every layer exposes a typed `thiserror` error and converts only at its own boundary. `hosxp-connector` returns `Error` (English, dev-facing; `Config`/`Connect`/`Database`/`Guard`); `search-core`'s trait contract uses `RepositoryError` (`Connection`/`Query`/`Guard`), and `impl From<Error> for RepositoryError` in `hosxp-connector` is the only conversion point between the two. The Thai user-facing translation happens **only** in `src-tauri/commands.rs` (`dev_log`/`map_repo_error`), which also logs the underlying cause for developers. Never map to ad-hoc `String` errors inside a crate, and never translate crate errors to Thai outside `commands.rs`.
- CSS: plain CSS implementing `DESIGN.md` tokens, no framework. Lives in `app/style/`.
- Prefer small, explicit functions over clever abstractions — this codebase should be easy for a pharmacist-developer to re-read six months later.
- Thai-language UI strings live in the frontend (`app/`); keep backend/crate code and comments in English for consistency with the rest of the workspace.

## 14. What NOT to do

- Don't add a full ORM or heavy query builder — `sqlx::query_as!` with explicit structs is the standard here.
- Don't introduce a second way to reach the database. `hosxp-connector` is the only door.
- Don't add write-capable Tauri fs/shell scopes "just in case." Capabilities should stay minimal.
- Don't cache patient data to disk persistently.
