//! Tauri command adapters.
//!
//! Input validation and generic, PII-free error messages live here. SQL
//! never does — the connector owns every statement (AGENTS.md §14).

use std::time::Instant;

use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use tauri::State;
use tokio::sync::Mutex;

use allerx_hosxp_connector::config::{HosxConfig, load, load_vault, save_encrypted};
use allerx_hosxp_connector::{HosxRepository, MySqlPool, pool};
use allerx_models::{DrugItem, HistoryVerdict, PatientSummary};
use allerx_search_core::{HosxRepository as _, RepositoryError, detect_query_kind};

use crate::state::AppState;
use crate::stats::{QuerySample, QueryStats};

/// Logs the underlying cause for developers (still PII-free — connector and
/// contract errors never carry parameter values, AGENTS.md §2) and returns
/// the user-facing Thai message.
///
/// This is the only place in the codebase where errors are translated for
/// the UI; crates stay English and typed.
fn dev_log(context: &str, detail: &impl std::fmt::Debug, message: &'static str) -> String {
    eprintln!("[allerx] {context} failed: {detail:?}");
    message.to_string()
}

/// Same as [`dev_log`], but for repository errors, whose user-facing text
/// depends on the variant. `action` is the Thai verb phrase describing what
/// failed (e.g. "ค้นหาผู้ป่วย") — the Query variant renders `{action}ไม่สำเร็จ`.
fn map_repo_error(err: RepositoryError, action: &'static str) -> String {
    eprintln!("[allerx] {action} failed: {err:?}");
    match err {
        RepositoryError::Connection => "เชื่อมต่อฐานข้อมูล HOSxP ไม่สำเร็จ".to_string(),
        RepositoryError::Guard => "ระบบความปลอดภัยของแอปปฏิเสธคำสั่งนี้".to_string(),
        RepositoryError::Query(_) => format!("{action}ไม่สำเร็จ"),
    }
}

/// Runs a command future and records its end-to-end duration into the
/// PII-free stats ring buffer (ROADMAP Phase 2). The outcome flag is set
/// from the result; the command's own error value passes through untouched.
async fn timed<T>(
    stats: &Mutex<QueryStats>,
    command: &'static str,
    fut: impl std::future::Future<Output = Result<T, String>>,
) -> Result<T, String> {
    let started = Instant::now();
    let result = fut.await;
    stats.lock().await.record(
        command,
        started.elapsed().as_millis() as u64,
        result.is_ok(),
    );
    result
}

/// Returns the PII-free timing samples collected since app launch (dev/ops
/// only — never rendered in the normal UI, never persisted). Command names
/// and durations only; no parameter values ever (AGENTS.md §2).
#[tauri::command]
pub async fn query_stats(state: State<'_, AppState>) -> Result<Vec<QuerySample>, String> {
    Ok(state.stats.lock().await.snapshot())
}

