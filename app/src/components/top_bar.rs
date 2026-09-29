//! Top bar — thin 40px header spanning full width.
//! Shows app name, connection status dot, and settings button.

use leptos::prelude::*;

use crate::components::icons::IconSettings;
use crate::state::{AppState, ConnectionHealth};

#[component]
pub fn TopBar(state: AppState) -> impl IntoView {
    view! {
        <header class="top-bar">
            <div class="top-bar__left">
                <svg class="top-bar__logo" viewBox="0 0 512 512" xmlns="http://www.w3.org/2000/svg">
                    <defs>
                        <linearGradient id="logo-bg" x1="0%" y1="0%" x2="100%" y2="100%">
                            <stop offset="0%" stop-color="#E11D48"/>
                            <stop offset="100%" stop-color="#881337"/>
                        </linearGradient>
                    </defs>
                    <rect x="16" y="16" width="480" height="480" rx="96" fill="url(#logo-bg)" stroke="#FB7185" stroke-width="8"/>
                    <g transform="translate(256, 256) scale(1.18)">
                        <g transform="rotate(-45)">
                            <rect x="-120" y="-40" width="240" height="80" rx="40" fill="#FFF" stroke="#9F1239" stroke-width="8"/>
                            <line x1="0" y1="-40" x2="0" y2="40" stroke="#9F1239" stroke-width="8"/>
                        </g>
                        <g transform="rotate(45)">
                            <rect x="-120" y="-40" width="240" height="80" rx="40" fill="#FF1744" stroke="#FFF" stroke-width="8"/>
                            <line x1="0" y1="-40" x2="0" y2="40" stroke="#FFF" stroke-width="8"/>
                        </g>
                    </g>
                </svg>
                <h1 class="top-bar__title">"AllerX"</h1>
            </div>
            <div class="top-bar__right">
                <div class="top-bar__status">
                    <span
                        class=move || {
                            match state.health.get() {
                                ConnectionHealth::Connected => "top-bar__status-dot",
                                _ => "top-bar__status-dot top-bar__status-dot--disconnected",
                            }
                        }
                    ></span>
                    <span class="top-bar__status-text">
                        {move || match state.health.get() {
                            ConnectionHealth::Connected => "เชื่อมต่อแล้ว".to_string(),
                            ConnectionHealth::Disconnected => "HOSxP ไม่พร้อมใช้งาน".to_string(),
                            ConnectionHealth::Unconfigured => "ยังไม่ได้ตั้งค่า".to_string(),
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
