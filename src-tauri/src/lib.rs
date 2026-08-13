//! Tauri 2 shell — thin command adapters only (AGENTS.md §3).
//!
//! No SQL and no business logic live here: everything HOSxP-related goes
//! through `allerx-hosxp-connector`, the only crate allowed to touch MySQL.

mod commands;
mod state;
mod stats;

use state::AppState;
use tauri::Manager;

/// Entry point used by `main.rs` and the mobile entry point.
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let config_dir = app.path().app_config_dir()?;
            let state = AppState::new(config_dir);
            // Live health (ROADMAP Phase 3): warms the pool now, then pings
            // every 30 s for the app's lifetime. Feeds connection_health
            // (the status dot) and the degraded-mode banner.
            let monitor_state = state.clone();
            tauri::async_runtime::spawn(async move {
                commands::run_health_monitor(monitor_state).await;
            });
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::connection_status,
            commands::connection_health,
            commands::configure_connection,
            commands::test_connection,
            commands::search_patients,
            commands::search_drugs,
            commands::fetch_drug_history,
            commands::query_stats,
            commands::clear_query_stats,
        ])
        .run(tauri::generate_context!())
        .expect("invariant: tauri::run() fails only when the platform cannot launch the app window — there is no fallback path for a GUI app")
}
