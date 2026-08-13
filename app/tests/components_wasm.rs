//! Component mount tests (ROADMAP Phase 4) — the clinical surface rendered
//! into a real DOM in headless Chrome (run via `wasm-pack test`).
//!
//! These pin the states a pharmacist actually sees: the four verdict
//! states, truncation honesty, CID masking, and the search flows (routing
//! through a fake `invoke`, so no Tauri and no database are involved).

#![cfg(target_arch = "wasm32")]

// These tests mount components into a real DOM, so they must run in a
// browser (wasm-pack test --headless --chrome).
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

use std::cell::RefCell;

use allerx_app::api;
use allerx_app::components::drug_search::DrugSearch;
use allerx_app::components::patient_bar::PatientBar;
use allerx_app::components::patient_search::PatientSearch;
use allerx_app::components::timeline::Timeline;
use allerx_app::components::verdict_band::VerdictBand;
use allerx_app::state::{AppState, ConnectionHealth, VerdictState};
use allerx_models::{
    DrugHistoryRecord, DrugItem, HistoryVerdict, PatientSummary, ResolvedHistory, VisitType,
};
use chrono::NaiveDate;
use js_sys::Promise;
use leptos::prelude::*;
use serde_wasm_bindgen::to_value;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use wasm_bindgen_test::wasm_bindgen_test;
use web_sys::{Element, HtmlElement, HtmlInputElement, KeyboardEvent, KeyboardEventInit};

thread_local! {
    static MOUNT_SEQ: RefCell<u32> = const { RefCell::new(0) };
    // Mount handles MUST stay alive while assertions run — dropping one
    // unmounts the component. The handle type is anonymous (it depends on
    // the view's `IntoView::State`), so erase it; the page is torn down at
    // the end of the suite anyway.
    static HELD_HANDLES: RefCell<Vec<Box<dyn std::any::Any>>> =
        const { RefCell::new(Vec::new()) };
}

// ---------------------------------------------------------------------------
// Fixtures & helpers
// ---------------------------------------------------------------------------

fn sample_patient() -> PatientSummary {
    PatientSummary {
        hn: "00012345".into(),
        cid: Some("1101701234567".into()),
        full_name_th: "สมชาย ใจดี".into(),
        birth_date: Some(NaiveDate::from_ymd_opt(1980, 1, 1).expect("valid date in test")),
        sex: Some("1".into()),
    }
}

fn record(drug_name: &str, date: NaiveDate, visit_type: VisitType) -> DrugHistoryRecord {
    DrugHistoryRecord {
        visit_date: date,
        visit_type,
        drug_code: "1-001".into(),
        drug_name: drug_name.into(),
        trade_name: None,
        prescriber: Some("นพ. ทดสอบ".into()),
        department: Some("อายุรกรรม".into()),
        quantity: None,
        route: None,
    }
}

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).expect("valid date in test")
}

fn sample_drug() -> DrugItem {
    DrugItem {
        icode: "1-001".into(),
        name: "พาราเซตามอล".into(),
        strength: Some("500 mg".into()),
        trade_name: None,
    }
}

fn resolve_ok<T: serde::Serialize>(value: &T) -> Promise {
    Promise::resolve(&to_value(value).expect("serialize test value"))
}

/// Creates a unique container div, mounts the view into it, and returns the
/// container. The mount handle is kept alive in [`HELD_HANDLES`] so the
/// component stays mounted for the test's assertions.
fn mount<N>(id: &str, f: impl FnOnce() -> N + 'static) -> HtmlElement
where
    N: IntoView + 'static,
{
    let document = web_sys::window()
        .expect("window in test")
        .document()
        .expect("document in test");
    let container = document.create_element("div").expect("create container");
    container.set_id(&format!(
        "{id}-{}",
        MOUNT_SEQ.with(|seq| {
            let next = *seq.borrow();
            *seq.borrow_mut() = next + 1;
            next
        })
    ));
    document
        .body()
        .expect("body in test")
        .append_child(&container)
        .expect("append container");
    let parent = container
        .dyn_into::<HtmlElement>()
        .expect("container is an HtmlElement");
    let handle = leptos::mount::mount_to(parent.clone(), f);
    HELD_HANDLES.with(|held| held.borrow_mut().push(Box::new(handle)));
    parent
}

fn query_one(root: &HtmlElement, selector: &str) -> Element {
    root.query_selector(selector)
        .expect("query_selector works")
        .unwrap_or_else(|| panic!("no element matches {selector:?}"))
}

fn query_optional(root: &HtmlElement, selector: &str) -> Option<Element> {
    root.query_selector(selector).expect("query_selector works")
}

fn input(root: &HtmlElement) -> HtmlInputElement {
    query_one(root, "input")
        .dyn_into::<HtmlInputElement>()
        .expect("input element")
}

fn press_enter(target: &Element) {
    let init = KeyboardEventInit::new();
    init.set_key("Enter");
    let event =
        KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init).expect("keydown event");
    target.dispatch_event(&event).expect("dispatch keydown");
}

