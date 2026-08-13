//! Patient search — sidebar section (DESIGN.md: sidebar__section).
//! Compact search box with autocomplete, adapted for sidebar layout.

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use leptos::prelude::*;

use crate::api;
use crate::components::icons::{IconSearch, IconUser, IconX};
use crate::state::AppState;

const DEBOUNCE_MS: u64 = 250;

#[component]
pub fn PatientSearch(state: AppState) -> impl IntoView {
    let term = RwSignal::new(String::new());
    let results = RwSignal::new(Vec::<allerx_models::PatientSummary>::new());
    let searched = RwSignal::new(false);
    let error = RwSignal::new(Option::<String>::None);

    let run_search = Rc::new(move || {
        let query = term.get_untracked();
        if query.trim().is_empty() {
            results.set(Vec::new());
            searched.set(false);
            error.set(None);
            return;
        }
        leptos::task::spawn_local(async move {
            searched.set(true);
            match api::search_patients(&query).await {
                Ok(list) => {
                    error.set(None);
                    results.set(list);
                }
                Err(message) => {
                    error.set(Some(message));
                    results.set(Vec::new());
                }
            }
        });
    });
    let run_search_enter = Rc::clone(&run_search);
    let debounce = Rc::new(Cell::new(None::<TimeoutHandle>));

    let clear_input = move |_| {
        term.set(String::new());
        results.set(Vec::new());
        searched.set(false);
        error.set(None);
    };

    view! {
        <div class="sidebar__section">
            <div class="sidebar__label">
                <IconUser class="icon" />
                "ค้นหาผู้ป่วย"
            </div>
            <div class="search-wrapper">
                <IconSearch class="search-icon" />
                <input
                    class="search-input"
                    placeholder="HN / CID / ชื่อ"
                    prop:value=move || term.get()
                    on:input=move |ev| {
                        let value = event_target_value(&ev);
                        term.set(value);
                        if let Some(handle) = debounce.get() {
                            handle.clear();
                        }
                        let schedule = Rc::clone(&run_search);
                        if let Ok(handle) = set_timeout_with_handle(
                            move || schedule(),
                            Duration::from_millis(DEBOUNCE_MS),
                        ) {
                            debounce.set(Some(handle));
                        }
                    }
                    on:keydown=move |ev| {
                        if ev.key() == "Enter" {
                            run_search_enter();
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
            {move || {
                if let Some(message) = error.get() {
                    view! { <p class="sidebar__empty">{message}</p> }.into_any()
                } else if searched.get() && results.get().is_empty() {
                    view! { <p class="sidebar__empty">"ไม่พบผู้ป่วย"</p> }.into_any()
                } else if !results.get().is_empty() {
                    view! {
                        <ul class="result-list">
                            {move || {
                                results
                                    .get()
                                    .into_iter()
                                    .map(|p| {
                                        let patient = p.clone();
                                        view! {
                                            <li
                                                class="search-result-row"
                                                on:click=move |_| {
                                                    state.patient.set(Some(patient.clone()));
                                                    term.set(String::new());
                                                    results.set(Vec::new());
                                                    searched.set(false);
                                                }
                                            >
                                                <span class="search-result-row__name">
                                                    {p.full_name_th.clone()}
                                                </span>
                                                <span class="search-result-row__code">
                                                    {format!(
                                                        "HN {} · {}",
                                                        p.hn,
                                                        p.birth_date
                                                            .map(|d| d.format("%d/%m/%Y").to_string())
                                                            .unwrap_or_else(|| "—".to_string())
                                                    )}
                                                </span>
                                            </li>
                                        }
                                    })
                                    .collect_view()
                            }}
                        </ul>
                    }
                        .into_any()
                } else {
                    view! { <span hidden></span> }.into_any()
                }
            }}
        </div>
    }
}
