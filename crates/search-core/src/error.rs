//! Error type for the search core — the contract error every repository
//! implementation (mock or real) must speak.
//!
//! English and dev-facing; the Thai user-facing translation happens only at
//! the Tauri command boundary (`src-tauri/commands.rs`). Messages never
//! carry parameter values (HN, CID, names) — see AGENTS.md §2.

/// Errors surfaced by repository implementations.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum RepositoryError {
    /// The database could not be reached (credentials, network, host down).
    #[error("database connection failed")]
    Connection,
    /// A query was sent but failed on the server side.
    #[error("database query failed: {0}")]
    Query(String),
    /// The read-only guard rejected an outgoing statement — a programming
    /// error (AGENTS.md §5.3), not a user or environment problem.
    #[error("read-only guard rejected a statement")]
    Guard,
}
