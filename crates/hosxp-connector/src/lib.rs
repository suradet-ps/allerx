//! The only crate allowed to talk to the HOSxP MySQL database.
//!
//! Strictly SELECT-only, enforced in layers (AGENTS.md §5):
//! 1. DB user grants (DBA-side, out of this crate's hands)
//! 2. `SET SESSION TRANSACTION READ ONLY` on every pooled connection ([`pool`])
//! 3. The SQL guard in [`readonly_guard`] — defense in depth
//! 4. Parameterized queries only, runtime values never interpolated
//!
//! Connection settings are stored encrypted at rest and decrypted only in
//! memory ([`config`]). The master key lives in the OS keychain.

pub mod config;
pub mod error;
pub mod pool;
mod queries;
pub mod readonly_guard;
pub mod repository;

pub use config::{HosxConfig, KeyStore, MasterKeyStore, VaultKeyStore, load_vault};
pub use error::Error;
pub use pool::connect;
pub use repository::HosxRepository;
pub use sqlx::MySqlPool;
