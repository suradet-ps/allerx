# perf-baseline.md — AllerX performance baseline & budget

The measurement protocol behind ROADMAP Phase 2 (Speed & Measurement).
Numbers are collected with the app's own PII-free timing buffer
(`query_stats` / `clear_query_stats` Tauri commands, ROADMAP Phase 2) —
nothing here ever touches a log file or leaves the machine.

## Reference hardware

A mid-range clinic PC, per the pilot requirement:

| Component | Specification |
|---|---|
| CPU | Intel Core i5 (or equivalent), ~3 GHz |
| RAM | 8 GB |
| Disk | 7200 rpm HDD (not SSD) |
| OS | Windows 10/11, hospital domain login |
| Network | Hospital LAN, HOSxP server on the same segment |

All measurements are recorded **on this machine**, against the **staging
HOSxP instance** (never production), with the app window at its default
960×760 size.

## Budgets

| Measurement | Budget | Method |
|---|---|---|
| App launch → interactive | < 3 s | cold start, `warm_up_pool` sample in `query_stats` + stopwatch to verdict-band hint |
| Patient search (HN/CID/name) | < 300 ms typical | `search_patients` sample |
| Drug autocomplete | < 300 ms typical | `search_drugs` sample |
| Full history lookup (OPD + IPD merged) | < 300 ms typical | `fetch_drug_history` sample |
| Verdict render (after response) | < 100 ms | browser-side measure (devtools), 5 samples |
| Connection test (`SELECT 1`) | < 100 ms | `test_connection` sample + operator-visible latency |

`< 300 ms` is AGENTS.md §8's target. A typical measurement = median of 10
consecutive samples with the debounce reset between each; a cold sample
(first query after launch) is recorded separately and excluded from the
median.

## Current configuration (documented so tuning is reproducible)

| Setting | Value | Where |
|---|---|---|
| Pool size | 5 (`MAX_CONNECTIONS`) | `crates/hosxp-connector/src/pool.rs` |
| Acquire timeout | 5 s | same |
| Server-side SELECT timeout | 5 s (`SET SESSION max_execution_time = 5000`, tolerated if unsupported) | same |
| Frontend debounce | 250 ms | `app/src/components/*_search.rs` |
| Result limits | 20 patients / 20 drugs / 200 rows per history source | `queries.rs` |

## DBA index checklist (the "missing index debt list")

Every AllerX query is a parameterized `SELECT`; performance depends on the
indexes HOSxP already has. Request the DBA to confirm (not create blindly —
indexes on a live system need the DBA's own process) that these exist:

| Table | Index (candidate) | Serves |
|---|---|---|
| `patient` | `(hn)` | patient search by HN |
| `patient` | `(cid)` | patient search by CID |
| `patient` | `(fname)`, `(lname)` | name prefix search |
| `drugitems` | `(icode)` | resolution + history joins |
| `drugitems` | `(name)` | autocomplete prefix match |
| `opitemrece` | `(hn, icode, vstdate, vsttime)` | OPD + IPD take-home history |
| `opitemrece` | `(an)` | IPD take-home branch |
| `iptitemrece` | `(an)` | IPD in-stay join |
| `ipt` | `(hn)` | IPD in-stay history |

If a budget is missed, the specific missing index named in this table is
the first suspect. Do **not** add indexes to HOSxP from the app side —
index work is DBA territory and read-only safe; request, don't execute.

## Recorded measurements (fill in during staging sessions / pilot)

| Date | Machine | App version | Test | Median | Cold | Notes |
|---|---|---|---|---|---|---|
| — | — | — | patient search | — | — | — |
| — | — | — | drug autocomplete | — | — | — |
| — | — | — | history lookup | — | — | — |
| — | — | — | launch → interactive | — | — | — |

Protocol for filling a row:

1. `clear_query_stats` (from the webview devtools: `__TAURI_INTERNALS__.invoke('clear_query_stats')`).
2. Run the flow 10 times with 250 ms debounce pauses.
3. `query_stats` → read the medians; record the first (cold) sample separately.
4. Note anything anomalous (server backups, antivirus scan, network event).

## Regression policy

- CI does not measure latency (no staging DB in CI — unchanged, by design).
- Regressions are caught at the pilot/staging sessions against this table;
  a new budget miss after a release must be explained in the release notes
  or the budget updated with justification.
- The instrumentation itself is PII-free by construction (command names,
  durations, outcome flag — never parameter values, AGENTS.md §2) and
  in-memory only.
