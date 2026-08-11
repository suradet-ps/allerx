//! Drug search — sidebar section (DESIGN.md: sidebar__section).
//! Autocomplete drug search with submit button, disabled until patient selected.

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use leptos::prelude::*;

use crate::api;
use crate::components::icons::{IconPill, IconSearch, IconX};
use crate::state::{AppState, VerdictState};

const DEBOUNCE_MS: u64 = 250;

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
            note.set(Some("เลือกผู้ป่วยก่อน".to_string()));
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

    let clear_input = move |_| {
        term.set(String::new());
        suggestions.set(Vec::new());
        selected_icode.set(None);
        note.set(None);
    };

    let is_disabled = move || state.patient.get().is_none();

    view! {
        <div class="sidebar__section">
            <div class="sidebar__label">
                <IconPill class="icon" />
                "ค้นหายา"
            </div>
            <div class="search-wrapper">
                <IconSearch class="search-icon" />
                <input
                    class="search-input"
                    placeholder="ชื่อยา (สามัญ / การค้า)"
                    prop:disabled=is_disabled
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
                {move || (!term.get().is_empty()).then(|| {
                    view! {
                        <button
                            class="search-clear"
                            on:click=clear_input
                            aria-label="ล้าง"
                        >
                            <IconX class="icon" />
                        </button>
                    }.into_any()
                })}
            </div>
            <div style="margin-top: var(--sp-sm);">
                <button
                    class="button-primary"
                    prop:disabled=is_disabled
                    on:click=move |_| run_check_click()
                >
                    "ตรวจประวัติ"
                </button>
            </div>
            {move || {
                note.get().map(|text| {
                    view! { <p class="sidebar__empty">{text}</p> }.into_any()
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
                                            let term_value = format!(
                                                "{}{}", drug.name, strength_suffix
                                            );
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
        </div>
    }
}
