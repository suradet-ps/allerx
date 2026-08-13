//! Tauri command adapters.
//!
//! Input validation and generic, PII-free error messages live here. SQL
//! never does — the connector owns every statement (AGENTS.md §14).
//!
//! Errors cross the IPC as a typed [`CommandError`] (kind + Thai message,
//! ROADMAP Phase 3) so the frontend can decide presentation from the kind
//! — e.g. a connection banner — instead of matching on message text.

use std::time::Duration;
use std::time::Instant;

use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use tauri::State;
use tokio::sync::Mutex;

use allerx_hosxp_connector::config::{HosxConfig, load, load_vault, save_encrypted};
use allerx_hosxp_connector::{HosxRepository, MySqlPool, pool};
use allerx_models::{ConcurrentMedication, DrugCheckResult, DrugItem, PatientSummary};
use allerx_search_core::{HosxRepository as _, RepositoryError, detect_query_kind};

use crate::state::{AppState, ConnectionHealth};
use crate::stats::{QuerySample, QueryStats};

/// How often the background health monitor pings HOSxP while the app runs.
const HEALTH_CHECK_INTERVAL: Duration = Duration::from_secs(30);

/// The failure class of a command (ROADMAP Phase 3 — failure taxonomy).
///
/// The frontend switches on this for presentation: `Connection` raises the
/// degraded-mode banner, `NotConfigured` suggests the settings dialog,
/// `Query`/`Guard` show the inline message. Message text is never used for
/// logic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CommandErrorKind {
    /// No connection settings stored.
    NotConfigured,
    /// HOSxP could not be reached (pool open or acquire failed).
    Connection,
    /// The read-only guard rejected a statement — an internal error.
    Guard,
    /// The statement failed server-side.
    Query,
}

/// User-facing command error: a machine-readable kind plus the Thai message
/// that is shown verbatim (PII-free — crates never carry parameter values).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub kind: CommandErrorKind,
    pub message: String,
}

impl CommandError {
    fn new(kind: CommandErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

/// Logs the underlying cause for developers (still PII-free — connector and
/// contract errors never carry parameter values, AGENTS.md §2) and returns
/// the user-facing Thai error.
///
/// This is the only place in the codebase where errors are translated for
/// the UI; crates stay English and typed.
fn dev_log(
    context: &str,
    detail: &impl std::fmt::Debug,
    kind: CommandErrorKind,
    message: &'static str,
) -> CommandError {
    eprintln!("[allerx] {context} failed: {detail:?}");
    CommandError::new(kind, message)
}

/// Same as [`dev_log`], but for repository errors, whose user-facing text
/// depends on the variant. `action` is the Thai verb phrase describing what
/// failed (e.g. "ค้นหาผู้ป่วย") — the Query variant renders `{action}ไม่สำเร็จ`.
fn map_repo_error(err: RepositoryError, action: &'static str) -> CommandError {
    eprintln!("[allerx] {action} failed: {err:?}");
    match err {
        RepositoryError::Connection => {
            CommandError::new(CommandErrorKind::Connection, "เชื่อมต่อฐานข้อมูล HOSxP ไม่สำเร็จ")
        }
        RepositoryError::Guard => {
            CommandError::new(CommandErrorKind::Guard, "ระบบความปลอดภัยของแอปปฏิเสธคำสั่งนี้")
        }
        RepositoryError::Query(_) => {
            CommandError::new(CommandErrorKind::Query, format!("{action}ไม่สำเร็จ"))
        }
    }
}

/// Runs a command future and records its end-to-end duration into the
/// PII-free stats ring buffer (ROADMAP Phase 2). The outcome flag is set
/// from the result; the command's own error value passes through untouched.
async fn timed<T>(
    stats: &Mutex<QueryStats>,
    command: &'static str,
    fut: impl std::future::Future<Output = Result<T, CommandError>>,
) -> Result<T, CommandError> {
    let started = Instant::now();
    let result = fut.await;
    stats.lock().await.record(
        command,
        started.elapsed().as_millis() as u64,
        result.is_ok(),
    );
    result
}

/// Reflects query outcomes in the live health state (ROADMAP Phase 3):
/// a success means reachable, a connection failure means not. Guard/query
/// failures do not change reachability — the database answered.
async fn update_health_from_repo_result<T>(state: &AppState, result: &Result<T, RepositoryError>) {
    match result {
        Ok(_) => *state.health.lock().await = ConnectionHealth::Connected,
        Err(RepositoryError::Connection) => {
            *state.health.lock().await = ConnectionHealth::Disconnected
        }
        Err(_) => {}
    }
}

/// Returns the PII-free timing samples collected since app launch (dev/ops
/// only — never rendered in the normal UI, never persisted). Command names
/// and durations only; no parameter values ever (AGENTS.md §2).
#[tauri::command]
pub async fn query_stats(state: State<'_, AppState>) -> Result<Vec<QuerySample>, CommandError> {
    Ok(state.stats.lock().await.snapshot())
}

/// Drops all collected timing samples — start a fresh measurement session.
#[tauri::command]
pub async fn clear_query_stats(state: State<'_, AppState>) -> Result<(), CommandError> {
    state.stats.lock().await.clear();
    Ok(())
}

/// Plaintext connection settings, received from the operator exactly once
/// and encrypted before anything touches disk.
///
/// The password arrives via the local Tauri IPC (webview → Rust, same
/// machine) and is deserialized straight into a [`SecretString`]: zeroized
/// when this struct drops, and `Debug`-redacted. The plaintext necessarily
/// exists in the webview's JS heap while the operator types — that window
/// is inherent and cannot be closed from the Rust side.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionInput {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub user: String,
    pub password: SecretString,
}

/// Result of the M0 smoke test (`SELECT 1`).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionTestResult {
    pub latency_ms: u64,
    pub ok: bool,
}

