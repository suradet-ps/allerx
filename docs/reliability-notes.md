# reliability-notes.md — AllerX connection health & degraded operation

How AllerX behaves when HOSxP is unreachable, and how to verify it
(ROADMAP Phase 3, Gap G2). The app must never pretend it is connected, and
must never hold the pharmacist hostage to a reconnect spinner.

## The health model

| State | Meaning | Drives |
|---|---|---|
| `Connected` | A ping or query succeeded recently | green dot, "เชื่อมต่อแล้ว" |
| `Disconnected` | A ping failed or a query could not reach HOSxP | red dot, "HOSxP ไม่พร้อมใช้งาน" + degraded banner on failed interactions |
| `Unconfigured` | No stored settings | red dot, "ยังไม่ได้ตั้งค่า" (first-run flow opens the settings dialog) |

Health is **never** derived from "the config file exists". It is kept fresh
by three independent mechanisms, so a mid-shift database outage shows up
within seconds:

1. **Startup warm-up** (`warm_up_pool`) — connects and pings once at launch.
2. **30-second health monitor** (`run_health_monitor`) — `SELECT 1` ping
   loop for the app's lifetime; each check lands in the PII-free stats
   buffer as a `health_check` sample.
3. **Query outcomes** — every successful command sets `Connected`; every
   connection failure sets `Disconnected` (guard/query errors do not change
   reachability — the database answered).

The frontend polls `connection_health` every 30 s for the status dot; the
banner is driven by actual failed interactions.

## Failure taxonomy (what the operator sees)

| Class (`CommandErrorKind`) | Where it happens | UI treatment | Example message |
|---|---|---|---|
| `notConfigured` | no stored settings | banner + inline | "ยังไม่ได้ตั้งค่าการเชื่อมต่อ HOSxP" |
| `connection` | pool open / acquire failed / ping failed | **degraded-mode banner** | "เชื่อมต่อฐานข้อมูล HOSxP ไม่สำเร็จ" |
| `guard` | read-only guard rejected a statement (internal bug) | inline message | "ระบบความปลอดภัยของแอปปฏิเสธคำสั่งนี้" |
| `query` | statement failed server-side | inline message | "ตรวจสอบประวัติไม่สำเร็จ" |

The frontend switches on `kind`, never on message text.

## Degraded-mode behavior

- When a search fails with `connection`: the amber banner appears above the
  verdict band ("เชื่อมต่อฐานข้อมูล HOSxP ไม่สำเร็จ") — the verdict band
  itself returns to `Pending` and never implies an answer.
- The UI stays fully interactive: the pharmacist can still select a
  patient, type a drug, and retry. Retrying is just another query.
- The banner is dismissed by the operator (X) or auto-cleared by the next
  successful query.
- Raw errors never reach the UI — messages are the fixed Thai strings from
  the command boundary.

## Kill-DB scenario (manual test procedure)

Run this on the pilot machine before sign-off, and after any change to the
health machinery:

1. Start AllerX with a working connection. Green dot: "เชื่อมต่อแล้ว".
2. Stop the MySQL/MariaDB service (or disconnect the machine from the LAN).
3. Within ~30 s the dot turns red: "HOSxP ไม่พร้อมใช้งาน" — **without any
   user interaction** (this is the health monitor, not a failed query).
4. Run a patient search. Expect: amber banner, no crash, no frozen UI, no
   raw error text, verdict stays `Pending`.
5. Restart MySQL (or reconnect the LAN). Within ~30 s the dot returns to
   green. A search now succeeds and clears the banner.
6. Repeat with the app started *while* HOSxP is already down: the app must
   open normally, show the red dot + banner on first failed query, and
   recover without an app restart.

## Keychain & config edge cases

- **Keyring unavailable** (headless/CI/locked-down workstation): warm-up
  and health checks silently skip (pool stays unset); the first query
  surfaces "ไม่สามารถเข้าถึงที่เก็บกุญแจของระบบได้" (kind `query`) and the
  settings dialog remains the path forward. This is by design: the app
  must not crash because the OS credential store is unreachable.
- **Corrupt settings file**: warm-up skips; first query shows
  "อ่านการตั้งค่าการเชื่อมต่อไม่สำเร็จ กรุณาตั้งค่าใหม่" — reopen the
  settings dialog and re-enter.
- **Settings saved without a successful test**: `configure_connection`
  drops the old pool and re-verifies the new settings in the background,
  so the dot reflects reality moments after saving.

## Timeouts (see also docs/perf-baseline.md)

| Timeout | Value | Effect |
|---|---|---|
| Pool acquire | 5 s | a dead server does not block a query forever |
| Server-side SELECT | 5 s (`max_execution_time`, tolerated if unsupported) | a pathological query cannot hang the session |
| Health ping loop | 30 s | dot freshness vs. load trade-off |

Everything in this document is PII-free: health samples record timing and
outcome only (AGENTS.md §2), never patient data.
