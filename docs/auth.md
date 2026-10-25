# auth.md - HOSxP officer login

AllerX opens on a login gate and signs the operator in with their HOSxP
**`officer`** account. The HOSxP database remains strictly read-only
(AGENTS.md §2); the session lives only in the Tauri process, and AllerX
never writes a session, token, or audit row back to HOSxP.

## A. Source of truth

| Table | Use |
|---|---|
| `officer` | **The only login source** - one row by `officer_login_name` |
| `opduser` | **Never read** - old V3 accounts; the hospital HOSxP gateway blacklists it |
| `onlineuser` | Never used for identity |
| `officer_activity_log` | Existing HOSxP audit trail; AllerX does not write it |

The login query reads only the local-password columns:

- `officer_id`
- `officer_login_name`
- `officer_login_password_md5` (32 hex; the plaintext
  `officer_login_password` column is **never selected**)
- `officer_active`
- `officer_authentication_type_id` (`NULL` is treated as `1` = local DB)
- `officer_doctor_code`, `officer_pname`, `officer_fname`, `officer_lname`
- `doctor.name` as the display-name fallback through `officer_doctor_code`

HOSxP group/permission tables are not a usable permission source, so every
signed-in officer gets the same `STAFF` role for now. App-specific roles are
a later milestone.

## B. Flow

```
Launch
  │
  ├─ startup probe: connection config + live health + backend session
  │     ├─ no saved connection → open the settings dialog
  │     └─ saved connection → reconnect in the background
  │
  └─ login gate
        ├─ "ตั้งค่าการเชื่อมต่อ" → settings dialog (host · port · database ·
        │     read-only user · password) → save → login gate
        │
        └─ officer login (username + password)
              ├─ fetch one officer row by login name
              ├─ verify MD5(password) against officer_login_password_md5
              ├─ check active = 'Y'
              ├─ check auth_type = 1 (local database)
              └─ store an in-memory SessionUser; the workspace opens
```

A successful login replaces any previous in-memory session. A connection
change (settings dialog → save) clears the session on both the backend and
the frontend and returns the operator to the login gate, because the
officer was verified against the old database. A login that is still in
flight when the connection changes is discarded rather than stored, so a
session verified against the old database cannot survive the switch.

## C. Password verification

The app computes `MD5(password)` in Rust (`search-core::verify_password`)
and compares it with the stored 32-hex digest, case-insensitively. The
plaintext password is deserialized straight into a `secrecy::SecretString`
at the Tauri command boundary; it is never written to disk, logged, or
returned to the frontend. The login command does not echo whether the
username or the password was wrong, and account-state messages (disabled
or LDAP) are returned only after the password matches.

> **Unverified assumption.** The prior source survey of the sibling
> ER-Refill project confirmed the shape (MD5, 32 hex) on a different HOSxP
> instance, but AllerX has not confirmed the hash input (salt, encoding,
> normalization) on the pilot instance. Test one known account with
> read-only access before the pilot. Thai passwords additionally require
> confirming the source encoding (UTF-8 vs TIS-620).

## D. Session and roles

- `SessionUser` lives in `AppState.session` in the Tauri process only.
- Restarting the app requires signing in again.
- The frontend never receives the password hash.
- These commands require a session at the command boundary:
  `search_patients`, `search_drugs`, `check_drugs`,
  `fetch_concurrent_medications`.
- These commands work before sign-in: `connection_status`,
  `connection_health`, `configure_connection`, `test_connection`, `login`,
  `logout`, `current_session`. The connection dialog must work on the login
  gate, before any officer exists.
- `query_stats` / `clear_query_stats` stay local, PII-free diagnostics.
- The session role is `STAFF` for everyone until app-specific roles are
  introduced. HOSxP group permissions are not consulted.
- The frontend bumps an auth generation counter on every login/logout; an
  in-flight query that finishes after the session changed is discarded, so
  one operator's results can never leak into the next operator's workspace.

## E. Query

The only login statement is `FETCH_OFFICER_SQL` in
`crates/hosxp-connector/src/queries.rs`:

```sql
SELECT CAST(o.officer_id AS SIGNED) AS officer_id,
       o.officer_login_name AS login_name,
       COALESCE(o.officer_login_password_md5, '') AS password_md5,
       COALESCE(o.officer_active, 'N') AS active,
       CAST(COALESCE(o.officer_authentication_type_id, 1) AS SIGNED) AS auth_type_id,
       NULLIF(o.officer_doctor_code, '') AS doctor_code,
       COALESCE(
           NULLIF(TRIM(CONCAT(COALESCE(o.officer_pname, ''), COALESCE(o.officer_fname, ''), ' ', COALESCE(o.officer_lname, ''))), ''),
           NULLIF(TRIM(d.name), ''),
           ''
       ) AS display_name
FROM officer o
LEFT JOIN doctor d ON d.code = NULLIF(o.officer_doctor_code, '')
WHERE o.officer_login_name = ?
LIMIT 1
```

It is a single `SELECT` with one bound parameter. `opduser` and the
plaintext password column never appear. The top bar shows the display name;
if every name source is blank, the backend falls back to the login name.

## F. Why there is no app_* account/session table

AllerX keeps HOSxP read-only and has no separate application database.
`CREATE TABLE`, `CREATE VIEW`, and audit inserts would all be writes, so
none are used. A persistent login log is also a privacy/product decision:
until the hospital approves a local audit store, the app records no staff
identifier on disk. The in-memory session is enough for a single-workstation
desktop app, and HOSxP's own `officer_activity_log` remains the system of
record.

If a local application database is approved later, it can be introduced
without touching HOSxP. That future design is out of scope for this
milestone.

## G. Acceptance checklist

- [ ] Verify `MD5(password)` on one real test account (read-only).
- [ ] Confirm the test account's password encoding (ASCII/UTF-8/TIS-620).
- [ ] Confirm `officer_authentication_type_id` means 1 = local database on
      the pilot instance.
- [ ] Confirm the display-name sources and the `doctor.name` fallback.
- [ ] Confirm whether users who only exist in `opduser` were migrated to
      `officer`; users only in `opduser` cannot sign in.
- [ ] Test active/inactive and LDAP rejection paths with test data.
- [ ] Confirm the read-only MySQL user can `SELECT officer` (no write
      grant).
