//! Verdict band — prominent result banner at top of main canvas.
//! The signature element. Only one verdict on screen at a time.

use leptos::prelude::*;

use crate::components::icons::{IconCheckCircle, IconClock, IconXCircle};
use crate::state::{AppState, VerdictState};

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
        VerdictState::Found { records } => {
            let latest = records.first()?;
            let visit_type = match latest.visit_type {
                allerx_models::VisitType::Opd => "OPD",
                allerx_models::VisitType::Ipd => "IPD",
            };
            let date = latest.visit_date.format("%d/%m/%Y").to_string();
            let prescriber = latest.prescriber.as_deref().unwrap_or("—");
            let department = latest.department.as_deref().unwrap_or("—");
            Some(
                view! {
                    <section class="verdict-band verdict-found">
                        <IconCheckCircle class="verdict-band__icon" />
                        <div class="verdict-band__content">
                            <p class="verdict-band__headline">"พบประวัติการได้รับยานี้"</p>
                            <p class="verdict-band__detail">
                                {format!(
                                    "ครั้งล่าสุด {date} ({visit_type}) โดย {prescriber} @ {department} — ทั้งหมด {} ครั้ง",
                                    records.len()
                                )}
                            </p>
                        </div>
                    </section>
                }
                .into_any(),
            )
        }
        VerdictState::NotFound => Some(
            view! {
                <section class="verdict-band verdict-notfound">
                    <IconXCircle class="verdict-band__icon" />
                    <div class="verdict-band__content">
                        <p class="verdict-band__headline">"ไม่พบประวัติการได้รับยานี้"</p>
                        <p class="verdict-band__detail">"ไม่เคยมีรายการจ่ายยานี้ในประวัติผู้ป่วย"</p>
                    </div>
                </section>
            }
            .into_any(),
        ),
    }
}