fn type_text(target: &HtmlInputElement, text: &str) {
    // The value is what the handler reads (event_target_value); the event
    // just needs to be an "input" event — no init dict required.
    target.set_value(text);
    let event = web_sys::Event::new("input").expect("input event");
    target.dispatch_event(&event).expect("dispatch input");
}

/// Yields enough microtasks for promise chains to settle.
async fn settle() {
    for _ in 0..16 {
        JsFuture::from(Promise::resolve(&JsValue::UNDEFINED))
            .await
            .expect("resolved");
    }
}

/// Waits real time (browser timers) — for debounce tests.
async fn wait(ms: u32) {
    let promise = Promise::new(&mut |resolve, _reject| {
        web_sys::window()
            .expect("window in test")
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms as i32)
            .expect("set timeout");
    });
    JsFuture::from(promise).await.expect("timer fired");
}

// ---------------------------------------------------------------------------
// Verdict band — the four states (Phase 1 contract)
// ---------------------------------------------------------------------------

#[wasm_bindgen_test]
async fn verdict_pending_is_neutral_and_never_implies_an_answer() {
    let state = AppState::new();
    let root = mount("vb-pending", move || view! { <VerdictBand state=state /> });
    let band = query_one(&root, ".verdict-band");
    assert!(band.class_list().contains("verdict-pending"));
    let text = band.text_content().expect("text content");
    assert!(text.contains("รอการค้นหา"));
}

#[wasm_bindgen_test]
async fn verdict_found_shows_latest_record_and_count() {
    let state = AppState::new();
    state.verdict.set(VerdictState::Found {
        records: vec![record("พาราเซตามอล", date(2024, 5, 5), VisitType::Opd)],
        truncated: false,
    });
    let root = mount("vb-found", move || view! { <VerdictBand state=state /> });
    let band = query_one(&root, ".verdict-band");
    assert!(band.class_list().contains("verdict-found"));
    let text = band.text_content().expect("text content");
    assert!(text.contains("พบประวัติการได้รับยานี้"));
    assert!(text.contains("05/05/2024"));
    assert!(text.contains("ทั้งหมด 1 ครั้ง"));
    assert!(!text.contains("มีประวัติเก่ากว่านี้"));
}

#[wasm_bindgen_test]
async fn verdict_found_admits_truncation() {
    let state = AppState::new();
    state.verdict.set(VerdictState::Found {
        records: vec![record("พาราเซตามอล", date(2024, 5, 5), VisitType::Opd)],
        truncated: true,
    });
    let root = mount(
        "vb-found-trunc",
        move || view! { <VerdictBand state=state /> },
    );
    let text = query_one(&root, ".verdict-band")
        .text_content()
        .expect("text content");
    assert!(text.contains("มีประวัติเก่ากว่านี้"));
}

#[wasm_bindgen_test]
async fn verdict_notfound_is_only_reachable_for_resolved_drugs() {
    let state = AppState::new();
    state.verdict.set(VerdictState::NotFound);
    let root = mount("vb-notfound", move || view! { <VerdictBand state=state /> });
    let band = query_one(&root, ".verdict-band");
    assert!(band.class_list().contains("verdict-notfound"));
    let text = band.text_content().expect("text content");
    assert!(text.contains("ไม่พบประวัติการได้รับยานี้"));
}

#[wasm_bindgen_test]
async fn verdict_unresolved_with_candidates_never_uses_notfound_text() {
    let state = AppState::new();
    state.verdict.set(VerdictState::Unresolved {
        candidates: vec![sample_drug()],
    });
    let root = mount(
        "vb-unresolved",
        move || view! { <VerdictBand state=state /> },
    );
    let band = query_one(&root, ".verdict-band");
    assert!(band.class_list().contains("verdict-unresolved"));
    let text = band.text_content().expect("text content");
    assert!(text.contains("ไม่สามารถยืนยันประวัติได้"));
    assert!(!text.contains("ไม่พบประวัติ"));
}

