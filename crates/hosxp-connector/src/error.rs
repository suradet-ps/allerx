//! Unified error type for hosxp-connector.
//!
//! English and dev-facing. The Thai user-facing translation happens only at
//! the Tauri command boundary (`src-tauri/commands.rs`); this crate never
//! formats messages for the UI.

/// Errors surfaced by the connector crate.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Connection settings could not be loaded, decrypted, or stored.
    #[error("connection settings error: {0}")]
    Config(#[from] crate::config::ConfigError),
    /// The pool could not open — network, credentials, or server down.
    ///
    /// Kept distinct from [`Error::Database`] so the command layer can tell
    /// "cannot reach HOSxP" from "a statement failed". The inner error
    /// never contains credentials or parameter values (AGENTS.md §2).
    #[error("failed to connect to the HOSxP database: {0}")]
    Connect(sqlx::Error),
    /// A statement failed on the server side.
    #[error("database query failed: {0}")]
    Database(#[from] sqlx::Error),
    /// The read-only guard rejected an outgoing statement (defense in
    /// depth, AGENTS.md §5.3).
    #[error("read-only guard rejected a statement")]
    Guard,
}

/// Collapses connector errors onto the search-core contract error.
///
/// This is the **only** conversion point between the two layers: internals
/// of this crate return [`Error`] and lose nothing; the trait boundary
/// maps once, deliberately dropping details the contract does not need.
impl From<Error> for allerx_search_core::RepositoryError {
    fn from(err: Error) -> Self {
        match err {
            Error::Config(inner) => Self::Query(inner.to_string()),
            Error::Connect(_) => Self::Connection,
            Error::Database(inner) => Self::Query(inner.to_string()),
            Error::Guard => Self::Guard,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ConfigError;
    use allerx_search_core::RepositoryError;

    #[test]
    fn maps_connect_to_connection() {
        assert_eq!(
            RepositoryError::from(Error::Connect(sqlx::Error::PoolTimedOut)),
            RepositoryError::Connection
        );
    }

    #[test]
    fn maps_database_to_query_preserving_detail() {
        // The Query payload carries the sqlx message so devs can diagnose
        // server-side failures; the exact wording is sqlx's, not ours.
        let mapped = RepositoryError::from(Error::Database(sqlx::Error::RowNotFound));
        match mapped {
            RepositoryError::Query(detail) => assert!(detail.contains("no rows returned")),
            other => panic!("expected Query variant, got {other:?}"),
        }
    }

    #[test]
    fn maps_guard_and_config_to_typed_variants() {
        assert_eq!(RepositoryError::from(Error::Guard), RepositoryError::Guard);
        let mapped =
            RepositoryError::from(Error::Config(ConfigError::Keyring("unavailable".into())));
        assert!(matches!(mapped, RepositoryError::Query(_)));
    }
}
