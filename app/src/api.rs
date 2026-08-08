//! Frontend API layer — the only place the webview talks to the Tauri
//! backend (AGENTS.md §1: the frontend only ever talks to the database
//! through the Tauri backend).
//!
//! No hosts, no credentials, no SQL live here — every call is a thin
//! `invoke` to a Rust command, which owns all connection concerns.

use std::collections::HashMap;

use allerx_models::{DrugHistoryRecord, DrugItem, PatientSummary};
use js_sys::{Object, Reflect};
use serde_wasm_bindgen::{from_value, to_value};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

/// Bridge to the Tauri IPC: `invoke(cmd, args)` resolves to the command's
/// return value, or rejects with its error string.
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["__TAURI_INTERNALS__"])]
    fn invoke(cmd: &str, args: JsValue) -> js_sys::Promise;
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
async fn call_empty<T>(cmd: &str) -> Result<T, String>
where
    T: serde::de::DeserializeOwned,
{
    let args =
        to_value(&HashMap::<String, String>::new()).map_err(|_| "สร้างคำขอไม่สำเร็จ".to_string())?;
    call_raw(cmd, args).await
}

/// Calls a Tauri command with a single string argument.
async fn call_string_arg<T>(cmd: &str, arg_name: &str, value: &str) -> Result<T, String>
where
    T: serde::de::DeserializeOwned,
{
    let mut args = HashMap::new();
    args.insert(arg_name, value);
    call_raw(
        cmd,
        to_value(&args).map_err(|_| "สร้างคำขอไม่สำเร็จ".to_string())?,
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
) -> Result<T, String>
where
    T: serde::de::DeserializeOwned,
{
    let arg_value = to_value(arg).map_err(|_| "สร้างคำขอไม่สำเร็จ".to_string())?;
    let args = Object::new();
    Reflect::set(&args, &JsValue::from_str(arg_name), &arg_value)
        .map_err(|_| "สร้างคำขอไม่สำเร็จ".to_string())?;
    call_raw(cmd, args.into()).await
}

/// Core invoke path: resolves the promise, then deserializes the result.
/// The rejection carries the backend's own (already generic, PII-free) error
/// message — surface it verbatim instead of hiding the cause.
async fn call_raw<T>(cmd: &str, args: JsValue) -> Result<T, String>
where
    T: serde::de::DeserializeOwned,
{
    let promise = invoke(cmd, args);
    let value = JsFuture::from(promise).await.map_err(|err| {
        err.as_string()
            .unwrap_or_else(|| "การเชื่อมต่อกับโปรแกรมหลักล้มเหลว".to_string())
    })?;
    from_value(value).map_err(|_| "อ่านผลลัพธ์ไม่สำเร็จ".to_string())
}

/// Whether encrypted HOSxP connection settings already exist on this machine.
pub async fn connection_status() -> bool {
    call_empty::<bool>("connection_status")
        .await
        .unwrap_or(false)
}

/// Encrypts and saves HOSxP connection settings via the backend.
pub async fn configure_connection(input: &ConnectionInput) -> Result<(), String> {
    call_struct_arg("configure_connection", "input", input).await
}

/// Runs the backend `SELECT 1` smoke test with the **typed** settings (not
/// the saved ones); returns latency in milliseconds.
pub async fn test_connection(input: &ConnectionInput) -> Result<u64, String> {
    let result: ConnectionTestResult =
        call_struct_arg("test_connection", "input", input).await?;
    Ok(result.latency_ms)
}

/// Patient search by HN / CID / name via the backend (M2).
///
/// Returns `Err` only for genuine failures (no connection, not configured,
/// query failed) — an empty list is a legitimate "no matching patients".
pub async fn search_patients(term: &str) -> Result<Vec<PatientSummary>, String> {
    call_string_arg("search_patients", "term", term).await
}

/// Drug autocomplete from the backend `drugitems` table (M3).
pub async fn search_drugs(prefix: &str) -> Result<Vec<DrugItem>, String> {
    call_string_arg("search_drugs", "term", prefix).await
}

/// Full medication history for a patient + drug, merged most-recent-first
/// (M4). An empty list is a legitimate "no history found".
pub async fn fetch_history(hn: &str, drug: &str) -> Result<Vec<DrugHistoryRecord>, String> {
    let mut args = HashMap::new();
    args.insert("hn", hn);
    args.insert("drug", drug);
    call_raw(
        "fetch_drug_history",
        to_value(&args).map_err(|_| "สร้างคำขอไม่สำเร็จ".to_string())?,
    )
    .await
}