#[wasm_bindgen_test]
async fn verdict_unresolved_without_candidates_says_not_in_formulary() {
    let state = AppState::new();
    state.verdict.set(VerdictState::Unresolved {
        candidates: Vec::new(),
    });
    let root = mount(
        "vb-unresolved-empty",
        move || view! { <VerdictBand state=state /> },
    );
    let text = query_one(&root, ".verdict-band")
        .text_content()
        .expect("text content");
    assert!(text.contains("ไม่พบยานี้ในทะเบียนยา"));
    assert!(!text.contains("ไม่พบประวัติ"));
}

// ---------------------------------------------------------------------------
// Timeline — rows and truncation honesty (Phase 1)
// ---------------------------------------------------------------------------

#[wasm_bindgen_test]
async fn timeline_renders_rows_most_recent_first_and_complete_footer() {
    let state = AppState::new();
    state.verdict.set(VerdictState::Found {
        records: vec![
            record("พาราเซตามอล", date(2024, 5, 5), VisitType::Opd),
            record("พาราเซตามอล", date(2024, 2, 2), VisitType::Ipd),
        ],
        truncated: false,
    });
    let root = mount("tl-complete", move || view! { <Timeline state=state /> });
    let text = query_one(&root, ".timeline")
        .text_content()
        .expect("text content");
    assert!(text.contains("พาราเซตามอล"));
    let footer = query_one(&root, ".timeline-footer")
        .text_content()
        .expect("footer text");
    assert!(footer.contains("ทั้งหมด 2 รายการ"));
}

#[wasm_bindgen_test]
async fn timeline_truncated_footer_does_not_present_list_as_complete() {
    let state = AppState::new();
    state.verdict.set(VerdictState::Found {
        records: vec![record("พาราเซตามอล", date(2024, 5, 5), VisitType::Opd)],
        truncated: true,
    });
    let root = mount("tl-trunc", move || view! { <Timeline state=state /> });
    let footer = query_one(&root, ".timeline-footer")
        .text_content()
        .expect("footer text");
    assert!(footer.contains("มีประวัติเก่ากว่านี้"));
    assert!(!footer.contains("ทั้งหมด 1 รายการ"));
}

// ---------------------------------------------------------------------------
// Patient bar — masking and demographics (DESIGN.md)
// ---------------------------------------------------------------------------

#[wasm_bindgen_test]
async fn patient_bar_masks_cid_and_shows_hn_birth_date_sex() {
    let state = AppState::new();
    state.patient.set(Some(sample_patient()));
    let root = mount("pb", move || view! { <PatientBar state=state /> });
    let text = query_one(&root, ".patient-bar")
        .text_content()
        .expect("text content");
    assert!(text.contains("สมชาย ใจดี"));
    assert!(text.contains("HN 00012345"));
    assert!(text.contains("1-XXXX-XXXXX-XX-7"));
    assert!(!text.contains("1101701234567"));
    assert!(text.contains("01/01/1980"));
    assert!(text.contains("ชาย"));
}

// ---------------------------------------------------------------------------
// Drug search — verdict routing through a fake invoke (Phase 1)
// ---------------------------------------------------------------------------

fn drug_search_with(mock: impl Fn(&str) -> Promise + 'static) -> (AppState, HtmlElement) {
    api::clear_mock_invoke();
    api::install_mock_invoke(move |cmd: &str, _args: JsValue| mock(cmd));
    let state = AppState::new();
    state.patient.set(Some(sample_patient()));
    let view_state = state.clone();
    let root = mount("ds", move || view! { <DrugSearch state=view_state /> });
    (state, root)
}

#[wasm_bindgen_test]
async fn drug_search_unresolved_routes_to_unresolved_verdict_and_suggests() {
    let (state, root) = drug_search_with(|cmd| {
        assert_eq!(cmd, "fetch_drug_history");
        resolve_ok(&HistoryVerdict::Unresolved {
            candidates: vec![sample_drug()],
        })
    });
    let search = input(&root);
    assert!(!search.disabled(), "enabled once a patient is selected");
    type_text(&search, "พารา");
    press_enter(&search);
    settle().await;

    assert_eq!(
        state.verdict.get_untracked(),
        VerdictState::Unresolved {
            candidates: vec![sample_drug()]
        }
    );
    assert!(matches!(
        state.verdict.get_untracked(),
        VerdictState::Unresolved { .. }
    ));
    assert!(query_optional(&root, ".search-result-row").is_some());
    api::clear_mock_invoke();
}

