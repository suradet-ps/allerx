//! Patient search box (DESIGN.md: search-input + search-result-row).
//!
//! M2: wired to the real backend — input auto-detects HN / CID / name
//! (AGENTS.md §7.1) and searches with a 250 ms debounce.

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use leptos::prelude::*;

use crate::api;
use crate::components::icons::IconSearch;
use crate::state::AppState;

/// Search-box debounce (AGENTS.md §7.1).
const DEBOUNCE_MS: u64 = 250;

/// Single search box for HN / CID / name (AGENTS.md §7.1).
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
    let run_search_click = Rc::clone(&run_search);
    let run_search_typed = Rc::clone(&run_search);
    let debounce = Rc::new(Cell::new(None::<TimeoutHandle>));

    view! {
        <section class="panel">
            <label class="panel__label" for="patient-search">
                "ค้นหาผู้ป่วย"
            </label>
            <div class="search-row">
                <input
                    id="patient-search"
                    class="search-input"
                    placeholder="เลข HN / เลขบัตรประชาชน / ชื่อ-นามสกุล"
                    prop:value=move || term.get()
                    on:input=move |ev| {
                        let value = event_target_value(&ev);
                        term.set(value);
                        if let Some(handle) = debounce.get() {
                            handle.clear();
                        }
                        let schedule = Rc::clone(&run_search_typed);
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
                <button class="button-primary" on:click=move |_| run_search_click()>
                    <IconSearch class="icon" />
                    "ค้นหา"
                </button>
            </div>
            {move || {
                if let Some(message) = error.get() {
                    view! { <p class="placeholder-note">{message}</p> }.into_any()
                } else if searched.get() && results.get().is_empty() {
                    view! { <p class="placeholder-note">"ไม่พบผู้ป่วย"</p> }.into_any()
                } else if results.get().is_empty() {
                    view! {
                        <p class="placeholder-note">
                            "พิมพ์เลข HN / เลขบัตรประชาชน / ชื่อ-นามสกุล เพื่อค้นหา"
                        </p>
                    }
                        .into_any()
                } else {
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
                                                on:click=move |_| state.patient.set(Some(patient.clone()))
                                            >
                                                <span class="search-result-row__name">
                                                    {p.full_name_th.clone()}
                                                </span>
                                                <span class="search-result-row__code">
                                                    {p.hn.clone()}
                                                </span>
                                            </li>
                                        }
                                    })
                                    .collect_view()
                            }}
                        </ul>
                    }
                        .into_any()
                }
            }}
        </section>
    }
}
