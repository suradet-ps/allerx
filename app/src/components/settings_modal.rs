//! Connection settings dialog (DESIGN.md: elevation level 3, system states).
//!
//! Opened from the top bar (or automatically on first launch when no
//! settings exist). Host/port/database/user/password are sent to the
//! backend, which encrypts them before anything touches disk (AGENTS.md §9).

use leptos::prelude::*;

use crate::api::{ConnectionInput, configure_connection, test_connection};
use crate::components::icons::{IconPlug, IconSave, IconX};
use crate::state::AppState;

/// Default MySQL port, prefilled in the port field.
const DEFAULT_PORT: u16 = 3306;

#[component]
pub fn SettingsModal(state: AppState) -> impl IntoView {
    let host = RwSignal::new(String::new());
    let port = RwSignal::new(DEFAULT_PORT.to_string());
    let database = RwSignal::new(String::new());
    let user = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let message = RwSignal::new(Option::<String>::None);
    let busy = RwSignal::new(false);

    let close = move || {
        message.set(None);
        state.settings_open.set(false);
    };

    // Builds a ConnectionInput from the form fields, validating the port
    // and required fields — shared by test and save so both behave
    // identically.
    let build_input = std::rc::Rc::new(move || -> Result<ConnectionInput, String> {
        let port_value = port
            .get_untracked()
            .trim()
            .parse::<u16>()
            .map_err(|_| "หมายเลขพอร์ตไม่ถูกต้อง".to_string())?;
        let input = ConnectionInput {
            host: host.get_untracked(),
            port: port_value,
            database: database.get_untracked(),
            user: user.get_untracked(),
            password: password.get_untracked(),
        };
        if input.host.trim().is_empty()
            || input.database.trim().is_empty()
            || input.user.trim().is_empty()
        {
            return Err("กรุณากรอก Host, ชื่อฐานข้อมูล และผู้ใช้ให้ครบ".to_string());
        }
        Ok(input)
    });

    let run_test = {
        let build_input = std::rc::Rc::clone(&build_input);
        std::rc::Rc::new(move || {
            if busy.get_untracked() {
                return;
            }
            // Test the values typed in the form, not the stored settings —
            // verification must happen before anything is saved to disk.
            let input = match build_input() {
                Ok(input) => input,
                Err(err_message) => {
                    message.set(Some(err_message));
                    return;
                }
            };
            busy.set(true);
            leptos::task::spawn_local(async move {
                match test_connection(&input).await {
                    Ok(_) => message.set(Some("เชื่อมต่อสำเร็จ".to_string())),
                    Err(error) => message.set(Some(error)),
                }
                busy.set(false);
            });
        })
    };
    let run_save = {
        let build_input = std::rc::Rc::clone(&build_input);
        std::rc::Rc::new(move || {
            let input = match build_input() {
                Ok(input) => input,
                Err(err_message) => {
                    message.set(Some(err_message));
                    return;
                }
            };
            busy.set(true);
            leptos::task::spawn_local(async move {
                match configure_connection(&input).await {
                    Ok(()) => {
                        state.configured.set(true);
                        state.settings_open.set(false);
                    }
                    Err(error) => {
                        message.set(Some(error));
                        busy.set(false);
                    }
                }
            });
        })
    };
    let run_test_click = std::rc::Rc::clone(&run_test);
    let run_save_click = std::rc::Rc::clone(&run_save);

    view! {
        <div
            class="modal-backdrop"
            style:display=move || {
                if state.settings_open.get() {
                    "flex"
                } else {
                    "none"
                }
            }
            on:click=move |_| close()
        >
            <section class="modal" on:click=move |ev| ev.stop_propagation()>
                    <h2 class="modal__title">"ตั้งค่าการเชื่อมต่อ HOSxP"</h2>
                    <p class="modal__status">
                        {move || {
                            if state.configured.get() {
                                "ตั้งค่าแล้ว — ข้อมูลการเชื่อมต่อถูกเข้ารหัสเก็บในเครื่องนี้"
                            } else {
                                "ยังไม่ได้ตั้งค่า — ต้องตั้งค่าก่อนใช้งาน"
                            }
                        }}
                    </p>

                    <div class="form-field">
                        <label for="cfg-host">"ที่อยู่เครื่อง (IP / ชื่อเครื่อง)"</label>
                        <input
                            id="cfg-host"
                            class="form-input"
                            placeholder="192.168.1.10"
                            prop:value=move || host.get()
                            on:input=move |ev| host.set(event_target_value(&ev))
                        />
                    </div>
                    <div class="form-row">
                        <div class="form-field">
                            <label for="cfg-port">"Port"</label>
                            <input
                                id="cfg-port"
                                class="form-input"
                                prop:value=move || port.get()
                                on:input=move |ev| port.set(event_target_value(&ev))
                            />
                        </div>
                        <div class="form-field form-field--grow">
                            <label for="cfg-database">"ชื่อฐานข้อมูล"</label>
                            <input
                                id="cfg-database"
                                class="form-input"
                                placeholder="hosxp"
                                prop:value=move || database.get()
                                on:input=move |ev| database.set(event_target_value(&ev))
                            />
                        </div>
                    </div>
                    <div class="form-field">
                        <label for="cfg-user">"ผู้ใช้ฐานข้อมูล"</label>
                        <input
                            id="cfg-user"
                            class="form-input"
                            placeholder="allerx_ro"
                            prop:value=move || user.get()
                            on:input=move |ev| user.set(event_target_value(&ev))
                        />
                    </div>
                    <div class="form-field">
                        <label for="cfg-password">"รหัสผ่าน"</label>
                        <input
                            id="cfg-password"
                            class="form-input form-input--mono"
                            type="password"
                            prop:value=move || password.get()
                            on:input=move |ev| password.set(event_target_value(&ev))
                        />
                    </div>

                    {move || {
                        message.get().map(|text| {
                            view! { <p class="modal__message">{text}</p> }
                        })
                    }}

                    <p class="modal__note">
                        "ข้อมูลถูกเข้ารหัสเก็บในเครื่องเท่านั้น — แอปอ่านฐานข้อมูลได้อย่างเดียว"
                    </p>

                    <div class="modal__actions">
                        <button
                            class="button-secondary"
                            on:click=move |_| run_test_click()
                            prop:disabled=move || busy.get()
                        >
                            <IconPlug class="icon" />
                            "ทดสอบการเชื่อมต่อ"
                        </button>
                        <button class="button-secondary" on:click=move |_| close()>
                            <IconX class="icon" />
                            "ปิด"
                        </button>
                        <button
                            class="button-primary"
                            on:click=move |_| run_save_click()
                            prop:disabled=move || busy.get()
                        >
                            <IconSave class="icon" />
                            "บันทึก"
                        </button>
                    </div>
                </section>
            </div>
    }
}
