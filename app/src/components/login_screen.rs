//! Login gate: verify a HOSxP `officer` account before the workspace opens.
//!
//! Connection setup lives in the settings dialog; this screen handles only
//! the officer credentials. Passwords are verified in the Rust backend
//! against the stored MD5 and are never persisted (`docs/auth.md`).

use leptos::prelude::*;

use crate::api;
use crate::components::icons::{IconArrowRight, IconLock, IconSettings};
use crate::components::top_bar::BrandMark;
use crate::secret::wipe_secret;
use crate::state::AppState;

/// Full-screen login surface used until a HOSxP officer signs in.
#[component]
pub fn LoginScreen(state: AppState) -> impl IntoView {
    let username = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);

    let open_settings = {
        let state = state.clone();
        move |_| state.settings_open.set(true)
    };

    let on_login = {
        let state = state.clone();
        move |ev: web_sys::SubmitEvent| {
            ev.prevent_default();
            if busy.get_untracked() {
                return;
            }
            if !state.configured.get_untracked() {
                error.set(Some("ยังไม่ได้ตั้งค่าการเชื่อมต่อ HOSxP".to_string()));
                return;
            }
            let login_name = username.get();
            if login_name.trim().is_empty() {
                error.set(Some("กรุณากรอกชื่อผู้ใช้".to_string()));
                return;
            }
            let login_password = password.get();
            if login_password.is_empty() {
                error.set(Some("กรุณากรอกรหัสผ่าน".to_string()));
                return;
            }
            busy.set(true);
            error.set(None);
            let state = state.clone();
            leptos::task::spawn_local(async move {
                match api::login(login_name.trim(), &login_password).await {
                    Ok(user) => {
                        wipe_secret(&password);
                        username.set(String::new());
                        state.finish_login(user);
                    }
                    Err(err) => error.set(Some(err.message)),
                }
                busy.set(false);
            });
        }
    };

    view! {
        <div class="login">
            <aside class="login__brand">
                <div class="login__brand-top">
                    <BrandMark class="login__logo" />
                    <div class="login__brand-titles">
                        <span class="login__brand-name">"AllerX"</span>
                        <span class="login__brand-sub">"ระบบตรวจสอบประวัติการใช้ยา"</span>
                    </div>
                </div>

                <div class="login__stage">
                    <h1 class="login__headline">"ตรวจสอบประวัติการใช้ยา"</h1>
                    <p class="login__headline-sub">"ก่อนประเมินการแพ้ยา"</p>
                    <div
                        class="login__art"
                        inner_html=include_str!("../../assets/login-illustration.svg")
                    ></div>
                </div>
            </aside>

            <main class="login__main">
                <div class="login__toolbar">
                    <button class="login__settings" on:click=open_settings>
                        <IconSettings class="icon" />
                        "ตั้งค่า"
                    </button>
                </div>

                <div class="login__card">
                    <header class="login__card-head">
                        <h2 class="login__card-title">"เข้าสู่ระบบ"</h2>
                    </header>

                    {move || {
                        (!state.configured.get())
                            .then(|| {
                                view! {
                                    <div class="login__notice" role="status">
                                        <div>
                                            <b>"ยังไม่ได้ตั้งค่าการเชื่อมต่อ HOSxP"</b>
                                            <small>
                                                "เปิดการตั้งค่าเพื่อเชื่อมต่อ HOSxP ก่อนเข้าสู่ระบบ"
                                            </small>
                                        </div>
                                        <button
                                            class="button-secondary"
                                            on:click=open_settings
                                        >
                                            "ตั้งค่าการเชื่อมต่อ"
                                        </button>
                                    </div>
                                }
                            })
                    }}

                    <form class="login__form" on:submit=on_login>
                        <label class="login__field">
                            <span>"ชื่อผู้ใช้"</span>
                            <input
                                class="form-input"
                                type="text"
                                autocomplete="username"
                                prop:value=move || username.get()
                                on:input=move |ev| username.set(event_target_value(&ev))
                            />
                        </label>
                        <label class="login__field">
                            <span>"รหัสผ่าน"</span>
                            <input
                                class="form-input"
                                type="password"
                                autocomplete="current-password"
                                prop:value=move || password.get()
                                on:input=move |ev| password.set(event_target_value(&ev))
                            />
                        </label>

                        {move || {
                            error
                                .get()
                                .map(|message| {
                                    view! {
                                        <p class="login__error" role="alert">
                                            <IconLock class="icon" />
                                            {message}
                                        </p>
                                    }
                                })
                        }}

                        <div class="login__actions">
                            <button
                                type="submit"
                                class="button-primary login__submit"
                                disabled=move || !state.configured.get() || busy.get()
                            >
                                <IconArrowRight class="icon" />
                                {move || if busy.get() { "กำลังเข้าสู่ระบบ..." } else { "เข้าสู่ระบบ" }}
                            </button>
                        </div>
                    </form>
                </div>
            </main>
        </div>
    }
}

/// Minimal startup state while the connection and session are probed.
#[component]
pub fn StartupSplash() -> impl IntoView {
    view! {
        <div class="startup">
            <BrandMark class="startup__logo" />
            <p>"กำลังเตรียมระบบ..."</p>
        </div>
    }
}