/// Encrypts and stores HOSxP connection settings.
///
/// The plaintext fields exist only inside this call frame — nothing is
/// logged, and disk only ever sees ciphertext (AGENTS.md §9). The old pool
/// is dropped and a background health check re-verifies the new settings,
/// so the status dot reflects reality within moments of saving.
#[tauri::command]
pub async fn configure_connection(
    state: State<'_, AppState>,
    input: ConnectionInput,
) -> Result<(), CommandError> {
    let store = load_vault().map_err(|err| {
        dev_log(
            "load_vault",
            &err,
            CommandErrorKind::Query,
            "ไม่สามารถเข้าถึงที่เก็บกุญแจของระบบได้",
        )
    })?;
    let cfg = HosxConfig::new(
        input.host,
        input.port,
        input.database,
        input.user,
        input.password,
    );
    save_encrypted(&state.config_path(), &store, &cfg)
        .await
        .map_err(|err| {
            dev_log(
                "save_encrypted",
                &err,
                CommandErrorKind::Query,
                "บันทึกการตั้งค่าการเชื่อมต่อไม่สำเร็จ",
            )
        })?;
    // Reconfigured — drop the old pool and re-verify in the background so
    // the status dot is honest about the new settings.
    *state.pool.lock().await = None;
    *state.health.lock().await = ConnectionHealth::Disconnected;
    let probe_state = state.inner().clone();
    tauri::async_runtime::spawn(async move {
        let health = health_check(&probe_state).await;
        *probe_state.health.lock().await = health;
    });
    Ok(())
}

/// Reports whether encrypted connection settings exist on this machine —
/// used by the first-run flow only. The status dot uses
/// [`connection_health`], which reflects live reachability, not this.
#[tauri::command]
pub fn connection_status(state: State<'_, AppState>) -> bool {
    state.config_path().exists()
}

/// The live connection state behind the top-bar dot (ROADMAP Phase 3).
///
/// This is a read of state kept fresh by the startup warm-up, the 30-second
/// health monitor, and every query outcome — the command itself does no I/O.
#[tauri::command]
pub async fn connection_health(
    state: State<'_, AppState>,
) -> Result<ConnectionHealth, CommandError> {
    Ok(*state.health.lock().await)
}

