//! Drug search + autocomplete (DESIGN.md: search-input, AGENTS.md §7.2).
//!
//! M3/M4: suggestions come from the backend `drugitems` table (250 ms
//! debounce); submitting runs the full history lookup and drives the
//! verdict band. Requires a selected patient first.

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use leptos::prelude::*;

use crate::api;
use crate::components::icons::{IconClock, IconPill};
use crate::state::{AppState, VerdictState};

/// Autocomplete debounce (AGENTS.md §7.1/§7.2).
const DEBOUNCE_MS: u64 = 250;

/// Drug-name search box; the verdict band sits directly below this.
#[component]
pub fn DrugSearch(state: AppState) -> impl IntoView {
    let term = RwSignal::new(String::new());
    let suggestions = RwSignal::new(Vec::<allerx_models::DrugItem>::new());
    let selected_icode = RwSignal::new(Option::<String>::None);
    let note = RwSignal::new(Option::<String>::None);

    let run_check = Rc::new(move || {
        let query = term.get_untracked();
        if query.trim().is_empty() {
            note.set(None);
            return;
        }
        let Some(patient) = state.patient.get_untracked() else {
            note.set(Some("กรุณาเลือกผู้ป่วยก่อนค้นหาประวัติ".to_string()));
            return;
        };
        note.set(None);
        let icode = selected_icode.get_untracked();
        let drug = icode.unwrap_or(query.trim().to_string());
        leptos::task::spawn_local(async move {
            match api::fetch_history(&patient.hn, &drug).await {
                Ok(records) if records.is_empty() => {
                    state.verdict.set(VerdictState::NotFound);
                }
                Ok(records) => {
                    state.verdict.set(VerdictState::Found { records });
                }
                Err(message) => {
                    state.verdict.set(VerdictState::Pending);
                    note.set(Some(message));
                }
            }
        });
    });
    let run_check_enter = Rc::clone(&run_check);
    let run_check_click = Rc::clone(&run_check);
    let debounce = Rc::new(Cell::new(None::<TimeoutHandle>));

    view! {
        <section class="panel">
            <label class="panel__label" for="drug-search">
                <IconPill class="icon" />
                "ค้นหายาที่ต้องการตรวจประวัติ"
            </label>
            <div class="search-row">
                <input
                    id="drug-search"
                    class="search-input"
                    placeholder="ชื่อยา (สามัญ / การค้า)"
                    prop:value=move || term.get()
                    on:input=move |ev| {
                        let value = event_target_value(&ev);
                        term.set(value.clone());
                        selected_icode.set(None);
                        note.set(None);
                        if value.trim().is_empty() {
                            suggestions.set(Vec::new());
                            return;
                        }
                        if let Some(handle) = debounce.get() {
                            handle.clear();
                        }
                        let schedule = Rc::new(move || {
                            let prefix = value.clone();
                            leptos::task::spawn_local(async move {
                                match api::search_drugs(&prefix).await {
                                    Ok(list) => suggestions.set(list),
                                    Err(message) => {
                                        suggestions.set(Vec::new());
                                        note.set(Some(message));
                                    }
                                }
                            });
                        });
                        if let Ok(handle) = set_timeout_with_handle(
                            move || schedule(),
                            Duration::from_millis(DEBOUNCE_MS),
                        ) {
                            debounce.set(Some(handle));
                        }
                    }
                    on:keydown=move |ev| {
                        if ev.key() == "Enter" {
                            run_check_enter();
                        }
                    }
                />
                <button class="button-primary" on:click=move |_| run_check_click()>
                    <IconClock class="icon" />
                    "ตรวจประวัติ"
                </button>
            </div>
            {move || {
                note.get().map(|text| {
                    view! { <p class="placeholder-note">{text}</p> }.into_any()
                })
            }}
            {move || {
                if suggestions.get().is_empty() {
                    None
                } else {
                    Some(
                        view! {
                        <ul class="result-list">
                            {move || {
                                suggestions
                                    .get()
                                    .into_iter()
                                    .map(|drug| {
                                        let strength_suffix = drug
                                            .strength
                                            .as_deref()
                                            .map(|s| format!(" ({s})"))
                                            .unwrap_or_default();
                                        let term_value = format!("{}{}", drug.name, strength_suffix);
                                        view! {
                                            <li
                                                class="search-result-row"
                                                on:click=move |_| {
                                                    term.set(term_value.clone());
                                                    selected_icode.set(Some(drug.icode.clone()));
                                                    suggestions.set(Vec::new());
                                                    note.set(None);
                                                }
                                            >
                                                <span class="search-result-row__name">
                                                    {drug.name.clone()}
                                                    <span class="search-result-row__strength">
                                                        {strength_suffix}
                                                    </span>
                                                </span>
                                                <span class="search-result-row__code">
                                                    {drug.icode.clone()}
                                                </span>
                                            </li>
                                        }
                                    })
                                    .collect_view()
                            }}
                        </ul>
                        }
                        .into_any(),
                    )
                }
            }}
        </section>
    }
}
