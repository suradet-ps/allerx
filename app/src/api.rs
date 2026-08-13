//! Frontend API layer — the only place the webview talks to the Tauri
//! backend (AGENTS.md §1: the frontend only ever talks to the database
//! through the Tauri backend).
//!
//! No hosts, no credentials, no SQL live here — every call is a thin
//! `invoke` to a Rust command, which owns all connection concerns.
//!
//! Errors cross the IPC as a typed [`ApiError`] (kind + Thai message,
//! ROADMAP Phase 3): components switch on `kind` (e.g. to raise the
//! connection banner) and display `message` verbatim.

use std::cell::RefCell;
use std::collections::HashMap;

use allerx_models::{ConcurrentMedication, DrugCheckResult, DrugItem, PatientSummary};
use js_sys::{Object, Reflect};
use serde_wasm_bindgen::{from_value, to_value};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

use crate::state::ConnectionHealth;

/// Bridge to the Tauri IPC: `invoke(cmd, args)` resolves to the command's
/// return value, or rejects with its serialized error.
///
/// Tests replace this with [`install_mock_invoke`] — in a headless-browser
/// test page `__TAURI_INTERNALS__` does not exist, so a mock is always
/// installed before any API call in tests.
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["__TAURI_INTERNALS__"])]
    fn invoke(cmd: &str, args: JsValue) -> js_sys::Promise;
}

/// A fake `invoke` implementation: command name + args in, promise out.
type MockInvoke = Box<dyn Fn(&str, JsValue) -> js_sys::Promise>;

// Per-thread invoke override (ROADMAP Phase 4). wasm tests are
// single-threaded, so `thread_local!` is the right scope; the mock is
// consulted before the real Tauri bridge.
thread_local! {
    static MOCK_INVOKE: RefCell<Option<MockInvoke>> = RefCell::new(None);
}

/// Installs a fake `invoke` implementation for tests: `mock(cmd, args)` is
/// called instead of the Tauri bridge and must return a promise resolving
/// with the serialized command result (or rejecting with a serialized
/// [`ApiError`]).
pub fn install_mock_invoke(mock: impl Fn(&str, JsValue) -> js_sys::Promise + 'static) {
    MOCK_INVOKE.with(|m| *m.borrow_mut() = Some(Box::new(mock)));
}

/// Removes any installed mock — restores the real bridge.
pub fn clear_mock_invoke() {
    MOCK_INVOKE.with(|m| *m.borrow_mut() = None);
}

fn call_invoke(cmd: &str, args: JsValue) -> js_sys::Promise {
    MOCK_INVOKE.with(|m| {
        if let Some(mock) = m.borrow().as_ref() {
            mock(cmd, args)
        } else {
            invoke(cmd, args)
        }
    })
}

/// Failure class of a backend command — mirrors the Rust
/// `CommandErrorKind` (camelCase over the wire).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ApiErrorKind {
    NotConfigured,
    Connection,
    Guard,
    Query,
}

/// A backend command failure: machine-readable kind + the Thai message to
/// show verbatim. Never decide presentation by matching message text.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiError {
    pub kind: ApiErrorKind,
    pub message: String,
}

/// Plaintext connection settings, typed by the operator in the settings
/// dialog. The plaintext lives in the webview's JS heap while the operator
/// types — a documented, unavoidable window (AGENTS.md §9); the Rust side
/// encrypts it before anything touches disk.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionInput {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub user: String,
    pub password: String,
}

/// Result of the backend's `SELECT 1` smoke test (latency only; the
/// backend's `ok` field is implied by a successful call).
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionTestResult {
    pub latency_ms: u64,
}

/// Calls a Tauri command with no arguments and deserializes its JSON result.
async fn call_empty<T>(cmd: &str) -> Result<T, ApiError>
where
    T: serde::de::DeserializeOwned,
{
    let args = to_value(&HashMap::<String, String>::new()).map_err(|_| ApiError {
        kind: ApiErrorKind::Query,
        message: "สร้างคำขอไม่สำเร็จ".to_string(),
    })?;
    call_raw(cmd, args).await
}

/// Calls a Tauri command with a single string argument.
async fn call_string_arg<T>(cmd: &str, arg_name: &str, value: &str) -> Result<T, ApiError>
where
    T: serde::de::DeserializeOwned,
{
    let mut args = HashMap::new();
    args.insert(arg_name, value);
    call_raw(
        cmd,
        to_value(&args).map_err(|_| ApiError {
            kind: ApiErrorKind::Query,
            message: "สร้างคำขอไม่สำเร็จ".to_string(),
        })?,
    )
    .await
}

