//! The repository contract every database implementation must satisfy.
//!
//! `hosxp-connector` implements this trait against the real HOSxP MySQL
//! database; tests and the frontend use [`MockRepository`](crate::mock::MockRepository).

use allerx_models::{
    ConcurrentMedication, DrugCheckResult, DrugItem, HistoryVerdict, PatientSummary,
};
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
    /// suggestion, otherwise the typed name. The implementation resolves it
    /// to an `icode` first (exact icode, then exact generic name, then exact
    /// trade name), queries OPD and IPD history concurrently, and merges
    /// most-recent-first.
    ///
    /// The verdict contract (ROADMAP Phase 1): an **unresolvable** term
    /// returns [`HistoryVerdict::Unresolved`] with the closest formulary
    /// candidates — never a resolved-but-empty list. Only a resolved drug
    /// may produce the "ไม่พบประวัติ" verdict.
    ///
    /// # Errors
    ///
    /// Returns [`RepositoryError::Query`] when a statement fails on the
    /// server side.
    async fn fetch_drug_history(
        &self,
        hn: &str,
        drug: &str,
    ) -> Result<HistoryVerdict, RepositoryError>;

    /// Batch history lookup for several drugs in one call (ROADMAP Phase 5):
    /// the same single question asked N times in one pass, one result per
    /// submitted term.
    ///
    /// The default implementation checks each term sequentially via
    /// [`fetch_drug_history`]; the real repository overrides it with a
    /// concurrent fan-out. Results carry their own `term` label, so order
    /// is informational, not contractual.
    ///
    /// # Errors
    ///
    /// Returns [`RepositoryError::Query`] when any drug's lookup fails.
    async fn check_drugs(
        &self,
        hn: &str,
        drugs: &[String],
    ) -> Result<Vec<DrugCheckResult>, RepositoryError> {
        let mut results = Vec::with_capacity(drugs.len());
        for drug in drugs {
            let verdict = self.fetch_drug_history(hn, drug).await?;
            results.push(DrugCheckResult {
                term: drug.clone(),
                verdict,
            });
        }
        Ok(results)
    }

    /// Recent concurrent medications for a patient (ROADMAP Phase 5) —
    /// deduped per icode, most recent first, drug category only. Powers the
    /// "ยาที่ได้รับล่าสุด" snapshot in the patient detail view.
    ///
    /// # Errors
    ///
    /// Returns [`RepositoryError::Query`] when the statement fails.
    async fn fetch_concurrent_medications(
        &self,
        hn: &str,
    ) -> Result<Vec<ConcurrentMedication>, RepositoryError>;
}
