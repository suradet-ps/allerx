//! AllerX frontend library (Leptos 0.8, CSR).
//!
//! Two-panel desktop layout: sidebar (input) + main canvas (output).
//! On launch the app checks for stored connection settings; if absent,
//! the settings dialog opens automatically. The top-bar status dot is
//! driven by polling the backend's live `connection_health` (Phase 3).
//!
//! The crate is split lib + bin so the components are wasm-testable
//! (ROADMAP Phase 4): `main.rs` is only `allerx_app::run()`.

pub mod api;
pub mod components;
pub mod state;

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use leptos::prelude::*;

use components::drug_search::DrugSearch;
use components::icons::{IconPrinter, IconX};
use components::patient_bar::PatientBar;
use components::patient_detail_modal::PatientDetailModal;
use components::patient_search::PatientSearch;
use components::print_sheet::PrintSheet;
use components::settings_modal::SettingsModal;
use components::timeline::Timeline;
use components::top_bar::TopBar;
use components::verdict_band::VerdictBand;
use state::{AppState, VerdictState};

/// How often the frontend polls the backend's live health state.
const HEALTH_POLL_INTERVAL: Duration = Duration::from_secs(30);

/// Self-scheduling poll loop cell: `None` until the trigger is installed.
type PollLoop = Rc<RefCell<Option<Rc<dyn Fn()>>>>;

/// Mounts the app into the document body.
pub fn run() {
    leptos::mount::mount_to_body(|| view! { <App /> });
}

/// Two-panel desktop layout (DESIGN.md "Structure").
#[component]
fn App() -> impl IntoView {
    let state = AppState::new();

    // First-run check: no stored settings → open the settings dialog.
    leptos::task::spawn_local(async move {
        let configured = api::connection_status().await;
        state.configured.set(configured);
        if !configured {
            state.settings_open.set(true);
        }
    });

    // Poll the backend's live reachability (ROADMAP Phase 3) — the status
    // dot must reflect a dead database within seconds, not "config exists".
    let poll_state = state.clone();
    let poll_health = Rc::new(move || {
        let poll_state = poll_state.clone();
        leptos::task::spawn_local(async move {
            if let Ok(health) = api::connection_health().await {
                poll_state.health.set(health);
            }
        });
    });
    // Self-scheduling loop: `poll_loop` holds the trigger, which schedules
    // itself again via the timer (one pending timer at a time, no leak).
    let poll_loop: PollLoop = Rc::new(RefCell::new(None));
    let trigger = {
        let poll_loop = Rc::clone(&poll_loop);
        Rc::new(move || {
            poll_health();
            let next = Rc::clone(
                poll_loop
                    .borrow()
                    .as_ref()
                    .expect("invariant: trigger is set before first poll"),
            );
            let _ = set_timeout_with_handle(move || next(), HEALTH_POLL_INTERVAL);
        })
    };
    *poll_loop.borrow_mut() = Some(Rc::clone(&trigger) as Rc<dyn Fn()>);
    trigger();

    view! {
        <div class="app">
            <TopBar state=state.clone() />
            <div class="app__body">
                <aside class="sidebar">
                    <PatientSearch state=state.clone() />
                    <PatientBar state=state.clone() />
                    <DrugSearch state=state.clone() />
                </aside>
                <main class="main-canvas">
                    {move || {
                        state.db_banner.get().map(|message| {
                            view! {
                                <div class="banner-warning">
                                    <span>{message}</span>
                                    <button
                                        class="banner-warning__close"
                                        on:click=move |_| state.db_banner.set(None)
                                        aria-label="ปิด"
                                    >
                                        <IconX class="icon" />
                                    </button>
                                </div>
                            }
                        })
                    }}
                    {move || {
                        let has_results = matches!(state.verdict.get(), VerdictState::Results { .. });
                        let has_patient = state.patient.get().is_some();
                        (has_results && has_patient).then(|| {
                            view! {
                                <div class="print-toolbar">
                                    <button
                                        class="button-secondary button-secondary--inline"
                                        on:click=move |_| {
                                            if let Some(window) = web_sys::window() {
                                                let _ = window.print();
                                            }
                                        }
                                    >
                                        <IconPrinter class="icon" />
                                        "พิมพ์ประวัติ"
                                    </button>
                                </div>
                            }
                                .into_any()
                        })
                    }}
                    <VerdictBand state=state.clone() />
                    <Timeline state=state.clone() />
                </main>
            </div>
            <SettingsModal state=state.clone() />
            <PatientDetailModal state=state.clone() />
            <PrintSheet state=state.clone() />
        </div>
    }
}
