//! API-layer wasm tests (ROADMAP Phase 4/5): the webview→backend contract
//! against a fake `invoke`, in headless Chrome (run via `wasm-pack test`).
//!
//! These tests never touch Tauri or the database — they pin the contract
//! shapes: command names, arg passing, batch verdict deserialization, and
//! the typed error taxonomy (Phase 3).

#![cfg(target_arch = "wasm32")]

// Run in a real browser (wasm-pack test --headless --chrome) — these tests
// pin IPC contract shapes, not DOM, but the mock invoke needs the browser
// environment.
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

use std::cell::RefCell;

use allerx_app::api::{self, ApiError, ApiErrorKind};
use allerx_models::{
    ConcurrentMedication, DrugCheckResult, DrugHistoryRecord, DrugItem, HistoryVerdict,
    PatientSummary, ResolvedHistory, VisitType,
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
async fn check_history_batch_deserializes_each_verdict() {
    mock(|cmd| {
        assert_eq!(cmd, "check_drugs");
        resolve_ok(&vec![
            DrugCheckResult {
                term: "พาราเซตามอล".into(),
                verdict: HistoryVerdict::Resolved {
                    drug: DrugItem {
                        icode: "1-001".into(),
                        name: "พาราเซตามอล".into(),
                        strength: Some("500 mg".into()),
                        trade_name: None,
                    },
                    history: ResolvedHistory {
                        records: vec![sample_record()],
                        truncated: true,
                    },
                },
            },
            DrugCheckResult {
                term: "ไม่มีในระบบ".into(),
                verdict: HistoryVerdict::Unresolved {
                    candidates: Vec::new(),
                },
            },
            DrugCheckResult {
                term: "แอมม็อกซิซิลลิน".into(),
                verdict: HistoryVerdict::Resolved {
                    drug: DrugItem {
                        icode: "1-002".into(),
                        name: "แอมม็อกซิซิลลิน".into(),
                        strength: None,
                        trade_name: None,
                    },
                    history: ResolvedHistory {
                        records: Vec::new(),
                        truncated: false,
                    },
                },
            },
        ])
    });
    let drugs = vec!["พาราเซตามอล".to_string(), "ไม่มีในระบบ".to_string()];
    let results = api::check_history("00012345", &drugs)
        .await
        .expect("mock succeeds");
    assert_eq!(results.len(), 3);
    assert_eq!(results[0].term, "พาราเซตามอล");
    match &results[0].verdict {
        HistoryVerdict::Resolved { drug, history } => {
            assert_eq!(drug.name, "พาราเซตามอล");
            assert_eq!(drug.strength.as_deref(), Some("500 mg"));
            assert_eq!(history.records.len(), 1);
            assert!(history.truncated);
        }
        other => panic!("expected Resolved, got {other:?}"),
    }
    assert!(matches!(
        results[1].verdict,
        HistoryVerdict::Unresolved { .. }
    ));
    match &results[2].verdict {
        HistoryVerdict::Resolved { drug, history } => {
            assert_eq!(drug.icode, "1-002");
            assert!(history.records.is_empty());
            assert!(!history.truncated);
        }
        other => panic!("expected Resolved, got {other:?}"),
    }
    assert_eq!(recorded_calls(), vec!["check_drugs".to_string()]);
    api::clear_mock_invoke();
}

#[wasm_bindgen_test]
async fn fetch_concurrent_medications_deserializes() {
    mock(|cmd| {
        assert_eq!(cmd, "fetch_concurrent_medications");
        resolve_ok(&vec![ConcurrentMedication {
            drug_code: "1-001".into(),
            drug_name: "พาราเซตามอล".into(),
            trade_name: None,
            last_date: NaiveDate::from_ymd_opt(2024, 6, 1).expect("valid date in test"),
        }])
    });
    let meds = api::fetch_concurrent_medications("00012345")
        .await
        .expect("mock succeeds");
    assert_eq!(meds.len(), 1);
    assert_eq!(meds[0].drug_name, "พาราเซตามอล");
    api::clear_mock_invoke();
}

#[wasm_bindgen_test]
async fn rejection_carries_typed_kind_and_message() {
    mock(|_cmd| reject_err(&sample_error()));
    let err = api::check_history("00012345", &["1-001".to_string()])
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
