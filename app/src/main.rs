//! AllerX frontend (Leptos 0.8, CSR).
//!
//! Two-panel desktop layout: sidebar (input) + main canvas (output).
//! On launch the app checks for stored connection settings; if absent,
//! the settings dialog opens automatically.

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
                    <VerdictBand state=state.clone() />
                    <Timeline state=state.clone() />
                </main>
            </div>
            <SettingsModal state=state.clone() />
        </div>
    }
}
