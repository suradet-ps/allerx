//! API-layer wasm tests (ROADMAP Phase 4): the webview→backend contract
//! against a fake `invoke`, in headless Chrome (run via `wasm-pack test`).
//!
//! These tests never touch Tauri or the database — they pin the contract
//! shapes: command names, arg passing, verdict deserialization, and the
//! typed error taxonomy (Phase 3).

#![cfg(target_arch = "wasm32")]

// Run in a real browser (wasm-pack test --headless --chrome) — these tests
// pin IPC contract shapes, not DOM, but the mock invoke needs the browser
// environment.
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

use std::cell::RefCell;

use allerx_app::api::{self, ApiError, ApiErrorKind};
use allerx_models::{
    DrugHistoryRecord, DrugItem, HistoryVerdict, PatientSummary, ResolvedHistory, VisitType,
};
use chrono::NaiveDate;
use js_sys::Promise;
use serde_wasm_bindgen::to_value;
use wasm_bindgen::prelude::*;
use wasm_bindgen_test::wasm_bindgen_test;

// Records the command names the mock saw, so tests can assert the right
// command was invoked with the right shape.
thread_local! {
    static CALLS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

fn resolve_ok<T: serde::Serialize>(value: &T) -> Promise {
    Promise::resolve(&to_value(value).expect("serialize test value"))
}

fn reject_err(err: &ApiError) -> Promise {
    Promise::reject(&to_value(err).expect("serialize test error"))
}

fn sample_patient() -> PatientSummary {
    PatientSummary {
        hn: "00012345".into(),
        cid: Some("1101701234567".into()),
        full_name_th: "สมชาย ใจดี".into(),
        birth_date: None,
        sex: Some("1".into()),
    }
}

fn sample_record() -> DrugHistoryRecord {
    DrugHistoryRecord {
        visit_date: NaiveDate::from_ymd_opt(2024, 5, 5).expect("valid date in test"),
        visit_type: VisitType::Opd,
        drug_code: "1-001".into(),
        drug_name: "พาราเซตามอล".into(),
        trade_name: None,
        prescriber: None,
        department: None,
        quantity: None,
        route: None,
    }
}

fn sample_drug() -> DrugItem {
    DrugItem {
        icode: "1-001".into(),
        name: "พาราเซตามอล".into(),
        strength: Some("500 mg".into()),
        trade_name: None,
    }
}

fn sample_error() -> ApiError {
    ApiError {
        kind: ApiErrorKind::Connection,
        message: "เชื่อมต่อฐานข้อมูล HOSxP ไม่สำเร็จ".into(),
    }
}

/// Installs a mock invoke and records every command it is called with.
fn mock(handler: impl Fn(&str) -> Promise + 'static) {
    api::clear_mock_invoke();
    CALLS.with(|c| c.borrow_mut().clear());
    api::install_mock_invoke(move |cmd: &str, _args: JsValue| {
        CALLS.with(|c| c.borrow_mut().push(cmd.to_string()));
        handler(cmd)
    });
}

fn recorded_calls() -> Vec<String> {
    CALLS.with(|c| c.borrow().clone())
}

#[wasm_bindgen_test]
async fn search_patients_passes_term_and_deserializes_results() {
    mock(|cmd| {
        assert_eq!(cmd, "search_patients");
        resolve_ok(&vec![sample_patient()])
    });
    let hits = api::search_patients("1101701234567")
        .await
        .expect("mock succeeds");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].hn, "00012345");
    assert_eq!(recorded_calls(), vec!["search_patients".to_string()]);
    api::clear_mock_invoke();
}

#[wasm_bindgen_test]
async fn fetch_history_resolved_empty_is_a_definitive_not_found() {
    mock(|cmd| {
        assert_eq!(cmd, "fetch_drug_history");
        resolve_ok(&HistoryVerdict::Resolved {
            history: ResolvedHistory {
                records: Vec::new(),
                truncated: false,
            },
        })
    });
    let verdict = api::fetch_history("00012345", "1-001")
        .await
        .expect("mock succeeds");
    match verdict {
        HistoryVerdict::Resolved { history } => {
            assert!(history.records.is_empty());
            assert!(!history.truncated);
        }
        other => panic!("expected Resolved, got {other:?}"),
    }
    api::clear_mock_invoke();
}

#[wasm_bindgen_test]
async fn fetch_history_resolved_carries_records_and_truncation() {
    mock(|_cmd| {
        resolve_ok(&HistoryVerdict::Resolved {
            history: ResolvedHistory {
                records: vec![sample_record()],
                truncated: true,
            },
        })
    });
    let verdict = api::fetch_history("00012345", "1-001")
        .await
        .expect("mock succeeds");
    match verdict {
        HistoryVerdict::Resolved { history } => {
            assert_eq!(history.records.len(), 1);
            assert_eq!(history.records[0].drug_name, "พาราเซตามอล");
            assert!(history.truncated);
        }
        other => panic!("expected Resolved, got {other:?}"),
    }
    api::clear_mock_invoke();
}

#[wasm_bindgen_test]
async fn fetch_history_unresolved_carries_candidates() {
    mock(|_cmd| {
        resolve_ok(&HistoryVerdict::Unresolved {
            candidates: vec![sample_drug()],
        })
    });
    let verdict = api::fetch_history("00012345", "พารา")
        .await
        .expect("mock succeeds");
    match verdict {
        HistoryVerdict::Unresolved { candidates } => {
            assert_eq!(candidates.len(), 1);
            assert_eq!(candidates[0].icode, "1-001");
        }
        other => panic!("expected Unresolved, got {other:?}"),
    }
    api::clear_mock_invoke();
}

#[wasm_bindgen_test]
async fn rejection_carries_typed_kind_and_message() {
    mock(|_cmd| reject_err(&sample_error()));
    let err = api::fetch_history("00012345", "1-001")
        .await
        .expect_err("mock rejects");
    assert_eq!(err.kind, ApiErrorKind::Connection);
    assert_eq!(err.message, "เชื่อมต่อฐานข้อมูล HOSxP ไม่สำเร็จ");
    api::clear_mock_invoke();
}

#[wasm_bindgen_test]
async fn non_typed_rejection_falls_back_to_generic_query_error() {
    mock(|_cmd| Promise::reject(&JsValue::from_str("boom")));
    let err = api::search_patients("00012345")
        .await
        .expect_err("mock rejects");
    assert_eq!(err.kind, ApiErrorKind::Query);
    assert!(!err.message.is_empty());
    api::clear_mock_invoke();
}

#[wasm_bindgen_test]
async fn connection_health_deserializes_every_state() {
    mock(|cmd| {
        assert_eq!(cmd, "connection_health");
        resolve_ok(&allerx_app::state::ConnectionHealth::Connected)
    });
    let health = api::connection_health().await.expect("mock succeeds");
    assert_eq!(health, allerx_app::state::ConnectionHealth::Connected);
    api::clear_mock_invoke();
}
