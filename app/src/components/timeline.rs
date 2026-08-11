//! Timeline — dense scrollable list in main canvas.
//! Two-line rows: date | badge | drug (line 1), meta (line 2).

use leptos::prelude::*;

use crate::components::icons::IconCalendar;
use crate::state::{AppState, VerdictState};

#[component]
pub fn Timeline(state: AppState) -> impl IntoView {
    move || {
        let VerdictState::Found { records } = state.verdict.get() else {
            return None;
        };
        if records.is_empty() {
            return None;
        }
        let total = records.len();
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
                                let prescriber = r.prescriber.as_deref().unwrap_or("—");
                                let department = r.department.as_deref().unwrap_or("—");
                                let quantity = r.quantity.as_deref().unwrap_or("");
                                let route = r.route.as_deref().unwrap_or("");
                                let mut meta = vec![format!("{prescriber} @ {department}")];
                                if !quantity.is_empty() {
                                    meta.push(format!("x{quantity}"));
                                }
                                if !route.is_empty() {
                                    meta.push(route.to_string());
                                }
                                let meta_text = meta.join(" · ");
                                view! {
                                    <li class="timeline-row">
                                        <span class="timeline-row__date">{date}</span>
                                        <span class="timeline-row__badge">
                                            <span class="badge">{visit_type}</span>
                                        </span>
                                        <div class="timeline-row__main">
                                            <p class="timeline-row__drug">{r.drug_name.clone()}</p>
                                            <p class="timeline-row__meta">{meta_text}</p>
                                        </div>
                                    </li>
                                }
                            })
                            .collect_view()}
                    </ul>
                    <p class="timeline-footer">{format!("ทั้งหมด {total} รายการ")}</p>
                </>
            }
            .into_any(),
        )
    }
}
