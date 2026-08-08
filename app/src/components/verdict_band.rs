//! The verdict band — the signature element (DESIGN.md).
//!
//! Full-width, high-contrast strip that appears the instant a drug search
//! resolves. Only one band exists at a time and it fully replaces the
//! previous one. Red/green appear **nowhere** else in the app.

use leptos::prelude::*;

use crate::components::icons::{IconCheckCircle, IconClock, IconXCircle};
use crate::state::{AppState, VerdictState};

/// Renders the current verdict state, or the neutral pending band.
#[component]
pub fn VerdictBand(state: AppState) -> impl IntoView {
    move || match state.verdict.get() {
        VerdictState::Pending => Some(
            view! {
                <section class="verdict-band verdict-pending">
                    <IconClock class="verdict-band__icon" />
                    <div>
                        <p class="verdict-band__headline">"รอการค้นหา"</p>
                        <p class="verdict-band__detail">
                            {if state.configured.get() {
                                "เลือกผู้ป่วยและพิมพ์ชื่อยา แล้วกดตรวจประวัติ"
                            } else {
                                "ยังไม่ได้ตั้งค่าการเชื่อมต่อ HOSxP — กดปุ่ม ตั้งค่า เพื่อเริ่มใช้งาน"
                            }}
                        </p>
                    </div>
                </section>
            }
            .into_any(),
        ),
        VerdictState::Found { records } => {
            let latest = records.first()?;
            let visit_type = match latest.visit_type {
                allerx_models::VisitType::Opd => "OPD",
                allerx_models::VisitType::Ipd => "IPD",
            };
            let date = latest.visit_date.format("%d/%m/%Y").to_string();
            Some(
                view! {
                    <section class="verdict-band verdict-found">
                        <IconCheckCircle class="verdict-band__icon" />
                        <div>
                            <p class="verdict-band__headline">"พบประวัติการได้รับยานี้"</p>
                            <p class="verdict-band__detail">
                                {format!("ได้รับครั้งล่าสุดเมื่อ {date} ({visit_type}) — ทั้งหมด {} ครั้ง", records.len())}
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
                    <div>
                        <p class="verdict-band__headline">"ไม่พบประวัติการได้รับยานี้"</p>
                        <p class="verdict-band__detail">"ไม่เคยพบรายการจ่ายยานี้ในประวัติผู้ป่วย"</p>
                    </div>
                </section>
            }
            .into_any(),
        ),
    }
}
