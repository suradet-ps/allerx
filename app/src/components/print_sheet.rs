//! Print sheet (ROADMAP Phase 5) — a printable Thai patient+history sheet
//! attached to a consultation note. Hidden on screen (`display: none`),
//! the only thing visible in `@media print`. Read-only and ephemeral: no
//! files are written, `window.print()` renders the current state.

use leptos::prelude::*;

use crate::state::{AppState, DrugVerdictState, VerdictState, merged_timeline};

#[component]
pub fn PrintSheet(state: AppState) -> impl IntoView {
    move || {
        let VerdictState::Results { results } = state.verdict.get() else {
            return None;
        };
        if results.is_empty() {
            return None;
        }
        let patient = state.patient.get()?;
        let (records, truncated) = merged_timeline(&results);
        let printed_on = chrono::Local::now().format("%d/%m/%Y %H:%M").to_string();
        Some(
            view! {
                <div class="print-sheet">
                    <header class="print-sheet__header">
                        <h1>"AllerX — ใบประวัติการได้รับยา"</h1>
                        <p>{format!("พิมพ์เมื่อ: {printed_on}")}</p>
                    </header>
                    <section class="print-sheet__patient">
                        <h2>"ข้อมูลผู้ป่วย"</h2>
                        <table>
                            <tr>
                                <td>"ชื่อ"</td>
                                <td>{patient.full_name_th.clone()}</td>
                                <td>"HN"</td>
                                <td>{patient.hn.clone()}</td>
                            </tr>
                            <tr>
                                <td>"CID"</td>
                                <td>{patient.cid.clone().unwrap_or_else(|| "—".to_string())}</td>
                                <td>"วันเกิด"</td>
                                <td>
                                    {patient
                                        .birth_date
                                        .map(|d| d.format("%d/%m/%Y").to_string())
                                        .unwrap_or_else(|| "—".to_string())}
                                </td>
                            </tr>
                        </table>
                    </section>
                    <section class="print-sheet__verdicts">
                        <h2>"ผลการตรวจ"</h2>
                        <table>
                            {results
                                .iter()
                                .map(|r| {
                                    let (status, detail) = match &r.state {
                        DrugVerdictState::Found { records, truncated } => {
                            let latest = records.first();
                            let when = latest
                                .map(|rec| {
                                    let vt = match rec.visit_type {
                                        allerx_models::VisitType::Opd => "OPD",
                                        allerx_models::VisitType::Ipd => "IPD",
                                    };
                                    format!(
                                        "ครั้งล่าสุด {} ({}) — ทั้งหมด {} ครั้ง",
                                        rec.visit_date.format("%d/%m/%Y"),
                                        vt,
                                        records.len()
                                    )
                                })
                                .unwrap_or_else(|| "—".to_string());
                            let extra = if *truncated {
                                " — มีประวัติเก่ากว่านี้"
                            } else {
                                ""
                            };
                            ("พบประวัติ", format!("{when}{extra}"))
                        }
                                        DrugVerdictState::NotFound => {
                                            ("ไม่พบประวัติ", "ไม่เคยมีรายการจ่ายยานี้".to_string())
                                        }
                                        DrugVerdictState::Unresolved { .. } => {
                                            (
                                                "ไม่สามารถยืนยันได้",
                                                "ไม่พบยานี้ในทะเบียนยา".to_string(),
                                            )
                                        }
                                    };
                                    let term = r.term.clone();
                                    view! {
                                        <tr>
                                            <td>{term}</td>
                                            <td>{status}</td>
                                            <td>{detail}</td>
                                        </tr>
                                    }
                                })
                                .collect_view()}
                        </table>
                    </section>
                    <section class="print-sheet__timeline">
                        <h2>"ประวัติการได้รับยา"</h2>
                        {if records.is_empty() {
                            view! { <p>"ไม่มีรายการ"</p> }.into_any()
                        } else {
                            view! {
                                <table>
                                    <thead>
                                        <tr>
                                            <th>"วันที่"</th>
                                            <th>"ประเภท"</th>
                                            <th>"ยา"</th>
                                            <th>"แพทย์"</th>
                                            <th>"แผนก"</th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        {records
                                            .iter()
                                            .map(|r| {
                                                let vt = match r.visit_type {
                                                    allerx_models::VisitType::Opd => "OPD",
                                                    allerx_models::VisitType::Ipd => "IPD",
                                                };
                                                let prescriber = r.prescriber.as_deref().unwrap_or("—");
                                                let department = r.department.as_deref().unwrap_or("—");
                                                view! {
                                                    <tr>
                                                        <td>{r.visit_date.format("%d/%m/%Y").to_string()}</td>
                                                        <td>{vt}</td>
                                                        <td>{r.drug_name.clone()}</td>
                                                        <td>{prescriber}</td>
                                                        <td>{department}</td>
                                                    </tr>
                                                }
                                            })
                                            .collect_view()}
                                    </tbody>
                                </table>
                            }
                                .into_any()
                        }}
                    </section>
                    <footer class="print-sheet__footer">
                        {if truncated {
                            view! { <p>"หมายเหตุ: มีประวัติเก่ากว่านี้ที่ไม่ได้แสดงในใบนี้"</p> }.into_any()
                        } else {
                            view! { <span></span> }.into_any()
                        }}
                        <p>
                            "เอกสารนี้พิมพ์จาก AllerX เพื่อประกอบการประเมินแพ้ยา — HOSxP ยังคงเป็นแหล่งข้อมูลหลัก ตรวจสอบกับเวชระเบียนก่อนตัดสินใจ"
                        </p>
                    </footer>
                </div>
            }
            .into_any(),
        )
    }
}
