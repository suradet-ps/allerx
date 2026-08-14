//! Connection settings dialog (DESIGN.md: elevation level 3, system states).
//!
//! Opened from the top bar (or automatically on first launch when no
//! settings exist). Host/port/database/user/password are sent to the
//! backend, which encrypts them before anything touches disk (AGENTS.md §9).

use std::rc::Rc;

use leptos::ev;
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
    // The last form operation's outcome: `Some((is_success, text))`. The
    // flag drives the message styling — never sniff the message text for a
    // success marker (Thai messages contain "ได้" in both outcomes, e.g.
    // "ไม่สามารถเข้าถึงที่เก็บกุญแจของระบบได้").
    let message = RwSignal::new(None::<(bool, String)>);
    let busy = RwSignal::new(false);

    /// Zeroizes an operator-typed field before dropping it — plain
    /// `String::new()` only frees the buffer, leaving the plaintext bytes behind
    /// for a memory scan. NUL is valid UTF-8, so the fill keeps the String
    /// well-formed while it is still alive inside this function.
    fn wipe_field(signal: &RwSignal<String>) {
        let mut value = signal.get_untracked();
        if !value.is_empty() {
            // SAFETY: the buffer is owned exclusively by `value`; filling it with
            // NUL bytes (valid UTF-8) keeps every invariant intact, and the
            // buffer is dropped unaliased at the end of this function.
            unsafe { value.as_mut_vec().fill(0) };
        }
        signal.set(String::new());
    }

    // Clears every field the operator typed — the modal stays mounted (hidden
    // by CSS), so without this the plaintext would linger in WASM memory for
    // the whole app session (AGENTS.md §9).
    let wipe_fields = move || {
        wipe_field(&host);
        wipe_field(&port);
        wipe_field(&database);
        wipe_field(&user);
        wipe_field(&password);
    };

    let close = Rc::new(move || {
        wipe_fields();
        message.set(None);
        state.settings_open.set(false);
    });

    // Escape closes the dialog from anywhere (DESIGN.md "Focus Management").
    // The listener sits on `window`, so focus location does not matter; the
    // open-flag guard ensures an Escape pressed elsewhere (e.g. clearing a
    // search input) never reaches `close()`, which zeroizes the typed
    // fields. The handle must stay alive for the component's lifetime —
    // dropping it unregisters the listener.
    let escape_state = state.clone();
    let close_on_escape = Rc::clone(&close);
    let escape_handle = window_event_listener(ev::keydown, move |event| {
        if event.key() == "Escape" && escape_state.settings_open.get_untracked() {
            close_on_escape();
        }
    });
    let _escape_handle = StoredValue::new(escape_handle);

    // Clones for the view's on:click closures (each captures its own).
    let backdrop_close = Rc::clone(&close);
    let button_close = Rc::clone(&close);

    // Builds a ConnectionInput from the form fields, validating the port
    // and required fields — shared by test and save so both behave
    // identically.
    let build_input = std::rc::Rc::new(move || -> Result<ConnectionInput, String> {
        let port_value = port
            .get_untracked()
            .trim()
            .parse::<u16>()
            .map_err(|_| "พอร์ตไม่ถูกต้อง".to_string())?;
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
            return Err("กรอก Host, Database, User ให้ครบ".to_string());
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
                    message.set(Some((false, err_message)));
                    return;
                }
            };
            busy.set(true);
            leptos::task::spawn_local(async move {
                match test_connection(&input).await {
                    Ok(_) => message.set(Some((true, "เชื่อมต่อได้".to_string()))),
                    Err(error) => message.set(Some((false, error.message))),
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
                    message.set(Some((false, err_message)));
                    return;
                }
            };
            busy.set(true);
            leptos::task::spawn_local(async move {
                match configure_connection(&input).await {
                    Ok(()) => {
                        wipe_fields();
                        state.configured.set(true);
                        state.settings_open.set(false);
                    }
                    Err(error) => {
                        message.set(Some((false, error.message)));
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
            on:click=move |_| backdrop_close()
        >
            <section class="modal" on:click=move |ev| ev.stop_propagation()>
                    <h2 class="modal__title">"ตั้งค่า HOSxP"</h2>
                    <p class="modal__status">
                        {move || {
                            if state.configured.get() {
                                "เชื่อมต่อแล้ว — เข้ารหัสเก็บในเครื่อง"
                            } else {
                                "ยังไม่ได้ตั้งค่า"
                            }
                        }}
                    </p>

                    <div class="form-field">
                        <label for="cfg-host">"Host"</label>
                        <input
                            id="cfg-host"
                            class="form-input"
                            placeholder="192.168.1.10"
                            prop:value=move || host.get()
                            on:input=move |ev| host.set(event_target_value(&ev))
                        />
                    </div>
                    <div class="form-row">
                        <div class="form-field" style="max-width: 100px;">
                            <label for="cfg-port">"Port"</label>
                            <input
                                id="cfg-port"
                                class="form-input"
                                prop:value=move || port.get()
                                on:input=move |ev| port.set(event_target_value(&ev))
                            />
                        </div>
                        <div class="form-field form-field--grow">
                            <label for="cfg-database">"Database"</label>
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
                        <label for="cfg-user">"User"</label>
                        <input
                            id="cfg-user"
                            class="form-input"
                            placeholder="allerx_ro"
                            prop:value=move || user.get()
                            on:input=move |ev| user.set(event_target_value(&ev))
                        />
                    </div>
                    <div class="form-field">
                        <label for="cfg-password">"Password"</label>
                        <input
                            id="cfg-password"
                            class="form-input form-input--mono"
                            type="password"
                            prop:value=move || password.get()
                            on:input=move |ev| password.set(event_target_value(&ev))
                        />
                    </div>

                    {move || {
                        message.get().map(|(is_success, text)| {
                            let class = if is_success {
                                "modal__message modal__message--success"
                            } else {
                                "modal__message modal__message--error"
                            };
                            view! { <p class=class>{text}</p> }
                        })
                    }}

                    <div class="modal__actions">
                        <button
                            class="button-secondary"
                            on:click=move |_| run_test_click()
                            prop:disabled=move || busy.get()
                        >
                            <IconPlug class="icon" />
                            "ทดสอบ"
                        </button>
                        <button class="button-secondary" on:click=move |_| button_close()>
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
