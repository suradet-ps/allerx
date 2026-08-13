//! MySQL connection pool with an enforced read-only session (AGENTS.md §5.2)
//! and a server-side SELECT timeout (ROADMAP Phase 2).

use std::time::Duration;

use sqlx::mysql::{MySqlConnectOptions, MySqlPool, MySqlPoolOptions};

use secrecy::ExposeSecret;

use crate::config::HosxConfig;
use crate::error::Error;
use crate::readonly_guard::READ_ONLY_SESSION_SQL;

/// Pool size: ~5 connections is enough for a single-hospital desktop app
/// (AGENTS.md §8).
const MAX_CONNECTIONS: u32 = 5;

/// Server-side SELECT timeout: 5000 ms, expressed in milliseconds.
///
/// `max_execution_time` (MySQL 5.7.8+; MariaDB uses `max_statement_time`)
/// cancels long-running SELECT statements on the server, so a pathological
/// query (missing index, huge scan) cannot hang the pharmacist mid-shift.
/// The typical-query budget is < 300 ms (AGENTS.md §8; see
/// `docs/perf-baseline.md`).
///
/// The `SET` is *tolerated*: on servers that lack the variable the
/// statement fails and the session proceeds without a timeout. This is an
/// optimization, not a security boundary — the read-only `SET` above it is
/// the one that must never fail.
const STATEMENT_TIMEOUT_SESSION_SQL: &str = "SET SESSION max_execution_time = 5000";

/// Opens a pool of read-only MySQL connections.
///
/// Every new connection immediately runs `SET SESSION TRANSACTION READ
/// ONLY`, so the session itself rejects any DML even if a non-SELECT query
/// slips through the application-level guard. The SELECT timeout is applied
/// best-effort after it.
///
/// # Errors
///
/// Returns [`Error::Connect`] when the pool cannot be opened (network,
/// credentials, server down) — distinct from [`Error::Database`] so the
/// command layer can tell "cannot reach HOSxP" from "query failed".
pub async fn connect(cfg: &HosxConfig) -> Result<MySqlPool, Error> {
    let options = MySqlConnectOptions::new()
        .host(&cfg.host)
        .port(cfg.port)
        .database(&cfg.database)
        .username(&cfg.user)
        // sqlx 0.8 keeps its own plaintext copy of the password inside the
        // pool (needed to reconnect) — documented residual, out of our
        // control. Everything under our control is zeroized on drop.
        .password(cfg.password.expose_secret());

    MySqlPoolOptions::new()
        .max_connections(MAX_CONNECTIONS)
        .acquire_timeout(Duration::from_secs(5))
        .after_connect(|conn, _| {
            Box::pin(async move {
                sqlx::query(READ_ONLY_SESSION_SQL)
                    .execute(&mut *conn)
                    .await?;
                // Best-effort SELECT timeout — see SELECT_TIMEOUT_MS.
                let _ = sqlx::query(STATEMENT_TIMEOUT_SESSION_SQL)
                    .execute(&mut *conn)
                    .await;
                Ok(())
            })
        })
        .connect_with(options)
        .await
        .map_err(Error::Connect)
}
