//! The repository contract every database implementation must satisfy.
//!
//! `hosxp-connector` implements this trait against the real HOSxP MySQL
//! database; tests and the frontend use [`MockRepository`](crate::mock::MockRepository).

use allerx_models::{DrugHistoryRecord, DrugItem, PatientSummary};
use async_trait::async_trait;

use crate::error::RepositoryError;
use crate::query_kind::QueryKind;

/// Read-only access to HOSxP. Extended per milestone (M2 patient search,
/// M3 drug autocomplete, M4 medication history).
#[async_trait]
pub trait HosxRepository: Send + Sync {
    /// Smoke test — verifies the connection works (`SELECT 1`).
    ///
    /// # Errors
    ///
    /// Returns [`RepositoryError::Connection`] when the pool cannot reach
    /// the database, or [`RepositoryError::Query`] when the statement fails.
    async fn ping(&self) -> Result<(), RepositoryError>;

    /// Searches patients by HN, CID, or name (AGENTS.md §7.1).
    ///
    /// The caller classifies the input with [`QueryKind`] first — the
    /// implementation runs the matching lookup (exact HN/CID match, or a
    /// name prefix match with a contains-match fallback) and returns at
    /// most 20 results.
    ///
    /// # Errors
    ///
    /// Returns [`RepositoryError::Query`] when the statement fails on the
    /// server side.
    async fn search_patients(
        &self,
        term: &str,
        kind: QueryKind,
    ) -> Result<Vec<PatientSummary>, RepositoryError>;

    /// Drug-name autocomplete from `drugitems` (AGENTS.md §7.2).
    ///
    /// Prefix match first (index-friendly), contains-match fallback when the
    /// prefix finds nothing. Returns at most 20 items.
    ///
    /// # Errors
    ///
    /// Returns [`RepositoryError::Query`] when the statement fails on the
    /// server side.
    async fn search_drugs(&self, term: &str) -> Result<Vec<DrugItem>, RepositoryError>;

    /// Full medication history for one patient + drug (AGENTS.md §7.2, M4).
    ///
    /// `drug` is the drug `icode` when the operator picked an autocomplete
    /// suggestion, otherwise the typed name — implementations resolve it to
    /// an `icode` first (exact icode, then exact name). OPD and IPD history
    /// are queried concurrently, merged, and returned most-recent-first.
    /// An unresolvable drug yields an empty list, never an error.
    ///
    /// # Errors
    ///
    /// Returns [`RepositoryError::Query`] when a statement fails on the
    /// server side.
    async fn fetch_drug_history(
        &self,
        hn: &str,
        drug: &str,
    ) -> Result<Vec<DrugHistoryRecord>, RepositoryError>;
}