/// Drops all collected timing samples — start a fresh measurement session.
#[tauri::command]
pub async fn clear_query_stats(state: State<'_, AppState>) -> Result<(), String> {
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
/// logged, and disk only ever sees ciphertext (AGENTS.md §9).
#[tauri::command]
pub async fn configure_connection(
    state: State<'_, AppState>,
    input: ConnectionInput,
) -> Result<(), String> {
    let store = load_vault()
        .map_err(|err| dev_log("load_vault", &err, "ไม่สามารถเข้าถึงที่เก็บกุญแจของระบบได้"))?;
    let cfg = HosxConfig::new(
        input.host,
        input.port,
        input.database,
        input.user,
        input.password,
    );
    save_encrypted(&state.config_path(), &store, &cfg)
        .await
        .map_err(|err| dev_log("save_encrypted", &err, "บันทึกการตั้งค่าการเชื่อมต่อไม่สำเร็จ"))?;
    // Reconfigured — drop the old pool so the next query rebuilds it.
    *state.pool.lock().await = None;
    Ok(())
}

/// Reports whether encrypted connection settings exist on this machine.
#[tauri::command]
pub fn connection_status(state: State<'_, AppState>) -> bool {
    state.config_path().exists()
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
) -> Result<ConnectionTestResult, String> {
    let cfg = HosxConfig::new(
        input.host,
        input.port,
        input.database,
        input.user,
        input.password,
    );
    let pool = pool::connect(&cfg)
        .await
        .map_err(|err| dev_log("pool::connect", &err, "เชื่อมต่อฐานข้อมูล HOSxP ไม่สำเร็จ"))?;

    let repo = HosxRepository::new(pool.clone());
    let started = Instant::now();
    let ping_result = repo
        .ping()
        .await
        .map_err(|err| map_repo_error(err, "ทดสอบการเชื่อมต่อ"));
    let latency_ms = started.elapsed().as_millis() as u64;
    let ok = ping_result.is_ok();
    // The latency is returned to the operator; also record it in the
    // PII-free stats buffer for the perf baseline (ROADMAP Phase 2).
    state
        .stats
        .lock()
        .await
        .record("test_connection", latency_ms, ok);
    ping_result?;

    *state.pool.lock().await = Some(pool);
    Ok(ConnectionTestResult {
        latency_ms,
        ok: true,
    })
}

/// Background warm-up (ROADMAP Phase 2): loads the stored settings and
/// opens the read-only pool, forcing a real connection with a ping, so the
/// first operator query never pays connect latency.
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
    let ok = match warm_up(state).await {
        Ok(()) => true,
        Err(_) => false,
    };
    state
        .stats
        .lock()
        .await
        .record("warm_up_pool", started.elapsed().as_millis() as u64, ok);
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

/// Returns the read-only pool, connecting on first use.
///
/// Prefers the pool already built by [`test_connection`]; when none exists,
/// the stored encrypted settings are loaded (and decrypted) to open a fresh
/// one. Error messages stay generic — no hosts, users, or parameter values.
async fn acquire_pool(state: &AppState) -> Result<MySqlPool, String> {
    if let Some(pool) = state.pool.lock().await.clone() {
        return Ok(pool);
    }
    let store = load_vault()
        .map_err(|err| dev_log("load_vault", &err, "ไม่สามารถเข้าถึงที่เก็บกุญแจของระบบได้"))?;
    let cfg = load(&state.config_path(), &store)
        .await
        .map_err(|err| {
            dev_log(
                "load_config",
                &err,
                "อ่านการตั้งค่าการเชื่อมต่อไม่สำเร็จ กรุณาตั้งค่าใหม่",
            )
        })?
        .ok_or_else(|| "ยังไม่ได้ตั้งค่าการเชื่อมต่อ HOSxP".to_string())?;
    let pool = pool::connect(&cfg)
        .await
        .map_err(|err| dev_log("pool::connect", &err, "เชื่อมต่อฐานข้อมูล HOSxP ไม่สำเร็จ"))?;
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
) -> Result<Vec<PatientSummary>, String> {
    let term = term.trim().to_string();
    if term.is_empty() {
        return Ok(Vec::new());
    }
    let stats = state.stats.clone();
    timed(&stats, "search_patients", async move {
        let pool = acquire_pool(&state).await?;
        let repo = HosxRepository::new(pool);
        repo.search_patients(&term, detect_query_kind(&term))
            .await
            .map_err(|err| map_repo_error(err, "ค้นหาผู้ป่วย"))
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
) -> Result<Vec<DrugItem>, String> {
    let term = term.trim().to_string();
    if term.is_empty() {
        return Ok(Vec::new());
    }
    let stats = state.stats.clone();
    timed(&stats, "search_drugs", async move {
        let pool = acquire_pool(&state).await?;
        let repo = HosxRepository::new(pool);
        repo.search_drugs(&term)
            .await
            .map_err(|err| map_repo_error(err, "ค้นหายา"))
    })
    .await
}

/// Medication history for one patient + drug (AGENTS.md §7.2, milestone M4).
///
/// OPD + IPD are queried concurrently on the backend and merged
/// most-recent-first. The verdict contract (ROADMAP Phase 1): an exact drug
/// hit yields [`HistoryVerdict::Resolved`] (possibly empty — a legitimate
/// "ไม่พบประวัติ"); an unresolvable term yields
/// [`HistoryVerdict::Unresolved`] with disambiguation candidates. The
/// frontend decides the visual verdict. Errors are generic.
#[tauri::command]
pub async fn fetch_drug_history(
    state: State<'_, AppState>,
    hn: String,
    drug: String,
) -> Result<HistoryVerdict, String> {
    let stats = state.stats.clone();
    timed(&stats, "fetch_drug_history", async move {
        let pool = acquire_pool(&state).await?;
        let repo = HosxRepository::new(pool);
        repo.fetch_drug_history(&hn, &drug)
            .await
            .map_err(|err| map_repo_error(err, "ตรวจสอบประวัติ"))
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_error_is_translated_independently_of_action() {
        let msg = map_repo_error(RepositoryError::Connection, "ค้นหาผู้ป่วย");
        assert_eq!(msg, "เชื่อมต่อฐานข้อมูล HOSxP ไม่สำเร็จ");
    }

    #[test]
    fn guard_error_is_translated_independently_of_action() {
        let msg = map_repo_error(RepositoryError::Guard, "ค้นหายา");
        assert_eq!(msg, "ระบบความปลอดภัยของแอปปฏิเสธคำสั่งนี้");
    }

    #[test]
    fn query_error_renders_the_action_that_failed() {
        let msg = map_repo_error(
            RepositoryError::Query("row not found".to_string()),
            "ตรวจสอบประวัติ",
        );
        assert_eq!(msg, "ตรวจสอบประวัติไม่สำเร็จ");
    }
}