/// Connects to HOSxP with the **typed** settings (not the stored ones) and
/// runs the `SELECT 1` smoke test — so the operator can verify the form
/// before committing anything to disk.
///
/// On success the pool is kept for subsequent queries; a later
/// [`configure_connection`] rebuilds it. Error messages are deliberately
/// generic — no credentials, hosts, or parameter values ever leak to the UI
/// or logs.
#[tauri::command]
pub async fn test_connection(
    state: State<'_, AppState>,
    input: ConnectionInput,
) -> Result<ConnectionTestResult, CommandError> {
    let cfg = HosxConfig::new(
        input.host,
        input.port,
        input.database,
        input.user,
        input.password,
    );
    let pool = match pool::connect(&cfg).await {
        Ok(pool) => pool,
        Err(err) => {
            *state.health.lock().await = ConnectionHealth::Disconnected;
            return Err(dev_log(
                "pool::connect",
                &err,
                CommandErrorKind::Connection,
                "เชื่อมต่อฐานข้อมูล HOSxP ไม่สำเร็จ",
            ));
        }
    };

    let repo = HosxRepository::new(pool.clone());
    let started = Instant::now();
    let ping_result = repo.ping().await;
    update_health_from_repo_result(&state, &ping_result).await;
    let latency_ms = started.elapsed().as_millis() as u64;
    let ok = ping_result.is_ok();
    // The latency is returned to the operator; also record it in the
    // PII-free stats buffer for the perf baseline (ROADMAP Phase 2).
    state
        .stats
        .lock()
        .await
        .record("test_connection", latency_ms, ok);
    ping_result.map_err(|err| map_repo_error(err, "ทดสอบการเชื่อมต่อ"))?;

    *state.pool.lock().await = Some(pool);
    Ok(ConnectionTestResult {
        latency_ms,
        ok: true,
    })
}

/// Background warm-up (ROADMAP Phase 2): loads the stored settings and
/// opens the read-only pool, forcing a real connection with a ping, so the
/// first operator query never pays connect latency. Sets the initial
/// health state (ROADMAP Phase 3).
///
/// Failures are swallowed here on purpose — the first real query retries
/// through [`acquire_pool`] and surfaces the proper (generic) error to the
/// operator. The outcome is recorded in the PII-free stats buffer so the
/// cold-start measurement can see it.
pub async fn warm_up_pool(state: &AppState) {
    if state.pool.lock().await.is_some() {
        return;
    }
    let started = Instant::now();
    let health = match warm_up(state).await {
        Ok(()) => ConnectionHealth::Connected,
        Err(_) if !state.config_path().exists() => ConnectionHealth::Unconfigured,
        Err(_) => ConnectionHealth::Disconnected,
    };
    *state.health.lock().await = health;
    state.stats.lock().await.record(
        "warm_up_pool",
        started.elapsed().as_millis() as u64,
        health == ConnectionHealth::Connected,
    );
}

/// Startup + periodic health monitor (ROADMAP Phase 3): warms the pool
/// once, then pings every [`HEALTH_CHECK_INTERVAL`] for the app's lifetime,
/// keeping `state.health` honest — a dead database shows up on the status
/// dot within seconds, mid-shift, without waiting for a failed query.
pub async fn run_health_monitor(state: AppState) {
    warm_up_pool(&state).await;
    let mut interval = tokio::time::interval(HEALTH_CHECK_INTERVAL);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        interval.tick().await;
        if !state.config_path().exists() {
            *state.health.lock().await = ConnectionHealth::Unconfigured;
            continue;
        }
        let health = health_check(&state).await;
        *state.health.lock().await = health;
    }
}

/// One live reachability check: ensures a pool exists (reconnecting from
/// stored settings when needed) and pings. Updates `state.pool` and records
/// a PII-free `health_check` sample. Never fails — the outcome IS the
/// return value.
pub async fn health_check(state: &AppState) -> ConnectionHealth {
    if !state.config_path().exists() {
        return ConnectionHealth::Unconfigured;
    }
    if state.pool.lock().await.is_none() {
        let _ = warm_up(state).await;
    }
    let Some(pool) = state.pool.lock().await.clone() else {
        return ConnectionHealth::Disconnected;
    };
    let repo = HosxRepository::new(pool);
    let started = Instant::now();
    let ok = repo.ping().await.is_ok();
    state
        .stats
        .lock()
        .await
        .record("health_check", started.elapsed().as_millis() as u64, ok);
    if ok {
        ConnectionHealth::Connected
    } else {
        ConnectionHealth::Disconnected
    }
}

