//! MySQL connection pool with an enforced read-only session (AGENTS.md §5.2).

use std::time::Duration;

use sqlx::mysql::{MySqlConnectOptions, MySqlPool, MySqlPoolOptions};

use secrecy::ExposeSecret;

use crate::config::HosxConfig;
use crate::error::Error;
use crate::readonly_guard::READ_ONLY_SESSION_SQL;

/// Pool size: ~5 connections is enough for a single-hospital desktop app
/// (AGENTS.md §8).
const MAX_CONNECTIONS: u32 = 5;

/// Opens a pool of read-only MySQL connections.
///
/// Every new connection immediately runs `SET SESSION TRANSACTION READ
/// ONLY`, so the session itself rejects any DML even if a non-SELECT query
/// slips through the application-level guard.
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
                sqlx::query(READ_ONLY_SESSION_SQL).execute(conn).await?;
                Ok(())
            })
        })
        .connect_with(options)
        .await
        .map_err(Error::Connect)
}
