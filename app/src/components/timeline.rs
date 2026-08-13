//! Timeline — dense scrollable list in main canvas.
//! Two-line rows: date | badge | drug (line 1), meta (line 2).
//! Since Phase 5 the timeline merges every found drug's records into one
//! chronological view (each row already shows its drug name); a filter bar
//! lets the pharmacist isolate a single drug's history when many drugs
//! were checked at once.

use leptos::prelude::*;

use crate::components::icons::IconCalendar;
use crate::state::{AppState, DrugVerdictState, VerdictState, merged_timeline};

/// A cheap content fingerprint of a results list — used to detect "a new
/// check replaced the view" without an Effect (see [`Timeline`]).
fn results_key(results: &[crate::state::DrugVerdict]) -> String {
    results
        .iter()
        .map(|v| {
            let (count, truncated) = match &v.state {
                DrugVerdictState::Found {
                    records, truncated, ..
                } => (records.len(), *truncated),
                _ => (0, false),
            };
            format!("{}|{count}|{truncated}", v.term)
        })
        .collect::<Vec<_>>()
        .join(",")
}

#[component]
pub fn Timeline(state: AppState) -> impl IntoView {
    // The checked-drug term the timeline is filtered to (`None` = all).
    let filter = RwSignal::new(None::<String>);
    // A new check result replaces the view — reset the filter with it.
    // Tracked by a content key, NOT an Effect: writing to a signal from an
    // Effect during mount broke the event-listener attachment in the wasm
    // tests (leptos 0.8), so the reset happens here in the render closure,
    // guarded so it only fires when the results actually change.
    let last_results_key = RwSignal::new(None::<String>);

    move || {
        let filter_term = filter.get();
        let VerdictState::Results { results } = state.verdict.get() else {
            last_results_key.set(None);
            return None;
        };
        let key = results_key(&results);
        if last_results_key.get_untracked().as_deref() != Some(key.as_str()) {
            last_results_key.set(Some(key));
            filter.set(None);
        }
        let (all_records, all_truncated) = merged_timeline(&results);
        let (records, truncated) = match filter_term.as_deref() {
            Some(term) => {
                let mut recs = results
                    .iter()
                    .find(|v| v.term == term)
                    .map(|v| match &v.state {
                        DrugVerdictState::Found { records, .. } => records.clone(),
                        _ => Vec::new(),
                    })
                    .unwrap_or_default();
                recs.sort_by_key(|r| std::cmp::Reverse(r.visit_date));
                let trunc = results.iter().any(|v| {
                    v.term == term
                        && matches!(
                            &v.state,
                            DrugVerdictState::Found {
                                truncated: true,
                                ..
                            }
                        )
                });
                (recs, trunc)
            }
            None => (all_records, all_truncated),
        };
        if records.is_empty() {
            return None;
        }
        let total = records.len();
        let footer = if truncated {
            format!("แสดง {total} รายการล่าสุด — มีประวัติเก่ากว่านี้")
        } else {
            format!("ทั้งหมด {total} รายการ")
        };
        let multi = results.len() > 1;
        Some(
            view! {
                <div class="timeline-view">
                    <h3 class="timeline-header">
                        <IconCalendar class="icon" />
                        "ประวัติการได้รับยา"
                    </h3>
                    {if multi {
                        Some(
                            view! {
                                <div class="timeline-filter">
                                    <button
                                        class="timeline-filter__chip"
                                        class:timeline-filter__chip--active=move || filter.get().is_none()
                                        on:click=move |_| filter.set(None)
                                    >
                                        "ทั้งหมด"
                                    </button>
                                    {results
                                        .iter()
                                        .map(|v| {
                                            let term = v.term.clone();
                                            let term_for_class = term.clone();
                                            let filter_for = filter;
                                            view! {
                                                <button
                                                    class="timeline-filter__chip"
                                                    class:timeline-filter__chip--active=move || {
                                                        filter_for.get().as_deref() == Some(term_for_class.as_str())
                                                    }
                                                    on:click=move |_| {
                                                        if filter_for.get().as_deref() == Some(term.as_str()) {
                                                            filter_for.set(None);
                                                        } else {
                                                            filter_for.set(Some(term.clone()));
                                                        }
                                                    }
                                                >
                                                    {term.clone()}
                                                </button>
                                            }
                                        })
                                        .collect_view()}
                                </div>
                            }
                                .into_any(),
                        )
                    } else {
                        None
                    }}
                    <ul class="timeline">
                        {records
                            .iter()
                            .map(|r| {
                                let visit_type = match r.visit_type {
                                    allerx_models::VisitType::Opd => "OPD",
                                    allerx_models::VisitType::Ipd => "IPD",
                                };
                                let date = r.visit_date.format("%d/%m/%Y").to_string();
                                let drug_label = crate::state::record_label(r);
                                let prescriber = r.prescriber.as_deref().unwrap_or("—");
                                let department = r.department.as_deref().unwrap_or("—");
                                // The meta line carries the prescriber and
                                // department only — quantity and usage are
                                // not shown (pilot feedback).
                                let meta_text = format!("{prescriber} @ {department}");
                                view! {
                                    <li class="timeline-row">
                                        <span class="timeline-row__date">{date}</span>
                                        <span class="timeline-row__badge">
                                            <span class="badge">{visit_type}</span>
                                        </span>
                                        <div class="timeline-row__main">
                                            <p class="timeline-row__drug">{drug_label}</p>
                                            <p class="timeline-row__meta">{meta_text}</p>
                                        </div>
                                    </li>
                                }
                            })
                            .collect_view()}
                    </ul>
                    <p class="timeline-footer">{footer}</p>
                </div>
            }
            .into_any(),
        )
    }
}
