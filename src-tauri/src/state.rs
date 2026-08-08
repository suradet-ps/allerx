//! Tauri-managed application state.

use std::path::PathBuf;

use allerx_hosxp_connector::MySqlPool;
use allerx_hosxp_connector::config::CONFIG_FILE_NAME;
use tokio::sync::Mutex;

/// Shared state for the command adapters.
pub struct AppState {
    /// Directory where the encrypted settings file lives (app config dir).
    pub config_dir: PathBuf,
    /// Reusable read-only pool, created by the first successful connection
    /// test. `None` until then (and after reconfiguration).
    pub pool: Mutex<Option<MySqlPool>>,
}

impl AppState {
    pub fn new(config_dir: PathBuf) -> Self {
        Self {
            config_dir,
            pool: Mutex::new(None),
        }
    }

    /// Full path of the encrypted settings file.
    pub fn config_path(&self) -> PathBuf {
        self.config_dir.join(CONFIG_FILE_NAME)
    }
}
