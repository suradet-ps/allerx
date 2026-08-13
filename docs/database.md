# database.md — AllerX HOSxP schema verification log

The living ledger behind AGENTS.md §6 and the `// SCHEMA-UNVERIFIED:` markers
in `crates/hosxp-connector/src/queries.rs`. AllerX reads HOSxP strictly
SELECT-only; this document records what has been confirmed against the live
instance, what is still pending, and the query patterns the connector uses.

## Confirmed against the live instance

| Item | Finding | Notes |
|---|---|---|
| `patient.birthday` | Column is `birthday` (not the `birthdate` variant found on some instances) | Verified via error 1054 on the wrong name; keep `birthday`. |
| `drugitems.name`, `drugitems.strength` | Present and populated (strength e.g. "500 mg") | Confirmed by live-instance testing feedback; the verdict bands now show the resolved drug's name + strength, and icode searches surface them from the autocomplete on. |
| `opitemrece.dep_code` | Column is `dep_code` (not `depcode`) | Verified via error 1054; join target `kskdepartment.depcode` still unverified (below). |

## Pending live-instance confirmation (drives `SCHEMA-UNVERIFIED` markers)

| Item | Where used | Working assumption | Impact of being wrong |
|---|---|---|---|
| `drugitems.trade_name` | drug autocomplete, resolution, history (Phase 1) | Column exists | Tolerated at runtime: tiered queries fall back to the plain baseline (missing column → MySQL 1054 → fallback), so the app degrades to name-only matching. Marker stays until confirmed. |
| `drugitems` drug-type field (`istype` vs `item_type`) | drug autocomplete typed tier (Phase 1) | `istype = '1'` = drugs/medical supplies (AGENTS.md §6.2) | If the column is absent → 1054 → fallback (autocomplete unfiltered). If it exists with different value semantics, autocomplete can silently return nothing — **must** be confirmed before pilot. |
| `iptitemrece` table + `idate`/`itime` + `ipt.hn` | IPD in-stay history | Standard HOSxP table (some instances name it `ipitemrece`) | Tolerated at runtime: a missing table yields "no in-stay records", never an error. Missing in-stay records is a known coverage gap on such instances. |
| `kskdepartment.depcode` | department name join | `depcode` is the PK of `kskdepartment` | Wrong name → 1054 → history queries fail loudly (no fallback for this join). |
| `opitemrece` IPD take-home branch (`an IS NOT NULL`) | IPD take-home history | This instance logs take-home meds in `opitemrece` with `an` populated | If false, IPD take-home coverage is silently empty. Must be confirmed. |
| Sarabos HN pattern (drives `detect_query_kind`) | patient search | 5–10 digits = HN, 13 digits = CID | Wrong pattern → searches classified as name search; still works, but slower and with more results. |
| Database charset (TIS-620 vs UTF-8) | Thai name search | UTF-8 (modern instances) | TIS-620 → Thai `LIKE` matching may fail silently; name search must be re-validated. |
| HOSxP allergy/adverse-reaction table | Phase 7 (proposed cross-check) | Table name varies by instance | n/a — not implemented yet. |

## Query patterns (Phase 1)

### Tiered statements with runtime schema tolerance

Per-instance HOSxP variations are real and documented (AGENTS.md §6). Instead
of failing the app when a column is absent, the connector runs the *first
statement that works*, in order (see `fetch_first_working` in
`repository.rs`). Each tier is a compile-time constant that passes the
read-only guard; tiers differ only in optional columns/filters:

1. **Typed tier** — richest: trade-name matching + drug-type filter
   (`istype = '1'`, unverified).
2. **Trade tier** — trade-name matching, no type filter.
3. **Plain tier** — baseline every instance supports (name match only).

The IPD in-stay source additionally tolerates the whole table being absent
(MySQL 1146) by yielding zero records — the pattern established for
`iptitemrece` in M1.

### Resolution flow (ROADMAP Phase 1)

`fetch_drug_history` maps a typed drug term to an icode in this order:

1. exact `drugitems.icode` hit;
2. exact generic-name hit (`drugitems.name`);
3. exact trade-name hit (`drugitems.trade_name`, tolerant);
4. otherwise → ranked candidates (top 10, sorted by name) surfaced as
   `HistoryVerdict::Unresolved` — **never** a silent "not found".

Only steps 1–3 can produce a `Resolved` verdict (possibly empty = definitive
"no dispensing history"). This is the core patient-safety invariant of the
tool: a false "ไม่พบประวัติ" is impossible by construction for a drug term
the system cannot identify.

### History coverage

| Source | Table | Filter | Limit |
|---|---|---|---|
| OPD | `opitemrece` (`an IS NULL`) | `hn = ? AND icode = ? AND qty > 0` | 200 |
| IPD take-home | `opitemrece` (`an IS NOT NULL`) | same | 200 |
| IPD in-stay | `iptitemrece` ⋈ `ipt` | `ipt.hn = ? AND icode = ? AND qty > 0` | 200 |

Per-source cap is 200 rows (newest first); when a source hits the cap the
`truncated` flag is set so the UI says "มีประวัติเก่ากว่านี้" instead of
presenting the list as complete.

## Open design questions (need clinical input, not just schema checks)

- **Same name, different strength/presentation.** Each strength is a separate
  `icode`, and history is per-icode. A pharmacist asking about "พาราเซตามอล"
  may want to see the 500 mg and 250 mg timelines together. Phase 1 keeps
  per-icode semantics (exact and honest about what was dispensed); a
  name-grouped query is a candidate for Phase 5/7 — it changes the meaning
  of the verdict and needs clinical sign-off first.
- **Type-filter value semantics.** If `istype` uses non-`'1'` values for
  drugs on this instance, the typed tier's `WHERE` must be adjusted; the
  confirmation is on the DBA checklist (ROADMAP Phase 6).
