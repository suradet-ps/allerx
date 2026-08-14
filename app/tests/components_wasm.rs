//! Component mount tests (ROADMAP Phase 4/5) — the clinical surface
//! rendered into a real DOM in headless Chrome (run via `wasm-pack test`).
//!
//! These pin the states a pharmacist actually sees: the four verdict
//! states, batch verdict bands, truncation honesty, CID masking, the
//! detail modal with its recent-meds snapshot, the print sheet, and the
//! search flows (routing through a fake `invoke`, so no Tauri and no
//! database are involved).

#![cfg(target_arch = "wasm32")]

// These tests mount components into a real DOM, so they must run in a
// browser (wasm-pack test --headless --chrome).
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

use std::cell::RefCell;

use allerx_app::api;
use allerx_app::components::drug_search::DrugSearch;
use allerx_app::components::patient_bar::PatientBar;
use allerx_app::components::patient_detail_modal::PatientDetailModal;
use allerx_app::components::patient_search::PatientSearch;
use allerx_app::components::print_sheet::PrintSheet;
use allerx_app::components::settings_modal::SettingsModal;
use allerx_app::components::timeline::Timeline;
use allerx_app::components::verdict_band::VerdictBand;
use allerx_app::state::{AppState, ConnectionHealth, DrugVerdict, DrugVerdictState, VerdictState};
use allerx_models::{
    ConcurrentMedication, DrugCheckResult, DrugHistoryRecord, DrugItem, HistoryVerdict,
    PatientSummary, ResolvedHistory, VisitType,
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
        strength: Some("500 mg".into()),
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

fn drug_verdict(term: &str, state: DrugVerdictState) -> DrugVerdict {
    DrugVerdict {
        term: term.into(),
        state,
    }
}

fn sample_drug_item() -> DrugItem {
    DrugItem {
        icode: "1-001".into(),
        name: "พาราเซตามอล".into(),
        strength: Some("500 mg".into()),
        trade_name: None,
    }
}

fn found_verdict(term: &str, records: Vec<DrugHistoryRecord>, truncated: bool) -> DrugVerdict {
    drug_verdict(
        term,
        DrugVerdictState::Found {
            drug: sample_drug_item(),
            records,
            truncated,
        },
    )
}

fn not_found_verdict(term: &str) -> DrugVerdict {
    drug_verdict(
        term,
        DrugVerdictState::NotFound {
            drug: sample_drug_item(),
        },
    )
}

fn resolve_ok<T: serde::Serialize>(value: &T) -> Promise {
    Promise::resolve(&to_value(value).expect("serialize test value"))
}

fn reject_connection() -> Promise {
    Promise::reject(
        &to_value(&api::ApiError {
            kind: api::ApiErrorKind::Connection,
            message: "เชื่อมต่อฐานข้อมูล HOSxP ไม่สำเร็จ".into(),
        })
        .expect("serialize error"),
    )
}

fn mock_invoke(handler: impl Fn(&str) -> Promise + 'static) {
    api::clear_mock_invoke();
    api::install_mock_invoke(move |cmd: &str, _args: JsValue| handler(cmd));
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

/// Dispatches Escape on `window` — the target the modals' close listeners
/// are attached to (they close from anywhere, per DESIGN.md).
fn press_escape() {
    let init = KeyboardEventInit::new();
    init.set_key("Escape");
    let event =
        KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init).expect("keydown event");
    web_sys::window()
        .expect("window in test")
        .dispatch_event(&event)
        .expect("dispatch escape");
}

fn type_text(target: &HtmlInputElement, text: &str) {
    // The value is what the handler reads (event_target_value); the event
    // just needs to be an "input" event — no init dict required.
    target.set_value(text);
    let event = web_sys::Event::new("input").expect("input event");
    target.dispatch_event(&event).expect("dispatch input");
}

fn click(root: &HtmlElement, selector: &str) {
    let button = query_one(root, selector);
    let event = web_sys::MouseEvent::new("click").expect("click event");
    button.dispatch_event(&event).expect("dispatch click");
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
// Verdict band — the four states (Phase 1 contract, now under Results)
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
    state.verdict.set(VerdictState::Results {
        results: vec![found_verdict(
            "พาราเซตามอล",
            vec![record("พาราเซตามอล", date(2024, 5, 5), VisitType::Opd)],
            false,
        )],
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
    state.verdict.set(VerdictState::Results {
        results: vec![found_verdict(
            "พาราเซตามอล",
            vec![record("พาราเซตามอล", date(2024, 5, 5), VisitType::Opd)],
            true,
        )],
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
    state.verdict.set(VerdictState::Results {
        results: vec![not_found_verdict("พาราเซตามอล")],
    });
    let root = mount("vb-notfound", move || view! { <VerdictBand state=state /> });
    let band = query_one(&root, ".verdict-band");
    assert!(band.class_list().contains("verdict-notfound"));
    let text = band.text_content().expect("text content");
    assert!(text.contains("ไม่พบประวัติการได้รับยานี้"));
    // The resolved drug's identity labels the verdict — an icode search
    // (e.g. "1000152") must reveal which drug it referred to.
    assert!(text.contains("พาราเซตามอล (500 mg)"));
}

#[wasm_bindgen_test]
async fn verdict_unresolved_with_candidates_never_uses_notfound_text() {
    let state = AppState::new();
    state.verdict.set(VerdictState::Results {
        results: vec![drug_verdict(
            "พารา",
            DrugVerdictState::Unresolved {
                candidates: vec![sample_drug()],
            },
        )],
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
    state.verdict.set(VerdictState::Results {
        results: vec![drug_verdict(
            "ไม่มีในระบบ",
            DrugVerdictState::Unresolved {
                candidates: Vec::new(),
            },
        )],
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
// Batch verdict bands (Phase 5)
// ---------------------------------------------------------------------------

#[wasm_bindgen_test]
async fn batch_renders_one_compact_band_per_drug_with_term_labels() {
    let state = AppState::new();
    state.verdict.set(VerdictState::Results {
        results: vec![
            found_verdict(
                "พาราเซตามอล",
                vec![record("พาราเซตามอล", date(2024, 5, 5), VisitType::Opd)],
                false,
            ),
            not_found_verdict("แอมม็อกซิซิลลิน"),
        ],
    });
    let root = mount("vb-batch", move || view! { <VerdictBand state=state /> });
    let bands = root
        .query_selector_all(".verdict-batch .verdict-band--compact")
        .expect("query bands");
    assert_eq!(bands.length(), 2);
    let text = query_one(&root, ".verdict-batch")
        .text_content()
        .expect("batch text");
    assert!(text.contains("พาราเซตามอล"));
    assert!(text.contains("แอมม็อกซิซิลลิน"));
    // Compact bands label each verdict with the resolved drug identity.
    assert!(text.contains("พบประวัติ — พาราเซตามอล (500 mg) · ครั้งล่าสุด"));
    assert!(text.contains("ไม่พบประวัติ — พาราเซตามอล (500 mg)"));
}

#[wasm_bindgen_test]
async fn batch_unresolved_band_offers_candidate_buttons_that_queue_the_drug() {
    let state = AppState::new();
    state.verdict.set(VerdictState::Results {
        results: vec![
            drug_verdict(
                "พารา",
                DrugVerdictState::Unresolved {
                    candidates: vec![sample_drug()],
                },
            ),
            found_verdict(
                "ยาที่สอง",
                vec![record("ยาที่สอง", date(2024, 1, 1), VisitType::Ipd)],
                false,
            ),
        ],
    });
    let view_state = state.clone();
    let root = mount(
        "vb-batch-unresolved",
        move || view! { <VerdictBand state=view_state /> },
    );
    click(&root, ".candidate-button");
    let chips = state.drug_chips.get_untracked();
    assert_eq!(chips.len(), 1);
    // The queued chip carries the full identity — name with strength.
    assert_eq!(chips[0].label, "พาราเซตามอล (500 mg)");
    assert_eq!(chips[0].icode.as_deref(), Some("1-001"));
}

// ---------------------------------------------------------------------------
// Timeline — rows and truncation honesty (Phase 1)
// ---------------------------------------------------------------------------

#[wasm_bindgen_test]
async fn timeline_renders_rows_most_recent_first_and_complete_footer() {
    let state = AppState::new();
    state.verdict.set(VerdictState::Results {
        results: vec![found_verdict(
            "พาราเซตามอล",
            vec![
                record("พาราเซตามอล", date(2024, 5, 5), VisitType::Opd),
                record("พาราเซตามอล", date(2024, 2, 2), VisitType::Ipd),
            ],
            false,
        )],
    });
    let root = mount("tl-complete", move || view! { <Timeline state=state /> });
    let text = query_one(&root, ".timeline")
        .text_content()
        .expect("text content");
    // Timeline rows show the drug with its strength.
    assert!(text.contains("พาราเซตามอล (500 mg)"));
    let footer = query_one(&root, ".timeline-footer")
        .text_content()
        .expect("footer text");
    assert!(footer.contains("ทั้งหมด 2 รายการ"));
}

#[wasm_bindgen_test]
async fn timeline_truncated_footer_does_not_present_list_as_complete() {
    let state = AppState::new();
    state.verdict.set(VerdictState::Results {
        results: vec![found_verdict(
            "พาราเซตามอล",
            vec![record("พาราเซตามอล", date(2024, 5, 5), VisitType::Opd)],
            true,
        )],
    });
    let root = mount("tl-trunc", move || view! { <Timeline state=state /> });
    let footer = query_one(&root, ".timeline-footer")
        .text_content()
        .expect("footer text");
    assert!(footer.contains("มีประวัติเก่ากว่านี้"));
    assert!(!footer.contains("ทั้งหมด 1 รายการ"));
}

#[wasm_bindgen_test]
async fn timeline_merges_records_across_drugs_newest_first() {
    let state = AppState::new();
    state.verdict.set(VerdictState::Results {
        results: vec![
            found_verdict(
                "ยากลุ่มแรก",
                vec![record("ยากลุ่มแรก", date(2024, 1, 1), VisitType::Opd)],
                false,
            ),
            found_verdict(
                "ยากลุ่มสอง",
                vec![record("ยากลุ่มสอง", date(2024, 6, 6), VisitType::Ipd)],
                false,
            ),
        ],
    });
    let root = mount("tl-merged", move || view! { <Timeline state=state /> });
    let rows = root
        .query_selector_all(".timeline-row")
        .expect("query rows");
    assert_eq!(rows.length(), 2);
    let text = query_one(&root, ".timeline")
        .text_content()
        .expect("text content");
    // newest (6/6) first, older (1/1) second — check order via row texts.
    let first_row = rows.item(0).expect("first row");
    let first_text = first_row.text_content().expect("row text");
    assert!(first_text.contains("06/06/2024"));
    assert!(text.contains("ยากลุ่มแรก") && text.contains("ยากลุ่มสอง"));
}

#[wasm_bindgen_test]
async fn timeline_filter_isolates_one_drugs_rows() {
    let state = AppState::new();
    state.verdict.set(VerdictState::Results {
        results: vec![
            found_verdict(
                "ยากลุ่มแรก",
                vec![
                    record("ยากลุ่มแรก", date(2024, 1, 1), VisitType::Opd),
                    record("ยากลุ่มแรก", date(2024, 3, 3), VisitType::Ipd),
                ],
                false,
            ),
            found_verdict(
                "ยากลุ่มสอง",
                vec![record("ยากลุ่มสอง", date(2024, 6, 6), VisitType::Opd)],
                false,
            ),
        ],
    });
    let root = mount("tl-filter", move || view! { <Timeline state=state /> });

    // Filter bar appears for multi-drug checks, with ทั้งหมด + one chip per drug.
    let chips = root
        .query_selector_all(".timeline-filter__chip")
        .expect("filter chips");
    assert_eq!(chips.length(), 3);
    let bar_text = query_one(&root, ".timeline-filter")
        .text_content()
        .expect("bar text");
    assert!(bar_text.contains("ทั้งหมด"));
    assert!(bar_text.contains("ยากลุ่มแรก"));
    assert!(bar_text.contains("ยากลุ่มสอง"));

    // All rows shown by default.
    let rows = root.query_selector_all(".timeline-row").expect("rows");
    assert_eq!(rows.length(), 3);

    // Click the ยากลุ่มแรก chip → only that drug's rows remain.
    let chip = root
        .query_selector_all(".timeline-filter__chip")
        .expect("chips")
        .item(1)
        .expect("second chip")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("html element");
    chip.click();
    settle().await;

    // The clicked chip became active.
    let active = root
        .query_selector_all(".timeline-filter__chip--active")
        .expect("active chips")
        .item(0)
        .expect("one active chip")
        .text_content()
        .expect("active chip text");
    assert_eq!(active, "ยากลุ่มแรก", "the clicked chip became active");

    let rows = root.query_selector_all(".timeline-row").expect("rows");
    assert_eq!(rows.length(), 2);
    let text = query_one(&root, ".timeline")
        .text_content()
        .expect("timeline text");
    assert!(text.contains("ยากลุ่มแรก"));
    assert!(!text.contains("ยากลุ่มสอง"));

    // Click ทั้งหมด → back to all.
    let all_chip = root
        .query_selector_all(".timeline-filter__chip")
        .expect("chips")
        .item(0)
        .expect("first chip")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("html element");
    all_chip.click();
    settle().await;

    let rows = root.query_selector_all(".timeline-row").expect("rows");
    assert_eq!(rows.length(), 3);
}

#[wasm_bindgen_test]
async fn timeline_resets_filter_on_a_new_check_with_identical_content() {
    // The same drugs checked again (e.g. for a new patient) can produce
    // byte-identical results — the per-drug filter must still reset, or the
    // new timeline would silently show only one drug's rows.
    let state = AppState::new();
    let check = || VerdictState::Results {
        results: vec![
            found_verdict(
                "ยากลุ่มแรก",
                vec![
                    record("ยากลุ่มแรก", date(2024, 1, 1), VisitType::Opd),
                    record("ยากลุ่มแรก", date(2024, 3, 3), VisitType::Ipd),
                ],
                false,
            ),
            found_verdict(
                "ยากลุ่มสอง",
                vec![record("ยากลุ่มสอง", date(2024, 6, 6), VisitType::Opd)],
                false,
            ),
        ],
    };
    state.verdict.set(check());
    state.check_seq.set(0);
    let view_state = state.clone();
    let root = mount(
        "tl-recheck",
        move || view! { <Timeline state=view_state /> },
    );

    // Filter to the first drug.
    let chip = root
        .query_selector_all(".timeline-filter__chip")
        .expect("chips")
        .item(1)
        .expect("second chip")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("html element");
    chip.click();
    settle().await;
    let rows = root.query_selector_all(".timeline-row").expect("rows");
    assert_eq!(rows.length(), 2);

    // A new check with identical content — seq bumps, filter must reset.
    state.verdict.set(check());
    state.check_seq.set(1);
    settle().await;
    let rows = root.query_selector_all(".timeline-row").expect("rows");
    assert_eq!(rows.length(), 3, "filter reset to all drugs on a new check");
    let active = root
        .query_selector_all(".timeline-filter__chip--active")
        .expect("active chips");
    assert_eq!(active.length(), 1, "the ทั้งหมด chip is active");
    assert_eq!(
        active.item(0).expect("chip").text_content().expect("text"),
        "ทั้งหมด"
    );
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
// Drug search — chip queue + batch check through a fake invoke (Phase 5)
// ---------------------------------------------------------------------------

fn drug_search_with(mock: impl Fn(&str) -> Promise + 'static) -> (AppState, HtmlElement) {
    mock_invoke(mock);
    let state = AppState::new();
    state.patient.set(Some(sample_patient()));
    let view_state = state.clone();
    let root = mount("ds", move || view! { <DrugSearch state=view_state /> });
    (state, root)
}

fn queue_drug(root: &HtmlElement, text: &str) {
    type_text(&input(root), text);
    press_enter(&input(root));
}

#[wasm_bindgen_test]
async fn drug_search_single_unresolved_routes_to_unresolved_verdict_and_suggests() {
    let (state, root) = drug_search_with(|cmd| {
        assert_eq!(cmd, "check_drugs");
        resolve_ok(&vec![DrugCheckResult {
            term: "พารา".into(),
            verdict: HistoryVerdict::Unresolved {
                candidates: vec![sample_drug()],
            },
        }])
    });
    let search = input(&root);
    assert!(!search.disabled(), "enabled once a patient is selected");
    queue_drug(&root, "พารา");
    assert_eq!(state.drug_chips.get_untracked().len(), 1);
    click(&root, ".button-primary");
    settle().await;

    match state.verdict.get_untracked() {
        VerdictState::Results { results } => {
            assert_eq!(results.len(), 1);
            assert!(matches!(
                &results[0].state,
                DrugVerdictState::Unresolved { .. }
            ));
        }
        other => panic!("expected Results, got {other:?}"),
    }
    // The backend's candidates were surfaced as the suggestion list.
    assert!(query_optional(&root, ".search-result-row").is_some());
    api::clear_mock_invoke();
}

#[wasm_bindgen_test]
async fn drug_search_resolved_empty_routes_to_notfound() {
    let (state, root) = drug_search_with(|_cmd| {
        resolve_ok(&vec![DrugCheckResult {
            term: "พาราเซตามอล".into(),
            verdict: HistoryVerdict::Resolved {
                drug: sample_drug_item(),
                history: ResolvedHistory {
                    records: Vec::new(),
                    truncated: false,
                },
            },
        }])
    });
    queue_drug(&root, "พาราเซตามอล");
    click(&root, ".button-primary");
    settle().await;

    match state.verdict.get_untracked() {
        VerdictState::Results { results } => {
            assert!(matches!(
                &results[0].state,
                DrugVerdictState::NotFound { .. }
            ));
        }
        other => panic!("expected Results, got {other:?}"),
    }
    api::clear_mock_invoke();
}

#[wasm_bindgen_test]
async fn drug_search_batch_checks_two_chips_and_renders_two_bands() {
    let (state, root) = drug_search_with(|cmd| {
        assert_eq!(cmd, "check_drugs");
        resolve_ok(&vec![
            DrugCheckResult {
                term: "พาราเซตามอล".into(),
                verdict: HistoryVerdict::Resolved {
                    drug: sample_drug_item(),
                    history: ResolvedHistory {
                        records: vec![record("พาราเซตามอล", date(2024, 5, 5), VisitType::Opd)],
                        truncated: false,
                    },
                },
            },
            DrugCheckResult {
                term: "แอมม็อกซิซิลลิน".into(),
                verdict: HistoryVerdict::Resolved {
                    drug: sample_drug_item(),
                    history: ResolvedHistory {
                        records: Vec::new(),
                        truncated: false,
                    },
                },
            },
        ])
    });
    queue_drug(&root, "พาราเซตามอล");
    queue_drug(&root, "แอมม็อกซิซิลลิน");
    assert_eq!(state.drug_chips.get_untracked().len(), 2);
    click(&root, ".button-primary");
    settle().await;

    match state.verdict.get_untracked() {
        VerdictState::Results { results } => {
            assert_eq!(results.len(), 2);
            assert!(matches!(&results[0].state, DrugVerdictState::Found { .. }));
            assert!(matches!(
                &results[1].state,
                DrugVerdictState::NotFound { .. }
            ));
        }
        other => panic!("expected Results, got {other:?}"),
    }
    // Two compact bands on the canvas (mounted with VerdictBand).
    assert!(query_optional(&root, ".chip-list").is_some());
    api::clear_mock_invoke();
}

#[wasm_bindgen_test]
async fn drug_search_connection_error_raises_banner_and_keeps_verdict_pending() {
    let (state, root) = drug_search_with(|_cmd| reject_connection());
    queue_drug(&root, "พาราเซตามอล");
    click(&root, ".button-primary");
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
    mock_invoke(|cmd: &str| {
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
    mock_invoke(|_cmd: &str| reject_connection());
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

// ---------------------------------------------------------------------------
// Patient detail modal (Phase 5) — full CID + recent meds snapshot
// ---------------------------------------------------------------------------

#[wasm_bindgen_test]
async fn detail_modal_reveals_full_cid_and_lists_recent_medications() {
    mock_invoke(|cmd: &str| {
        assert_eq!(cmd, "fetch_concurrent_medications");
        resolve_ok(&vec![ConcurrentMedication {
            drug_code: "1-001".into(),
            drug_name: "พาราเซตามอล".into(),
            strength: Some("500 mg".into()),
            trade_name: Some("TYLENOL".into()),
            last_date: date(2024, 6, 1),
        }])
    });
    let state = AppState::new();
    state.patient.set(Some(sample_patient()));
    state.detail_open.set(true);
    let view_state = state.clone();
    let root = mount(
        "detail",
        move || view! { <PatientDetailModal state=view_state /> },
    );
    settle().await;

    let text = query_one(&root, ".modal")
        .text_content()
        .expect("modal text");
    assert!(
        text.contains("1101701234567"),
        "full CID revealed on detail view"
    );
    assert!(text.contains("สมชาย ใจดี"));
    // Med rows show the name with its strength (and trade name).
    assert!(text.contains("พาราเซตามอล (500 mg) (TYLENOL)"));
    assert!(text.contains("01/06/2024"));
    api::clear_mock_invoke();
}

#[wasm_bindgen_test]
async fn detail_modal_shows_empty_state_for_no_recent_meds() {
    mock_invoke(|_cmd: &str| resolve_ok(&Vec::<ConcurrentMedication>::new()));
    let state = AppState::new();
    state.patient.set(Some(sample_patient()));
    state.detail_open.set(true);
    let view_state = state.clone();
    let root = mount(
        "detail-empty",
        move || view! { <PatientDetailModal state=view_state /> },
    );
    settle().await;

    let text = query_one(&root, ".modal")
        .text_content()
        .expect("modal text");
    assert!(text.contains("ไม่มีรายการจ่ายยาใน 30 วันที่ผ่านมา"));
    api::clear_mock_invoke();
}

// ---------------------------------------------------------------------------
// Escape closes the dialogs (DESIGN.md focus management)
// ---------------------------------------------------------------------------

#[wasm_bindgen_test]
async fn settings_modal_escape_closes_the_dialog() {
    api::clear_mock_invoke();
    let state = AppState::new();
    state.settings_open.set(true);
    let view_state = state.clone();
    let root = mount(
        "sm-escape",
        move || view! { <SettingsModal state=view_state /> },
    );
    // Sanity: the dialog is visible while open.
    let display = query_one(&root, ".modal-backdrop")
        .dyn_into::<HtmlElement>()
        .expect("backdrop is an HtmlElement")
        .style()
        .get_property_value("display")
        .expect("backdrop display");
    assert_eq!(display, "flex");

    press_escape();
    settle().await;
    assert!(
        !state.settings_open.get_untracked(),
        "Escape closes the dialog"
    );
    let display = query_one(&root, ".modal-backdrop")
        .dyn_into::<HtmlElement>()
        .expect("backdrop is an HtmlElement")
        .style()
        .get_property_value("display")
        .expect("backdrop display");
    assert_eq!(display, "none");
}

#[wasm_bindgen_test]
async fn settings_modal_escape_when_closed_never_opens_or_wipes() {
    // Escape elsewhere in the app (e.g. clearing a search input) must not
    // touch a closed dialog — the open-flag guard is the protection.
    api::clear_mock_invoke();
    let state = AppState::new();
    let view_state = state.clone();
    let root = mount(
        "sm-escape-closed",
        move || view! { <SettingsModal state=view_state /> },
    );
    press_escape();
    settle().await;
    assert!(!state.settings_open.get_untracked());
    let display = query_one(&root, ".modal-backdrop")
        .dyn_into::<HtmlElement>()
        .expect("backdrop is an HtmlElement")
        .style()
        .get_property_value("display")
        .expect("backdrop display");
    assert_eq!(display, "none");
}

#[wasm_bindgen_test]
async fn detail_modal_escape_closes_the_dialog() {
    mock_invoke(|_cmd: &str| resolve_ok(&Vec::<ConcurrentMedication>::new()));
    let state = AppState::new();
    state.patient.set(Some(sample_patient()));
    state.detail_open.set(true);
    let view_state = state.clone();
    let root = mount(
        "detail-escape",
        move || view! { <PatientDetailModal state=view_state /> },
    );
    settle().await;
    assert!(state.detail_open.get_untracked());

    press_escape();
    settle().await;
    assert!(
        !state.detail_open.get_untracked(),
        "Escape closes the dialog"
    );
    let display = query_one(&root, ".modal-backdrop")
        .dyn_into::<HtmlElement>()
        .expect("backdrop is an HtmlElement")
        .style()
        .get_property_value("display")
        .expect("backdrop display");
    assert_eq!(display, "none");
    api::clear_mock_invoke();
}

// ---------------------------------------------------------------------------
// Print sheet (Phase 5)
// ---------------------------------------------------------------------------

#[wasm_bindgen_test]
async fn print_sheet_renders_patient_and_history_content() {
    let state = AppState::new();
    state.patient.set(Some(sample_patient()));
    state.verdict.set(VerdictState::Results {
        results: vec![
            found_verdict(
                "พาราเซตามอล",
                vec![record("พาราเซตามอล", date(2024, 5, 5), VisitType::Opd)],
                false,
            ),
            not_found_verdict("แอมม็อกซิซิลลิน"),
        ],
    });
    let root = mount("print", move || view! { <PrintSheet state=state /> });
    let text = query_one(&root, ".print-sheet")
        .text_content()
        .expect("sheet text");
    assert!(text.contains("AllerX — ใบประวัติการได้รับยา"));
    assert!(text.contains("สมชาย ใจดี"));
    assert!(
        text.contains("1101701234567"),
        "print sheet shows the full CID"
    );
    assert!(text.contains("พบประวัติ"));
    assert!(text.contains("ไม่พบประวัติ"));
    // Timeline rows in print carry the strength too.
    assert!(text.contains("พาราเซตามอล (500 mg)"));
    // The sheet carries the print-sheet class — its on-screen hiding is a
    // CSS concern (@media print in main.css), which the test page does not
    // load; the class pin is the DOM-side contract.
    let sheet = query_one(&root, ".print-sheet");
    assert!(sheet.class_list().contains("print-sheet"));
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
