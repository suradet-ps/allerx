//! Verdict band — prominent result banner at top of main canvas.
//! The signature element. One verdict per checked drug; a single-drug
//! check renders the full-size band, a batch renders stacked compact bands
//! (ROADMAP Phase 5).

use leptos::prelude::*;

use crate::components::icons::{IconCheckCircle, IconClock, IconXCircle};
use crate::state::{AppState, DrugVerdictState, VerdictState};

#[component]
pub fn VerdictBand(state: AppState) -> impl IntoView {
    move || match state.verdict.get() {
        VerdictState::Pending => {
            let has_patient = state.patient.get().is_some();
            let configured = state.configured.get();
            let hint = if !configured {
                "ยังไม่ได้ตั้งค่าการเชื่อมต่อ HOSxP — กดปุ่ม ตั้งค่า เพื่อเริ่มใช้งาน"
            } else if !has_patient {
                "เลือกผู้ป่วยทางด้านซ้าย แล้วพิมพ์ชื่อยาเพื่อตรวจประวัติ"
            } else {
                "พิมพ์ชื่อยาทางด้านซ้าย แล้วกดตรวจประวัติ"
            };
            Some(
                view! {
                    <section class="verdict-band verdict-pending">
                        <IconClock class="verdict-band__icon" />
                        <div class="verdict-band__content">
                            <p class="verdict-band__headline">"รอการค้นหา"</p>
                            <p class="verdict-band__detail">{hint}</p>
                        </div>
                    </section>
                }
                .into_any(),
            )
        }
        VerdictState::Results { results } => {
            if results.is_empty() {
                return None;
            }
            if results.len() == 1 {
                Some(render_single(&results[0], &state).into_any())
            } else {
                Some(render_batch(&results, &state).into_any())
            }
        }
    }
}

/// The single-drug verdict — the full-size band (DESIGN.md).
fn render_single(result: &crate::state::DrugVerdict, _state: &AppState) -> impl IntoView {
    match &result.state {
        DrugVerdictState::Found { records, truncated } => {
            let latest = records.first();
            let (date, visit_type, prescriber, department) = match latest {
                Some(record) => {
                    let visit_type = match record.visit_type {
                        allerx_models::VisitType::Opd => "OPD",
                        allerx_models::VisitType::Ipd => "IPD",
                    };
                    (
                        record.visit_date.format("%d/%m/%Y").to_string(),
                        visit_type,
                        record.prescriber.as_deref().unwrap_or("—").to_string(),
                        record.department.as_deref().unwrap_or("—").to_string(),
                    )
                }
                None => ("—".to_string(), "—", "—".to_string(), "—".to_string()),
            };
            let truncation_suffix = if *truncated {
                " — มีประวัติเก่ากว่านี้"
            } else {
                ""
            };
            view! {
                <section class="verdict-band verdict-found">
                    <IconCheckCircle class="verdict-band__icon" />
                    <div class="verdict-band__content">
                        <p class="verdict-band__headline">"พบประวัติการได้รับยานี้"</p>
                        <p class="verdict-band__detail">
                            {format!(
                                "ครั้งล่าสุด {date} ({visit_type}) โดย {prescriber} @ {department} — ทั้งหมด {} ครั้ง{truncation_suffix}",
                                records.len()
                            )}
                        </p>
                    </div>
                </section>
            }
            .into_any()
        }
        DrugVerdictState::NotFound => view! {
            <section class="verdict-band verdict-notfound">
                <IconXCircle class="verdict-band__icon" />
                <div class="verdict-band__content">
                    <p class="verdict-band__headline">"ไม่พบประวัติการได้รับยานี้"</p>
                    <p class="verdict-band__detail">"ไม่เคยมีรายการจ่ายยานี้ในประวัติผู้ป่วย"</p>
                </div>
            </section>
        }
        .into_any(),
        DrugVerdictState::Unresolved { candidates } => {
            let (headline, detail) = if candidates.is_empty() {
                (
                    "ไม่พบยานี้ในทะเบียนยา".to_string(),
                    "ไม่สามารถตรวจประวัติได้ — ตรวจสอบการสะกดชื่อยา หรือสอบถามผู้สั่งยา".to_string(),
                )
            } else {
                (
                    "ไม่สามารถยืนยันประวัติได้".to_string(),
                    "ไม่พบชื่อที่ตรงกับยาในทะเบียน — เลือกยาจากรายการแนะนำทางด้านซ้าย แล้วกดตรวจประวัติ"
                        .to_string(),
                )
            };
            view! {
                <section class="verdict-band verdict-unresolved">
                    <IconXCircle class="verdict-band__icon" />
                    <div class="verdict-band__content">
                        <p class="verdict-band__headline">{headline}</p>
                        <p class="verdict-band__detail">{detail}</p>
                    </div>
                </section>
            }
            .into_any()
        }
    }
}

