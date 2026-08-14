//! Patient detail modal (ROADMAP Phase 5) — the DESIGN.md-mandated detail
//! view where the full CID is revealed, plus the "ยาที่ได้รับล่าสุด"
//! concurrent-medications snapshot (read-only, last 30 days).

use std::rc::Rc;

use leptos::prelude::*;
use leptos::ev;

use crate::api;
use crate::components::icons::{IconUser, IconX};
use crate::state::AppState;

/// Modal visibility + snapshot loading/error states.
#[derive(Debug, Clone, Default)]
enum MedsState {
    #[default]
    Idle,
    Loading,
    Loaded {
        meds: Vec<allerx_models::ConcurrentMedication>,
    },
    Failed {
        message: String,
    },
}

#[component]
pub fn PatientDetailModal(state: AppState) -> impl IntoView {
    let meds = RwSignal::new(MedsState::Idle);

    // Load the recent-medications snapshot every time the modal opens.
    Effect::new(move |_| {
        let open = state.detail_open.get();
        if !open {
            return;
        }
        let Some(patient) = state.patient.get_untracked() else {
            return;
        };
        let hn = patient.hn.clone();
        meds.set(MedsState::Loading);
        leptos::task::spawn_local(async move {
            match api::fetch_concurrent_medications(&hn).await {
                Ok(list) => meds.set(MedsState::Loaded { meds: list }),
                Err(err) => meds.set(MedsState::Failed {
                    message: err.message,
                }),
            }
        });
    });

    let close = Rc::new(move |_: ()| {
        state.detail_open.set(false);
        meds.set(MedsState::Idle);
    });

    // Escape closes the dialog from anywhere (DESIGN.md "Focus Management") —
    // window-level listener, guarded by the open flag so an Escape pressed
    // elsewhere never resets the meds snapshot state. The handle must stay
    // alive for the component's lifetime; dropping it unregisters the
    // listener.
    let escape_state = state.clone();
    let close_on_escape = Rc::clone(&close);
    let escape_handle = window_event_listener(ev::keydown, move |event| {
        if event.key() == "Escape" && escape_state.detail_open.get_untracked() {
            close_on_escape(());
        }
    });
    let _escape_handle = StoredValue::new(escape_handle);

    // Clones for the view's on:click closures (each captures its own).
    let backdrop_close = Rc::clone(&close);
    let button_close = Rc::clone(&close);

    view! {
        <div
            class="modal-backdrop"
            style:display=move || {
                if state.detail_open.get() {
                    "flex"
                } else {
                    "none"
                }
            }
            on:click=move |_| backdrop_close(())
        >
            <section class="modal modal--wide" on:click=move |ev| ev.stop_propagation()>
                <div class="modal__header">
                    <h2 class="modal__title">
                        <IconUser class="icon modal__title-icon" />
                        "ข้อมูลผู้ป่วย"
                    </h2>
                    <button class="button-ghost" on:click=move |_| button_close(()) aria-label="ปิด">
                        <IconX class="icon" />
                    </button>
                </div>

                {move || {
                    let patient = state.patient.get()?;
                    let cid = patient
                        .cid
                        .as_deref()
                        .unwrap_or("—")
                        .to_string();
                    let dob = patient
                        .birth_date
                        .map(|d| d.format("%d/%m/%Y").to_string())
                        .unwrap_or_else(|| "—".to_string());
                    let sex = match patient.sex.as_deref() {
                        Some("1") => "ชาย",
                        Some("2") => "หญิง",
                        _ => "ไม่ระบุ",
                    };
                    Some(
                        view! {
                            <div class="detail-grid">
                                <div class="detail-row">
                                    <span class="detail-row__label">"ชื่อ"</span>
                                    <span class="detail-row__value">{patient.full_name_th.clone()}</span>
                                </div>
                                <div class="detail-row">
                                    <span class="detail-row__label">"HN"</span>
                                    <span class="detail-row__value detail-row__value--mono">{patient.hn.clone()}</span>
                                </div>
                                <div class="detail-row">
                                    <span class="detail-row__label">"CID"</span>
                                    <span class="detail-row__value detail-row__value--mono">{cid}</span>
                                </div>
                                <div class="detail-row">
                                    <span class="detail-row__label">"วันเกิด"</span>
                                    <span class="detail-row__value">{dob}</span>
                                </div>
                                <div class="detail-row">
                                    <span class="detail-row__label">"เพศ"</span>
                                    <span class="detail-row__value">{sex}</span>
                                </div>
                            </div>
                        }
                        .into_any(),
                    )
                }}

                <h3 class="modal__section-title">"ยาที่ได้รับล่าสุด (30 วัน)"</h3>
                {move || match meds.get() {
                    MedsState::Idle | MedsState::Loading => {
                        view! { <p class="sidebar__empty">"กำลังโหลด..."</p> }.into_any()
                    }
                    MedsState::Failed { message } => {
                        view! { <p class="sidebar__empty">{message}</p> }.into_any()
                    }
                    MedsState::Loaded { meds } if meds.is_empty() => {
                        view! { <p class="sidebar__empty">"ไม่มีรายการจ่ายยาใน 30 วันที่ผ่านมา"</p> }.into_any()
                    }
                    MedsState::Loaded { meds } => view! {
                        <ul class="med-list">
                            {meds
                                .into_iter()
                                .map(|m| {
                                    let strength = m
                                        .strength
                                        .as_deref()
                                        .map(|s| format!(" ({s})"))
                                        .unwrap_or_default();
                                    let trade = m
                                        .trade_name
                                        .as_deref()
                                        .map(|t| format!(" ({t})"))
                                        .unwrap_or_default();
                                    view! {
                                        <li class="med-row">
                                            <span class="med-row__name">
                                                {format!("{}{}{}", m.drug_name, strength, trade)}
                                            </span>
                                            <span class="med-row__meta">
                                                {m.last_date.format("%d/%m/%Y").to_string()}
                                            </span>
                                        </li>
                                    }
                                })
                                .collect_view()}
                        </ul>
                    }
                    .into_any(),
                }}
            </section>
        </div>
    }
}
