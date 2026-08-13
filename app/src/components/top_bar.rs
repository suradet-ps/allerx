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
                        <linearGradient id="logo-bg" x1="0%" y1="0%" x2="0%" y2="100%">
                            <stop offset="0%" stop-color="#FF5252"/>
                            <stop offset="100%" stop-color="#D32F2F"/>
                        </linearGradient>
                        <linearGradient id="logo-cap" x1="0%" y1="0%" x2="0%" y2="100%">
                            <stop offset="0%" stop-color="#FFFFFF"/>
                            <stop offset="100%" stop-color="#E0E0E0"/>
                        </linearGradient>
                    </defs>
                    <circle cx="256" cy="256" r="236" fill="url(#logo-bg)"/>
                    <circle cx="256" cy="256" r="220" fill="none" stroke="#FFF" stroke-width="12" stroke-opacity="0.3"/>
                    <rect x="206" y="100" width="100" height="32" rx="6" fill="url(#logo-cap)"/>
                    <rect x="216" y="132" width="80" height="16" fill="#BDBDBD"/>
                    <path d="M166 160 C166 148 176 148 186 148 L326 148 C336 148 346 148 346 160 L346 370 C346 392 328 410 306 410 L206 410 C184 410 166 392 166 370Z" fill="#FFF"/>
                    <rect x="186" y="180" width="140" height="200" rx="12" fill="#FFEBEE"/>
                    <path d="M256 205 L310 295 C314 302 309 310 300 310 L212 310 C203 310 198 302 202 295Z" fill="#D32F2F"/>
                    <path d="M256 235 L256 270" stroke="#FFF" stroke-width="8" stroke-linecap="round"/>
                    <circle cx="256" cy="287" r="4.5" fill="#FFF"/>
                    <line x1="216" y1="340" x2="296" y2="340" stroke="#D32F2F" stroke-width="8" stroke-linecap="round"/>
                    <line x1="216" y1="360" x2="276" y2="360" stroke="#B0BEC5" stroke-width="6" stroke-linecap="round"/>
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
