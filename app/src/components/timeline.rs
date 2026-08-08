//! Medication-history timeline (DESIGN.md: timeline-row).
//!
//! Rows are read top-to-bottom, most recent first — order carries clinical
//! meaning here. Shown only when the verdict is "found".

use leptos::prelude::*;

use crate::components::icons::IconCalendar;
use crate::state::{AppState, VerdictState};

/// Full history list for the searched drug. Renders nothing until a found
/// verdict exists.
#[component]
pub fn Timeline(state: AppState) -> impl IntoView {
    move || {
        let VerdictState::Found { records } = state.verdict.get() else {
            return None;
        };
        if records.is_empty() {
            return None;
        }
        Some(
            view! {
                <section class="panel">
                    <h2 class="panel__heading">
                        <IconCalendar class="icon" />
                        "ประวัติการได้รับยา"
                    </h2>
                <ul class="timeline">
                    {records
                        .iter()
                        .map(|r| {
                            let visit_type = match r.visit_type {
                                allerx_models::VisitType::Opd => "OPD",
                                allerx_models::VisitType::Ipd => "IPD",
                            };
                            let date = r.visit_date.format("%d/%m/%Y").to_string();
                            let prescriber = r.prescriber.as_deref().unwrap_or("—");
                            let department = r.department.as_deref().unwrap_or("—");
                            view! {
                                <li class="timeline-row">
                                    <span class="timeline-row__date">{date}</span>
                                    <span class="badge">{visit_type}</span>
                                    <div class="timeline-row__main">
                                        <p class="timeline-row__drug">{r.drug_name.clone()}</p>
                                        <p class="timeline-row__meta">
                                            {format!("แพทย์ {prescriber} — {department}")}
                                        </p>
                                    </div>
                                </li>
                            }
                        })
                        .collect_view()}
                </ul>
                </section>
            }
            .into_any(),
        )
    }
}
