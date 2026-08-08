//! AllerX frontend (Leptos 0.8, CSR).
//!
//! Single-page flow per DESIGN.md. On launch the app asks the backend
//! whether encrypted connection settings exist; if not, the settings dialog
//! opens automatically so the first run is configurable without hunting.

mod api;
mod components;
mod state;

use leptos::prelude::*;

use components::drug_search::DrugSearch;
use components::patient_bar::PatientBar;
use components::patient_search::PatientSearch;
use components::settings_modal::SettingsModal;
use components::timeline::Timeline;
use components::top_bar::TopBar;
use components::verdict_band::VerdictBand;
use state::AppState;

fn main() {
    leptos::mount::mount_to_body(|| view! { <App /> });
}

/// Single-page layout (DESIGN.md "Structure", top to bottom).
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

    view! {
        <div class="app">
            <TopBar state=state.clone() />
            <main class="app__content">
                <div class="bento__cell bento__patient-search">
                    <PatientSearch state=state.clone() />
                    <PatientBar state=state.clone() />
                </div>
                <div class="bento__cell bento__drug-search">
                    <DrugSearch state=state.clone() />
                </div>
                <div class="bento__cell bento__verdict">
                    <VerdictBand state=state.clone() />
                </div>
                <div class="bento__cell bento__timeline">
                    <Timeline state=state.clone() />
                </div>
            </main>
            <SettingsModal state=state.clone() />
        </div>
    }
}
