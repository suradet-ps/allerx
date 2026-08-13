//! Drug search — sidebar section (DESIGN.md: sidebar__section).
//! Autocomplete drug search with a chip queue: type/pick drugs, then check
//! them all at once (ROADMAP Phase 5 — a single drug is a batch of one).

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use leptos::prelude::*;

use crate::api;
use crate::components::icons::{IconPill, IconSearch, IconX};
use crate::state::{AppState, DrugChip, DrugVerdict, DrugVerdictState, VerdictState};

const DEBOUNCE_MS: u64 = 250;

#[component]
pub fn DrugSearch(state: AppState) -> impl IntoView {
    let term = RwSignal::new(String::new());
    let suggestions = RwSignal::new(Vec::<allerx_models::DrugItem>::new());
    let selected_icode = RwSignal::new(Option::<String>::None);
    let note = RwSignal::new(Option::<String>::None);

    // Queues one drug for checking (deduped by icode or label).
    let add_chip = Rc::new(move |label: String, icode: Option<String>| {
        let chips = state.drug_chips.get_untracked();
        let duplicate = chips.iter().any(|c| {
            if let (Some(a), Some(b)) = (&c.icode, &icode) {
                a == b
            } else {
                c.label == label
            }
        });
        if !duplicate {
            state.drug_chips.update(|chips| {
                chips.push(DrugChip { label, icode });
            });
        }
    });
    let add_chip_typed = Rc::clone(&add_chip);
    let debounce = Rc::new(Cell::new(None::<TimeoutHandle>));

    // Runs the batch check for the queued chips (or the typed term when
    // nothing is queued).
    let run_check = Rc::new(move || {
        let Some(patient) = state.patient.get_untracked() else {
            note.set(Some("เลือกผู้ป่วยก่อน".to_string()));
            return;
        };
        let chips = state.drug_chips.get_untracked();
        let mut terms: Vec<String> = chips
            .iter()
            .map(|c| c.icode.clone().unwrap_or_else(|| c.label.clone()))
            .collect();
        if terms.is_empty() {
            let typed = term.get_untracked().trim().to_string();
            if typed.is_empty() {
                return;
            }
            terms.push(typed);
        }
        note.set(None);
        let hn = patient.hn.clone();
        leptos::task::spawn_local(async move {
            match api::check_history(&hn, &terms).await {
                Ok(results) => {
                    state.db_banner.set(None);
                    let verdicts = results
                        .into_iter()
                        .map(|r| DrugVerdict {
                            term: r.term,
                            state: match r.verdict {
                                allerx_models::HistoryVerdict::Resolved { history } => {
                                    if history.records.is_empty() {
                                        DrugVerdictState::NotFound
                                    } else {
                                        DrugVerdictState::Found {
                                            records: history.records,
                                            truncated: history.truncated,
                                        }
                                    }
                                }
                                allerx_models::HistoryVerdict::Unresolved { candidates } => {
                                    DrugVerdictState::Unresolved { candidates }
                                }
                            },
                        })
                        .collect::<Vec<_>>();
                    // Single-drug unresolved: the band points at the
                    // suggestions list — make the backend's candidates
                    // visible there so disambiguation is one click away.
                    if let [
                        DrugVerdict {
                            state: DrugVerdictState::Unresolved { candidates },
                            ..
                        },
                    ] = &verdicts[..]
                    {
                        suggestions.set(candidates.clone());
                    }
                    state
                        .verdict
                        .set(VerdictState::Results { results: verdicts });
                }
                Err(err) => {
                    state.verdict.set(VerdictState::Pending);
                    // Reachability failures raise the degraded-mode banner
                    // (ROADMAP Phase 3); everything else stays inline.
                    if matches!(
                        err.kind,
                        api::ApiErrorKind::Connection | api::ApiErrorKind::NotConfigured
                    ) {
                        state.db_banner.set(Some(err.message));
                    } else {
                        note.set(Some(err.message));
                    }
                }
            }
        });
    });
    let run_check_click = Rc::clone(&run_check);

    let clear_input = move |_| {
        term.set(String::new());
        suggestions.set(Vec::new());
        selected_icode.set(None);
        note.set(None);
    };

    let clear_all = move |_ev: leptos::ev::MouseEvent| {
        term.set(String::new());
        suggestions.set(Vec::new());
        selected_icode.set(None);
        note.set(None);
        state.drug_chips.set(Vec::new());
        state.verdict.set(VerdictState::Pending);
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
                                    Ok(list) => {
                                        state.db_banner.set(None);
                                        suggestions.set(list);
                                    }
                                    Err(err) => {
                                        suggestions.set(Vec::new());
                                        if matches!(
                                            err.kind,
                                            api::ApiErrorKind::Connection
                                                | api::ApiErrorKind::NotConfigured
                                        ) {
                                            state.db_banner.set(Some(err.message));
                                        } else {
                                            note.set(Some(err.message));
                                        }
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
                            let value = term.get_untracked().trim().to_string();
                            if !value.is_empty() {
                                add_chip_typed(value, None);
                                term.set(String::new());
                                suggestions.set(Vec::new());
                                selected_icode.set(None);
                            }
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
                let chips = state.drug_chips.get();
                if chips.is_empty() {
                    None
                } else {
                    Some(
                        view! {
                            <>
                                <ul class="chip-list">
                                    {chips
                                        .into_iter()
                                        .map(|chip| {
                                            let chip_label = chip.label.clone();
                                            let chip_icode = chip.icode.clone();
                                            let aria = format!("ลบ {chip_label}");
                                            let remove_chip = move |_| {
                                                state.drug_chips.update(|chips| {
                                                    chips.retain(|c| {
                                                        c.label != chip_label || c.icode != chip_icode
                                                    });
                                                });
                                            };
                                            view! {
                                                <li class="chip">
                                                    <span class="chip__label">{chip.label.clone()}</span>
                                                    <button
                                                        class="chip__remove"
                                                        on:click=remove_chip
                                                        aria-label=aria
                                                    >
                                                        <IconX class="icon" />
                                                    </button>
                                                </li>
                                            }
                                        })
                                        .collect_view()}
                                </ul>
                                <button
                                    class="button-ghost button-ghost--clear"
                                    on:click=clear_all
                                >
                                    "ล้างทั้งหมด"
                                </button>
                            </>
                        }
                            .into_any(),
                    )
                }
            }}
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
                let list = suggestions.get();
                if list.is_empty() {
                    None
                } else {
                    Some(
                        view! {
                            <ul class="result-list">
                                {list
                                    .into_iter()
                                    .map(|drug| {
                                        let strength_suffix = drug
                                            .strength
                                            .as_deref()
                                            .map(|s| format!(" ({s})"))
                                            .unwrap_or_default();
                                        let label = format!(
                                            "{}{}", drug.name, strength_suffix
                                        );
                                        let trade_suffix = drug
                                            .trade_name
                                            .as_deref()
                                            .map(|t| format!(" · {t}"))
                                            .unwrap_or_default();
                                        let state = state.clone();
                                        let icode = drug.icode.clone();
                                        view! {
                                            <li
                                                class="search-result-row"
                                                on:click=move |_| {
                                                    let mut chips = state.drug_chips.get_untracked();
                                                    let dup = chips.iter().any(|c| {
                                                        c.icode.as_deref() == Some(icode.as_str())
                                                            || c.label == label
                                                    });
                                                    if !dup {
                                                        chips.push(crate::state::DrugChip {
                                                            label: label.clone(),
                                                            icode: Some(icode.clone()),
                                                        });
                                                        state.drug_chips.set(chips);
                                                    }
                                                    term.set(String::new());
                                                    suggestions.set(Vec::new());
                                                    selected_icode.set(None);
                                                    note.set(None);
                                                }
                                            >
                                                <span class="search-result-row__name">
                                                    {drug.name.clone()}
                                                </span>
                                                <span class="search-result-row__code">
                                                    {format!("{}{}", trade_suffix, drug.icode)}
                                                </span>
                                            </li>
                                        }
                                    })
                                    .collect_view()}
                            </ul>
                        }
                        .into_any(),
                    )
                }
            }}
        </div>
    }
}