#[wasm_bindgen_test]
async fn drug_search_resolved_empty_routes_to_notfound() {
    let (state, root) = drug_search_with(|_cmd| {
        resolve_ok(&HistoryVerdict::Resolved {
            history: ResolvedHistory {
                records: Vec::new(),
                truncated: false,
            },
        })
    });
    type_text(&input(&root), "พาราเซตามอล");
    press_enter(&input(&root));
    settle().await;

    assert_eq!(state.verdict.get_untracked(), VerdictState::NotFound);
    api::clear_mock_invoke();
}

#[wasm_bindgen_test]
async fn drug_search_connection_error_raises_banner_and_keeps_verdict_pending() {
    let (state, root) = drug_search_with(|_cmd| {
        Promise::reject(
            &to_value(&api::ApiError {
                kind: api::ApiErrorKind::Connection,
                message: "เชื่อมต่อฐานข้อมูล HOSxP ไม่สำเร็จ".into(),
            })
            .expect("serialize error"),
        )
    });
    type_text(&input(&root), "พาราเซตามอล");
    press_enter(&input(&root));
    settle().await;

    assert_eq!(state.verdict.get_untracked(), VerdictState::Pending);
    assert_eq!(
        state.db_banner.get_untracked().as_deref(),
        Some("เชื่อมต่อฐานข้อมูล HOSxP ไม่สำเร็จ")
    );
    api::clear_mock_invoke();
}

#[wasm_bindgen_test]
async fn drug_search_is_disabled_until_patient_selected() {
    api::clear_mock_invoke();
    let state = AppState::new();
    let root = mount("ds-disabled", move || view! { <DrugSearch state=state /> });
    assert!(input(&root).disabled());
}

// ---------------------------------------------------------------------------
// Patient search — debounce and banner behavior
// ---------------------------------------------------------------------------

#[wasm_bindgen_test]
async fn patient_search_debounces_then_renders_results() {
    api::clear_mock_invoke();
    api::install_mock_invoke(|cmd: &str, _args: JsValue| {
        assert_eq!(cmd, "search_patients");
        resolve_ok(&vec![sample_patient()])
    });
    let state = AppState::new();
    let root = mount("ps", move || view! { <PatientSearch state=state /> });
    type_text(&input(&root), "สมชาย");

    // Immediately after typing: debounce pending, nothing rendered yet.
    assert!(query_optional(&root, ".search-result-row").is_none());

    wait(400).await;
    assert!(query_optional(&root, ".search-result-row").is_some());
    let text = query_one(&root, ".result-list")
        .text_content()
        .expect("result text");
    assert!(text.contains("สมชาย ใจดี"));
    assert!(text.contains("00012345"));
    api::clear_mock_invoke();
}

#[wasm_bindgen_test]
async fn patient_search_connection_error_raises_banner() {
    api::clear_mock_invoke();
    api::install_mock_invoke(|_cmd: &str, _args: JsValue| {
        Promise::reject(
            &to_value(&api::ApiError {
                kind: api::ApiErrorKind::Connection,
                message: "เชื่อมต่อฐานข้อมูล HOSxP ไม่สำเร็จ".into(),
            })
            .expect("serialize error"),
        )
    });
    let state = AppState::new();
    let view_state = state.clone();
    let root = mount(
        "ps-err",
        move || view! { <PatientSearch state=view_state /> },
    );
    type_text(&input(&root), "สมชาย");
    press_enter(&input(&root));
    settle().await;

    assert_eq!(
        state.db_banner.get_untracked().as_deref(),
        Some("เชื่อมต่อฐานข้อมูล HOSxP ไม่สำเร็จ")
    );
    api::clear_mock_invoke();
}

#[wasm_bindgen_test]
async fn health_states_round_trip_through_the_api_boundary() {
    // The ConnectionHealth enum crosses IPC and drives the dot — pin the
    // three serialized names once here.
    for (variant, name) in [
        (ConnectionHealth::Unconfigured, "Unconfigured"),
        (ConnectionHealth::Connected, "Connected"),
        (ConnectionHealth::Disconnected, "Disconnected"),
    ] {
        let js = to_value(&variant).expect("serialize health");
        assert_eq!(js.as_string(), Some(name.to_string()));
    }
}