/// Calls a Tauri command with a serializable argument object.
///
/// `arg_name` must equal the command's Rust parameter name — Tauri 2 looks
/// up arguments by that key (e.g. `input` for `configure_connection(input)`).
async fn call_struct_arg<T>(
    cmd: &str,
    arg_name: &str,
    arg: &impl serde::Serialize,
) -> Result<T, ApiError>
where
    T: serde::de::DeserializeOwned,
{
    let arg_value = to_value(arg).map_err(|_| ApiError {
        kind: ApiErrorKind::Query,
        message: "สร้างคำขอไม่สำเร็จ".to_string(),
    })?;
    let args = Object::new();
    Reflect::set(&args, &JsValue::from_str(arg_name), &arg_value).map_err(|_| ApiError {
        kind: ApiErrorKind::Query,
        message: "สร้างคำขอไม่สำเร็จ".to_string(),
    })?;
    call_raw(cmd, args.into()).await
}

/// Core invoke path: resolves the promise, then deserializes the result.
/// The rejection carries the backend's typed error — surface its message
/// verbatim instead of hiding the cause. A rejection that does not
/// deserialize as [`ApiError`] (an internal IPC problem) becomes a generic
/// Query error.
async fn call_raw<T>(cmd: &str, args: JsValue) -> Result<T, ApiError>
where
    T: serde::de::DeserializeOwned,
{
    let promise = call_invoke(cmd, args);
    let value = JsFuture::from(promise).await.map_err(|err| {
        from_value::<ApiError>(err).unwrap_or(ApiError {
            kind: ApiErrorKind::Query,
            message: "การเชื่อมต่อกับโปรแกรมหลักล้มเหลว".to_string(),
        })
    })?;
    from_value(value).map_err(|_| ApiError {
        kind: ApiErrorKind::Query,
        message: "อ่านผลลัพธ์ไม่สำเร็จ".to_string(),
    })
}

/// Whether encrypted HOSxP connection settings already exist on this machine.
pub async fn connection_status() -> bool {
    call_empty::<bool>("connection_status")
        .await
        .unwrap_or(false)
}

/// Polled live reachability (ROADMAP Phase 3) — the status dot source.
pub async fn connection_health() -> Result<ConnectionHealth, ApiError> {
    call_empty("connection_health").await
}

/// Encrypts and saves HOSxP connection settings via the backend.
pub async fn configure_connection(input: &ConnectionInput) -> Result<(), ApiError> {
    call_struct_arg("configure_connection", "input", input).await
}

/// Runs the backend `SELECT 1` smoke test with the **typed** settings (not
/// the saved ones); returns latency in milliseconds.
pub async fn test_connection(input: &ConnectionInput) -> Result<u64, ApiError> {
    let result: ConnectionTestResult = call_struct_arg("test_connection", "input", input).await?;
    Ok(result.latency_ms)
}

/// Patient search by HN / CID / name via the backend (M2).
///
/// Returns `Err` only for genuine failures (no connection, not configured,
/// query failed) — an empty list is a legitimate "no matching patients".
pub async fn search_patients(term: &str) -> Result<Vec<PatientSummary>, ApiError> {
    call_string_arg("search_patients", "term", term).await
}

/// Drug autocomplete from the backend `drugitems` table (M3).
pub async fn search_drugs(prefix: &str) -> Result<Vec<DrugItem>, ApiError> {
    call_string_arg("search_drugs", "term", prefix).await
}

/// Full medication history for a patient + one or more drugs (ROADMAP
/// Phase 5 — a single drug is a batch of one). The backend checks each drug
/// concurrently and merges OPD+IPD most-recent-first. The per-drug
/// three-state contract (ROADMAP Phase 1): `Resolved` with empty records is
/// a legitimate "no history"; `Unresolved` means the term could not be
/// matched to the formulary and carries disambiguation candidates — the UI
/// must never show "ไม่พบประวัติ" for it.
pub async fn check_history(hn: &str, drugs: &[String]) -> Result<Vec<DrugCheckResult>, ApiError> {
    let args = Object::new();
    Reflect::set(
        &args,
        &JsValue::from_str("hn"),
        &to_value(hn).map_err(|_| ApiError {
            kind: ApiErrorKind::Query,
            message: "สร้างคำขอไม่สำเร็จ".to_string(),
        })?,
    )
    .map_err(|_| ApiError {
        kind: ApiErrorKind::Query,
        message: "สร้างคำขอไม่สำเร็จ".to_string(),
    })?;
    Reflect::set(
        &args,
        &JsValue::from_str("drugs"),
        &to_value(drugs).map_err(|_| ApiError {
            kind: ApiErrorKind::Query,
            message: "สร้างคำขอไม่สำเร็จ".to_string(),
        })?,
    )
    .map_err(|_| ApiError {
        kind: ApiErrorKind::Query,
        message: "สร้างคำขอไม่สำเร็จ".to_string(),
    })?;
    call_raw("check_drugs", args.into()).await
}

/// Recent concurrent medications for a patient (ROADMAP Phase 5) — the
/// "ยาที่ได้รับล่าสุด" snapshot in the patient detail view.
pub async fn fetch_concurrent_medications(hn: &str) -> Result<Vec<ConcurrentMedication>, ApiError> {
    call_string_arg("fetch_concurrent_medications", "hn", hn).await
}
