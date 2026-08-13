//! Timeline — dense scrollable list in main canvas.
//! Two-line rows: date | badge | drug (line 1), meta (line 2).
//! Since Phase 5 the timeline merges every found drug's records into one
//! chronological view (each row already shows its drug name).

use leptos::prelude::*;

use crate::components::icons::IconCalendar;
use crate::state::{AppState, VerdictState, merged_timeline};

#[component]
pub fn Timeline(state: AppState) -> impl IntoView {
    move || {
        let VerdictState::Results { results } = state.verdict.get() else {
            return None;
        };
        let (records, truncated) = merged_timeline(&results);
        if records.is_empty() {
            return None;
        }
        let total = records.len();
        let footer = if truncated {
            format!("แสดง {total} รายการล่าสุด — มีประวัติเก่ากว่านี้")
        } else {
            format!("ทั้งหมด {total} รายการ")
        };
        Some(
            view! {
                <>
                    <h3 class="timeline-header">
                        <IconCalendar class="icon" />
                        "ประวัติการได้รับยา"
                    </h3>
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
                </>
            }
            .into_any(),
        )
    }
}