/// One warm-up attempt. `Ok` means a working pool is stored in `state`.
async fn warm_up(state: &AppState) -> Result<(), ()> {
    let store = load_vault().map_err(|_| ())?;
    let cfg = load(&state.config_path(), &store)
        .await
        .map_err(|_| ())?
        .ok_or(())?;
    let pool = pool::connect(&cfg).await.map_err(|_| ())?;
    let repo = HosxRepository::new(pool.clone());
    // Force a real connection; never cache a pool to a dead database.
    repo.ping().await.map_err(|_| ())?;
    *state.pool.lock().await = Some(pool);
    Ok(())
}

/// Returns the read-only pool, connecting on first use. Updates the live
/// health state on every path (ROADMAP Phase 3).
///
/// Prefers the pool already built by [`test_connection`]; when none exists,
/// the stored encrypted settings are loaded (and decrypted) to open a fresh
/// one. Error messages stay generic — no hosts, users, or parameter values.
async fn acquire_pool(state: &AppState) -> Result<MySqlPool, CommandError> {
    if let Some(pool) = state.pool.lock().await.clone() {
        *state.health.lock().await = ConnectionHealth::Connected;
        return Ok(pool);
    }
    let store = load_vault().map_err(|err| {
        dev_log(
            "load_vault",
            &err,
            CommandErrorKind::Query,
            "ไม่สามารถเข้าถึงที่เก็บกุญแจของระบบได้",
        )
    })?;
    let cfg = match load(&state.config_path(), &store).await {
        Ok(Some(cfg)) => cfg,
        Ok(None) => {
            *state.health.lock().await = ConnectionHealth::Unconfigured;
            return Err(CommandError::new(
                CommandErrorKind::NotConfigured,
                "ยังไม่ได้ตั้งค่าการเชื่อมต่อ HOSxP",
            ));
        }
        Err(err) => {
            return Err(dev_log(
                "load_config",
                &err,
                CommandErrorKind::Query,
                "อ่านการตั้งค่าการเชื่อมต่อไม่สำเร็จ กรุณาตั้งค่าใหม่",
            ));
        }
    };
    let pool = match pool::connect(&cfg).await {
        Ok(pool) => pool,
        Err(err) => {
            *state.health.lock().await = ConnectionHealth::Disconnected;
            return Err(dev_log(
                "pool::connect",
                &err,
                CommandErrorKind::Connection,
                "เชื่อมต่อฐานข้อมูล HOSxP ไม่สำเร็จ",
            ));
        }
    };
    *state.health.lock().await = ConnectionHealth::Connected;
    *state.pool.lock().await = Some(pool.clone());
    Ok(pool)
}

/// Patient search by HN / CID / name (AGENTS.md §7.1, milestone M2).
///
/// The input type is auto-detected; an empty term short-circuits to an
/// empty result list. Errors are generic — parameter values never reach
/// the UI or logs.
#[tauri::command]
pub async fn search_patients(
    state: State<'_, AppState>,
    term: String,
) -> Result<Vec<PatientSummary>, CommandError> {
    let term = term.trim().to_string();
    if term.is_empty() {
        return Ok(Vec::new());
    }
    let stats = state.stats.clone();
    timed(&stats, "search_patients", async move {
        let pool = acquire_pool(&state).await?;
        let repo = HosxRepository::new(pool);
        let result = repo.search_patients(&term, detect_query_kind(&term)).await;
        update_health_from_repo_result(&state, &result).await;
        result.map_err(|err| map_repo_error(err, "ค้นหาผู้ป่วย"))
    })
    .await
}