/// The batch verdict — one compact band per checked drug, term-labelled.
fn render_batch(results: &[crate::state::DrugVerdict], state: &AppState) -> impl IntoView {
    view! {
        <section class="verdict-batch">
            {results
                .iter()
                .map(|result| {
                    let term = result.term.clone();
                    match &result.state {
                        DrugVerdictState::Found { records, truncated } => {
                            let latest = records.first();
                            let (date, visit_type, count) = match latest {
                                Some(record) => {
                                    let visit_type = match record.visit_type {
                                        allerx_models::VisitType::Opd => "OPD",
                                        allerx_models::VisitType::Ipd => "IPD",
                                    };
                                    (
                                        record.visit_date.format("%d/%m/%Y").to_string(),
                                        visit_type,
                                        format!("ทั้งหมด {} ครั้ง", records.len()),
                                    )
                                }
                                None => ("—".to_string(), "—", "ทั้งหมด 0 ครั้ง".to_string()),
                            };
                            let truncation = if *truncated { " — มีประวัติเก่ากว่านี้" } else { "" };
                            view! {
                                <div class="verdict-band verdict-band--compact verdict-found">
                                    <IconCheckCircle class="verdict-band__icon" />
                                    <div class="verdict-band__content">
                                        <p class="verdict-band__term">{term}</p>
                                        <p class="verdict-band__detail">
                                            {format!("พบประวัติ — ครั้งล่าสุด {date} ({visit_type}) · {count}{truncation}")}
                                        </p>
                                    </div>
                                </div>
                            }
                            .into_any()
                        }
                        DrugVerdictState::NotFound => view! {
                            <div class="verdict-band verdict-band--compact verdict-notfound">
                                <IconXCircle class="verdict-band__icon" />
                                <div class="verdict-band__content">
                                    <p class="verdict-band__term">{term}</p>
                                    <p class="verdict-band__detail">"ไม่พบประวัติการได้รับยานี้"</p>
                                </div>
                            </div>
                        }
                        .into_any(),
                        DrugVerdictState::Unresolved { candidates } => {
                            let add = state.drug_chips;
                            view! {
                                <div class="verdict-band verdict-band--compact verdict-unresolved">
                                    <IconXCircle class="verdict-band__icon" />
                                    <div class="verdict-band__content">
                                        <p class="verdict-band__term">{term.clone()}</p>
                                        <p class="verdict-band__detail">
                                            {if candidates.is_empty() {
                                                "ไม่พบยานี้ในทะเบียนยา — ตรวจสอบการสะกด".to_string()
                                            } else {
                                                "ไม่พบชื่อที่ตรงกับยาในทะเบียน — เลือกยาจากรายการ".to_string()
                                            }}
                                        </p>
                                        {if candidates.is_empty() {
                                            None
                                        } else {
                                            Some(
                                                view! {
                                                    <ul class="candidate-list">
                                                        {candidates
                                                            .iter()
                                                            .map(|drug| {
                                                                let label = drug.name.clone();
                                                                let icode = drug.icode.clone();
                                                                view! {
                                                                    <li>
                                                                        <button
                                                                            class="candidate-button"
                                                                            on:click=move |_| {
                                                                                let mut chips = add.get_untracked();
                                                                                let dup = chips.iter().any(|c| {
                                                                                    c.icode.as_deref() == Some(icode.as_str())
                                                                                        || c.label == label
                                                                                });
                                                                                if !dup {
                                                                                    chips.push(crate::state::DrugChip {
                                                                                        label: label.clone(),
                                                                                        icode: Some(icode.clone()),
                                                                                    });
                                                                                    add.set(chips);
                                                                                }
                                                                            }
                                                                        >
                                                                            {label.clone()}
                                                                        </button>
                                                                    </li>
                                                                }
                                                            })
                                                            .collect_view()}
                                                    </ul>
                                                }
                                                    .into_any(),
                                            )
                                        }}
                                    </div>
                                </div>
                            }
                            .into_any()
                        }
                    }
                })
                .collect_view()}
        </section>
    }
}
