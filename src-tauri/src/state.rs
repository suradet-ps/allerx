//! Tauri-managed application state.

use std::path::PathBuf;
use std::sync::Arc;

use allerx_hosxp_connector::MySqlPool;
use allerx_hosxp_connector::config::CONFIG_FILE_NAME;
use serde::Serialize;
use tokio::sync::Mutex;

use crate::stats::QueryStats;

/// Live reachability of HOSxP (ROADMAP Phase 3).
///
/// Driven by the startup warm-up, the periodic health monitor, and query
/// outcomes — **never** by "the config file exists", which is what the old
/// status dot meant. This is the value behind the top-bar connection dot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ConnectionHealth {
    /// No stored settings — the settings dialog is the flow.
    Unconfigured,
    /// A ping succeeded recently (or a query succeeded).
    Connected,
    /// A ping failed, or a query could not reach the database.
    Disconnected,
}

/// Shared state for the command adapters.
///
/// `Arc<Mutex<…>>` (not `Mutex<…>` directly): `tokio::sync::Mutex` has no
/// `Clone` impl, and both the startup/monitor tasks and command closures
/// need cheap owned handles into the same state.
#[derive(Clone)]
pub struct AppState {
    /// Directory where the encrypted settings file lives (app config dir).
    pub config_dir: PathBuf,
    /// Reusable read-only pool, created by the first successful connection
    /// test (or by the startup warm-up). `None` until then (and after
    /// reconfiguration).
    pub pool: Arc<Mutex<Option<MySqlPool>>>,
    /// PII-free query timing ring buffer (ROADMAP Phase 2) — command names,
    /// durations and outcomes only, never parameter values.
    pub stats: Arc<Mutex<QueryStats>>,
    /// Live reachability of HOSxP, kept fresh by the health monitor and
    /// every query outcome (ROADMAP Phase 3).
    pub health: Arc<Mutex<ConnectionHealth>>,
}

impl AppState {
    pub fn new(config_dir: PathBuf) -> Self {
        Self {
            config_dir,
            pool: Arc::new(Mutex::new(None)),
            stats: Arc::new(Mutex::new(QueryStats::new())),
            health: Arc::new(Mutex::new(ConnectionHealth::Unconfigured)),
        }
    }

    /// Full path of the encrypted settings file.
    pub fn config_path(&self) -> PathBuf {
        self.config_dir.join(CONFIG_FILE_NAME)
    }
}
