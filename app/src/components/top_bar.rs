//! Top bar — thin 44px header spanning full width.
//! Shows app name, signed-in officer, connection status, settings, and logout.

use leptos::prelude::*;

use crate::api;
use crate::components::icons::{IconLogOut, IconSettings};
use crate::state::{AppState, ConnectionHealth};

#[component]
pub fn TopBar(state: AppState) -> impl IntoView {
    let logout = {
        let state = state.clone();
        move |_| {
            let state = state.clone();
            leptos::task::spawn_local(async move {
                let _ = api::logout().await;
                state.clear_session();
            });
        }
    };

    view! {
        <header class="top-bar">
            <div class="top-bar__left">
                <BrandMark class="top-bar__logo" />
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
                        {move || state.health.get().label_th()}
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
                {move || {
                    state
                        .session
                        .get()
                        .map(|user| {
                            let initial = user
                                .display_name
                                .chars()
                                .next()
                                .or_else(|| user.login_name.chars().next())
                                .map(|c| c.to_string())
                                .unwrap_or_default();
                            let title = format!("{} · {}", user.display_name, user.login_name);
                            view! {
                                <span class="user-chip" title=title>
                                    <span class="user-chip__avatar">{initial}</span>
                                    <span class="user-chip__name">{user.display_name}</span>
                                </span>
                            }
                        })
                }}
                <button
                    class="top-bar__button top-bar__logout"
                    on:click=logout
                    data-tooltip="ออกจากระบบ"
                >
                    <IconLogOut class="icon" />
                    "ออกจากระบบ"
                </button>
            </div>
        </header>
    }
}

/// The app mark: crossed capsules on the ruby tile, matching `icon-master.svg`.
/// `class` controls the rendered size from the consuming layout.
#[component]
pub fn BrandMark(#[prop(default = "top-bar__logo")] class: &'static str) -> impl IntoView {
    view! {
        <svg class=class viewBox="0 0 512 512" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
            <defs>
                <linearGradient id="logo-bg" x1="0%" y1="0%" x2="100%" y2="100%">
                    <stop offset="0%" stop-color="#E11D48"/>
                    <stop offset="100%" stop-color="#881337"/>
                </linearGradient>
                <filter id="logo-inner-glow" x="-20%" y="-20%" width="140%" height="140%">
                    <feDropShadow dx="0" dy="8" stdDeviation="10" flood-color="#000000" flood-opacity="0.3"/>
                </filter>
            </defs>
            <rect x="16" y="16" width="480" height="480" rx="96" fill="url(#logo-bg)" stroke="#FB7185" stroke-width="8"/>
            <g transform="translate(256, 256) scale(1.48)" filter="url(#logo-inner-glow)">
                <g transform="rotate(-45)">
                    <rect x="-120" y="-40" width="240" height="80" rx="40" fill="#FFF" stroke="#9F1239" stroke-width="6"/>
                    <line x1="0" y1="-40" x2="0" y2="40" stroke="#9F1239" stroke-width="5"/>
                </g>
                <g transform="rotate(45)">
                    <rect x="-120" y="-40" width="240" height="80" rx="40" fill="#FF1744" stroke="#FFF" stroke-width="6"/>
                    <line x1="0" y1="-40" x2="0" y2="40" stroke="#FFF" stroke-width="5"/>
                </g>
            </g>
        </svg>
    }
}
