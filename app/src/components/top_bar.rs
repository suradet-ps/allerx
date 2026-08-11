//! Top bar — thin 40px header spanning full width.
//! Shows app name, connection status dot, and settings button.

use leptos::prelude::*;

use crate::components::icons::IconSettings;
use crate::state::AppState;

#[component]
pub fn TopBar(state: AppState) -> impl IntoView {
    let connected = state.configured;

    view! {
        <header class="top-bar">
            <div class="top-bar__left">
                <h1 class="top-bar__title">"AllerX"</h1>
            </div>
            <div class="top-bar__right">
                <div class="top-bar__status">
                    <span
                        class=move || {
                            if connected.get() {
                                "top-bar__status-dot"
                            } else {
                                "top-bar__status-dot top-bar__status-dot--disconnected"
                            }
                        }
                    ></span>
                    <span class="top-bar__status-text">
                        {move || {
                            if connected.get() {
                                "เชื่อมต่อแล้ว"
                            } else {
                                "ไม่ได้เชื่อมต่อ"
                            }
                        }}
                    </span>
                </div>
                <button
                    class="top-bar__button"
                    on:click=move |_| state.settings_open.set(true)
                    data-tooltip="ตั้งค่าการเชื่อมต่อ"
                >
                    <IconSettings class="icon" />
                    "ตั้งค่า"
                </button>
            </div>
        </header>
    }
}