/// Drug-name autocomplete from `drugitems` (AGENTS.md §7.2, milestone M3).
///
/// Prefix match first, contains-match fallback, at most 20 items. Errors
/// are generic — parameter values never reach the UI or logs.
#[tauri::command]
pub async fn search_drugs(
    state: State<'_, AppState>,
    term: String,
) -> Result<Vec<DrugItem>, CommandError> {
    let term = term.trim().to_string();
    if term.is_empty() {
        return Ok(Vec::new());
    }
    let stats = state.stats.clone();
    timed(&stats, "search_drugs", async move {
        let pool = acquire_pool(&state).await?;
        let repo = HosxRepository::new(pool);
        let result = repo.search_drugs(&term).await;
        update_health_from_repo_result(&state, &result).await;
        result.map_err(|err| map_repo_error(err, "ค้นหายา"))
    })
    .await
}

/// Medication history for one patient + several drugs (ROADMAP Phase 5).
///
/// Each drug is checked concurrently on the backend (OPD + IPD merged
/// most-recent-first) and the results carry their term labels. The verdict
/// contract (ROADMAP Phase 1) applies per drug: an exact hit yields
/// [`HistoryVerdict::Resolved`] (possibly empty — a legitimate
/// "ไม่พบประวัติ"); an unresolvable term yields
/// [`HistoryVerdict::Unresolved`] with disambiguation candidates. The
/// frontend decides the visual verdicts. Errors are generic.
#[tauri::command]
pub async fn check_drugs(
    state: State<'_, AppState>,
    hn: String,
    drugs: Vec<String>,
) -> Result<Vec<DrugCheckResult>, CommandError> {
    let hn = hn.trim().to_string();
    if hn.is_empty() || drugs.is_empty() {
        return Ok(Vec::new());
    }
    let stats = state.stats.clone();
    timed(&stats, "check_drugs", async move {
        let pool = acquire_pool(&state).await?;
        let repo = HosxRepository::new(pool);
        let result = repo.check_drugs(&hn, &drugs).await;
        update_health_from_repo_result(&state, &result).await;
        result.map_err(|err| map_repo_error(err, "ตรวจสอบประวัติ"))
    })
    .await
}

/// Recent concurrent medications for a patient (ROADMAP Phase 5) — the
/// "ยาที่ได้รับล่าสุด" snapshot for the detail view, deduped per icode.
#[tauri::command]
pub async fn fetch_concurrent_medications(
    state: State<'_, AppState>,
    hn: String,
) -> Result<Vec<ConcurrentMedication>, CommandError> {
    let hn = hn.trim().to_string();
    if hn.is_empty() {
        return Ok(Vec::new());
    }
    let stats = state.stats.clone();
    timed(&stats, "fetch_concurrent_medications", async move {
        let pool = acquire_pool(&state).await?;
        let repo = HosxRepository::new(pool);
        let result = repo.fetch_concurrent_medications(&hn).await;
        update_health_from_repo_result(&state, &result).await;
        result.map_err(|err| map_repo_error(err, "ดึงรายการยา"))
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_error_is_translated_independently_of_action() {
        let err = map_repo_error(RepositoryError::Connection, "ค้นหาผู้ป่วย");
        assert_eq!(err.kind, CommandErrorKind::Connection);
        assert_eq!(err.message, "เชื่อมต่อฐานข้อมูล HOSxP ไม่สำเร็จ");
    }

    #[test]
    fn guard_error_is_translated_independently_of_action() {
        let err = map_repo_error(RepositoryError::Guard, "ค้นหายา");
        assert_eq!(err.kind, CommandErrorKind::Guard);
        assert_eq!(err.message, "ระบบความปลอดภัยของแอปปฏิเสธคำสั่งนี้");
    }

    #[test]
    fn query_error_renders_the_action_that_failed() {
        let err = map_repo_error(
            RepositoryError::Query("row not found".to_string()),
            "ตรวจสอบประวัติ",
        );
        assert_eq!(err.kind, CommandErrorKind::Query);
        assert_eq!(err.message, "ตรวจสอบประวัติไม่สำเร็จ");
    }

    #[test]
    fn not_configured_error_carries_its_kind() {
        let err = CommandError::new(CommandErrorKind::NotConfigured, "ยังไม่ได้ตั้งค่าการเชื่อมต่อ HOSxP");
        assert_eq!(err.kind, CommandErrorKind::NotConfigured);
    }
}
