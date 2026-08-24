# deployment.md — AllerX DBA sign-off & installer runbook

The Phase 6 checklist (ROADMAP): everything that must be true before the
pilot starts. Part A is for the HOSxP DBA; Part B is for whoever cuts the
release and prepares the pilot PC. Nothing here writes to HOSxP data — the
only writes anywhere are the DBA's own `CREATE USER`/`GRANT`, executed by
the DBA under their own process, never by AllerX.

## Part A — DBA sign-off checklist

The pilot starts only when A1–A4 are all checked and every finding is
recorded in [database.md](database.md) (removing the matching
`SCHEMA-UNVERIFIED` marker in the same commit, AGENTS.md §11).

### A1. Dedicated read-only user (the real security boundary)

```sql
-- Run as an admin on the HOSxP server. Scope the host to the pilot PC.
CREATE USER 'allerx_ro'@'<pilot-pc-host-or-ip>' IDENTIFIED BY '<password>';
GRANT SELECT ON <schema>.* TO 'allerx_ro'@'<pilot-pc-host-or-ip>';

-- Verify: exactly these two rows — Alter routine etc. must NOT appear.
SHOW GRANTS FOR 'allerx_ro'@'<pilot-pc-host-or-ip>';
```

- `SELECT` only. No `INSERT`/`UPDATE`/`DELETE`, no DDL, no temp tables —
  not "for convenience", ever (AGENTS.md §2).
- The password travels via the hospital's password manager, never chat or
  email; it is entered once into AllerX's settings dialog and stored
  encrypted (`encryptman`, AGENTS.md §9).
- The app adds three more read-only layers on top (session mode, SQL
  guard, parameterized queries) — but the grant is the boundary that
  matters.

### A2. Charset verification (drives Thai name search)

```sql
SHOW VARIABLES LIKE 'character_set%';
SHOW CREATE TABLE patient;
```

Expected: `utf8mb4`/`utf8`. If the instance is **TIS-620**, record it in
`database.md` — Thai `LIKE` matching can fail silently, so name search
must be re-validated against live rows before sign-off.

### A3. Schema confirmation (clears the Debt Ledger)

Run on the live instance; write the result into the table below *and*
into [database.md](database.md):

| # | Item | How to check | Result |
|---|---|---|---|
| 1 | `drugitems.trade_name` exists | `SHOW COLUMNS FROM drugitems LIKE 'trade_name';` | ☐ |
| 2 | Drug-type field: `istype` or `item_type`, and which value means drugs | `SHOW COLUMNS FROM drugitems;` then `SELECT DISTINCT <col> FROM drugitems LIMIT 20;` | ☐ |
| 3 | IPD in-stay table: `iptitemrece` vs `ipitemrece` | `SHOW TABLES LIKE '%itemrece%';` | ☐ |
| 4 | `kskdepartment.depcode` is the PK | `SHOW COLUMNS FROM kskdepartment;` | ☐ |
| 5 | IPD take-home meds land in `opitemrece` with `an` populated | `SELECT COUNT(*) FROM opitemrece WHERE an IS NOT NULL AND qty > 0;` (non-zero?) | ☐ |
| 6 | Sarabos HN pattern (digits count/format) | Ask IT/reception for the *format* — no real HNs needed | ☐ |
| 7 | Allergy/adverse-reaction table (Phase 7 groundwork) | `SHOW TABLES LIKE '%allergy%';` / `SHOW TABLES LIKE '%adr%';` | ☐ |

Items 2 and 5 are **must-confirm-before-pilot**: they change what the
verdict covers (see the impact column in `database.md`). Items with
runtime fallbacks degrade gracefully; items without (e.g. 4) fail loudly
and are fixed on the spot.

### A4. Index confirmation (the Phase 2 debt list)

The candidate-index table lives in [perf-baseline.md](perf-baseline.md).
One query shows the DBA what AllerX depends on:

```sql
SELECT table_name, index_name, GROUP_CONCAT(column_name ORDER BY seq_in_index) AS cols
FROM information_schema.statistics
WHERE table_schema = '<schema>'
  AND ((table_name = 'patient'      AND column_name IN ('hn','cid','fname','lname'))
    OR (table_name = 'drugitems'   AND column_name IN ('icode','name'))
    OR (table_name = 'opitemrece'  AND column_name IN ('hn','icode','vstdate','an'))
    OR (table_name IN ('iptitemrece','ipitemrece','ipt') AND column_name = 'an'))
GROUP BY table_name, index_name
ORDER BY table_name, index_name;
```

Missing indexes are a **request** to the DBA (creating them is their call
and their process); requests are PII-free and safe under the read-only
model. If a budget from `perf-baseline.md` is later missed, the missing
index named there is the first suspect.

**Sign-off:** A1 ☐ A2 ☐ A3 ☐ A4 ☐ — dated, recorded in `database.md`.

## Part B — Installer & release runbook

### B1. Pre-tag smoke build (never let a tag be the first Windows bundle)

1. GitHub → Actions → **Test Build** → *Run workflow* (`workflow_dispatch`)
   on `main`.
2. Both jobs run; download the `allerx-windows-installers` artifact from
   the `build-windows-installer` job (NSIS `.exe` + MSI).
3. Install the NSIS `.exe` on any handy Windows box first — only after it
   installs cleanly does a tag make sense.

### B2. Cut the release

1. `main` is green (CI gate: fmt/clippy/test/deny + wasm build).
2. `git tag vX.Y.Z && git push origin vX.Y.Z` → `publish-release.yml`
   builds Windows/Linux/macOS, runs `cargo audit`, publishes the release
   (not a draft).
3. Pilot PC gets the Windows asset: `AllerX_<version>_x64-setup.exe`.

### B3. Install verification on the pilot PC (checklist)

| # | Check | Reference |
|---|---|---|
| 1 | WebView2 Runtime present — **pre-install the Evergreen offline installer**: the hospital LAN has no internet and the Tauri bootstrapper would otherwise try to download it | Microsoft WebView2 offline installer |
| 2 | Installer completes; app launches to the first-run settings modal | — |
| 3 | Connect against the **staging** HOSxP; one search round-trip renders a verdict band | — |
| 4 | Kill-DB recovery scenario passes on this machine | [reliability-notes.md](reliability-notes.md) |
| 5 | Keyring/DPI/hygiene checks pass | [pilot-notes.md](pilot-notes.md) |
| 6 | Keyboard walkthrough + NVDA pass logged | [a11y-notes.md](a11y-notes.md) |

### B4. Code-signing posture — decided: unsigned for the pilot (v0.x)

Single-department internal distribution does not justify certificate cost
and process yet. Consequences, stated plainly:

- Windows SmartScreen will warn ("More info → Run anyway") — put these
  instructions in the handover note.
- Hospital AV may heuristically flag an unsigned new binary — give IT the
  installer hash and install path for whitelisting.
- Revisit before any multi-department rollout: purchase an OV code-signing
  cert or adopt hospital-IT-managed signing.

### B5. Auto-update posture — decided: manual installs for v0.x

The Tauri updater is deliberately not configured: hospital IT controls
what runs on clinic PCs, and silent self-update is exactly the kind of
behavior they must be able to veto. Each release goes through B1–B3 by
hand. Revisit only if the pilot shows update pain.

---

After B3 passes on the pilot machine, start the protocol in
[pilot-notes.md](pilot-notes.md).
