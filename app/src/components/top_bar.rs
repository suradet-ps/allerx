//! Top bar (DESIGN.md: brand-teal chrome strip, app name).

use leptos::prelude::*;

use crate::components::icons::IconSettings;
use crate::state::AppState;

/// Brand-teal header — chrome only, never carries verdict colors. The
/// settings button lives here as the only chrome action.
#[component]
pub fn TopBar(state: AppState) -> impl IntoView {
    view! {
        <header class="top-bar">
            <div class="top-bar__inner">
                <div class="top-bar__title-block">
                    <h1 class="top-bar__title">"AllerX"</h1>
                    <span class="top-bar__tagline">
                        "ตรวจประวัติการได้รับยาก่อนการประเมินแพ้ยา"
                    </span>
                </div>
                <div class="top-bar__actions">
                    <button
                        class="top-bar__button"
                        on:click=move |_| state.settings_open.set(true)
                    >
                        <IconSettings class="icon" />
                        "ตั้งค่า"
                    </button>
                </div>
            </div>
        </header>
    }
}
