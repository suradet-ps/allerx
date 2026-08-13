//! Production [`HosxRepository`] implementation over the read-only pool.

use crate::error::Error;
use crate::queries::{
    CONCURRENT_MEDS, CONCURRENT_MEDS_TRADE, DRUG_RESOLVE_BY_ICODE, DRUG_RESOLVE_BY_ICODE_TRADE,
    DRUG_RESOLVE_BY_NAME, DRUG_RESOLVE_BY_NAME_TRADE, DRUG_RESOLVE_BY_TRADE_NAME,
    DRUG_SEARCH_CONTAINS_PLAIN, DRUG_SEARCH_CONTAINS_TRADE, DRUG_SEARCH_CONTAINS_TYPED,
    DRUG_SEARCH_PREFIX_PLAIN, DRUG_SEARCH_PREFIX_TRADE, DRUG_SEARCH_PREFIX_TYPED, HISTORY_IPD_STAY,
    HISTORY_IPD_STAY_TRADE, HISTORY_IPD_TAKEHOME, HISTORY_IPD_TAKEHOME_FALLBACK, HISTORY_OPD,
    HISTORY_OPD_FALLBACK, PATIENT_SEARCH_BY_CID, PATIENT_SEARCH_BY_HN,
    PATIENT_SEARCH_NAME_CONTAINS, PATIENT_SEARCH_NAME_PREFIX,
};
use crate::readonly_guard::{PING_SQL, assert_read_only};
use allerx_models::{
    ConcurrentMedication, DrugCheckResult, DrugHistoryRecord, DrugItem, HistoryVerdict,
    PatientSummary, VisitType,
};
use allerx_search_core::{
    DrugResolution, HosxRepository as HosxRepositoryTrait, QueryKind, RepositoryError,
    classify_drug_resolution, merge_drug_history, rank_candidates, verdict_from_resolution,
};
use async_trait::async_trait;
use chrono::NaiveDate;
use sqlx::MySqlPool;

/// Per-source row cap — must stay in sync with the `LIMIT` inside every
/// `HISTORY_*` statement in `queries.rs` (enforced by a test below). At the
/// cap, older history exists but is not returned and the UI must say so.
const HISTORY_LIMIT: usize = 200;

/// Maximum candidates offered when a drug term does not resolve exactly.
const RESOLUTION_CANDIDATE_LIMIT: usize = 10;

/// One `patient` row as `(hn, cid, full_name_th, birth_date, sex)`,
/// matching the SELECT column order in `queries.rs`.
type PatientRow = (
    String,
    Option<String>,
    Option<String>,
    Option<NaiveDate>,
    Option<String>,
);

/// One history row as `(visit_date, drug_code, drug_name, strength,
/// trade_name, prescriber, department, route, quantity)` — same shape for
/// OPD and both IPD sources, including the fallback (`NULL` in the
/// optional columns' places).
type HistoryRow = (
    NaiveDate,
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);

/// One drug row as `(icode, name, strength, trade_name)` — again shared by
/// the typed/trade/plain autocomplete tiers.
type DrugRow = (String, String, Option<String>, Option<String>);

/// The repository that talks to the real HOSxP database. Created from a
/// [`connect`](crate::connect) pool — this is the only object that ever
/// executes SQL in the app.
#[derive(Debug, Clone)]
pub struct HosxRepository {
    pool: MySqlPool,
}

impl HosxRepository {
    /// Wraps a read-only pool.
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    /// Runs one patient-search statement with the given bound parameters.
    ///
    /// `params` are bound in order; LIKE patterns (not SQL text) are built
    /// by the caller.
    async fn fetch_patients(
        &self,
        sql: &str,
        params: &[&str],
    ) -> Result<Vec<PatientSummary>, Error> {
        let rows: Vec<PatientRow> = self.guarded_fetch(sql, params).await?;
        Ok(rows.into_iter().map(patient_from_row).collect())
    }

    /// Runs one statement with the given bound parameters, applying the
    /// read-only guard. Guard rejections surface as [`Error::Guard`],
    /// server-side failures as [`Error::Database`].
    async fn guarded_fetch<T>(&self, sql: &str, params: &[&str]) -> Result<Vec<T>, Error>
    where
        T: for<'r> sqlx::FromRow<'r, sqlx::mysql::MySqlRow> + Send + Unpin,
    {
        assert_read_only(sql).map_err(|_| Error::Guard)?;
        self.raw_fetch(sql, params).await.map_err(Error::from)
    }

    /// Runs one statement with the given bound parameters, returning raw
    /// sqlx errors so callers can apply error-policy decisions.
    async fn raw_fetch<T>(&self, sql: &str, params: &[&str]) -> Result<Vec<T>, sqlx::Error>
    where
        T: for<'r> sqlx::FromRow<'r, sqlx::mysql::MySqlRow> + Send + Unpin,
    {
        let mut query = sqlx::query_as::<_, T>(sql);
        for param in params {
            query = query.bind(param);
        }
        query.fetch_all(&self.pool).await
    }

    /// Runs the first statement that succeeds, in order.
    ///
    /// Statements failing with a missing-table/column error (MySQL 1146/1054
    /// — a documented per-instance variation, AGENTS.md §6) are skipped for
    /// the next tier, so a richer query (trade names, drug-type filter) can
    /// degrade to the safe baseline instead of breaking the app. Any other
    /// failure propagates immediately. Every statement must pass the
    /// read-only guard.
    async fn fetch_first_working<T>(&self, candidates: &[(&str, &[&str])]) -> Result<Vec<T>, Error>
    where
        T: for<'r> sqlx::FromRow<'r, sqlx::mysql::MySqlRow> + Send + Unpin,
    {
        let mut last_schema_error: Option<sqlx::Error> = None;
        for (sql, params) in candidates {
            assert_read_only(sql).map_err(|_| Error::Guard)?;
            match self.raw_fetch(sql, params).await {
                Ok(rows) => return Ok(rows),
                Err(err) if is_schema_variation(&err) => last_schema_error = Some(err),
                Err(err) => return Err(Error::Database(err)),
            }
        }
        // Invariant: every call site passes at least one statement that the
        // instance must support (the plain tier), so exhausting the list
        // means even the baseline failed with a schema error.
        Err(Error::Database(last_schema_error.expect(
            "invariant: at least one candidate statement is provided",
        )))
    }

    /// Resolves a drug term to its full `drugitems` row (icode, name,
    /// strength, trade name): exact icode, then exact display name, then
    /// exact trade name. `None` means the term is not in `drugitems` under
    /// any of the three — the caller then falls back to candidate
    /// suggestions (never a "not found" verdict, ROADMAP Phase 1).
    ///
    /// The returned identity labels the verdict — a pharmacist searching by
    /// icode sees the drug's name and strength, not a bare code.
    async fn resolve_drug_item(&self, drug: &str) -> Result<Option<DrugItem>, Error> {
        if let Some(item) = self
            .fetch_single_drug_item_tiered(DRUG_RESOLVE_BY_ICODE_TRADE, DRUG_RESOLVE_BY_ICODE, drug)
            .await?
        {
            return Ok(Some(item));
        }
        if let Some(item) = self
            .fetch_single_drug_item_tiered(DRUG_RESOLVE_BY_NAME_TRADE, DRUG_RESOLVE_BY_NAME, drug)
            .await?
        {
            return Ok(Some(item));
        }
        // A missing trade-name column is a documented schema variation —
        // "no trade-name match", not an error.
        self.fetch_single_drug_item_tolerant(DRUG_RESOLVE_BY_TRADE_NAME, drug)
            .await
    }

    /// Fetches one drug row, degrading the trade-name column to the
    /// fallback statement when the instance lacks it (MySQL 1054).
    async fn fetch_single_drug_item_tiered(
        &self,
        sql: &str,
        fallback_sql: &str,
        param: &str,
    ) -> Result<Option<DrugItem>, Error> {
        assert_read_only(sql).map_err(|_| Error::Guard)?;
        assert_read_only(fallback_sql).map_err(|_| Error::Guard)?;
        match self.fetch_single_drug_row(sql, param).await {
            Ok(row) => Ok(row),
            Err(Error::Database(ref err)) if is_schema_variation(err) => {
                self.fetch_single_drug_row(fallback_sql, param).await
            }
            Err(err) => Err(err),
        }
    }

    /// Like [`fetch_single_drug_item_tiered`], but a missing column/table
    /// (MySQL 1146/1054) yields `Ok(None)` instead of an error.
    async fn fetch_single_drug_item_tolerant(
        &self,
        sql: &str,
        param: &str,
    ) -> Result<Option<DrugItem>, Error> {
        assert_read_only(sql).map_err(|_| Error::Guard)?;
        match self.fetch_single_drug_row(sql, param).await {
            Ok(row) => Ok(row),
            Err(Error::Database(ref err)) if is_schema_variation(err) => Ok(None),
            Err(err) => Err(err),
        }
    }

    async fn fetch_single_drug_row(
        &self,
        sql: &str,
        param: &str,
    ) -> Result<Option<DrugItem>, Error> {
        sqlx::query_as::<_, DrugRow>(sql)
            .bind(param)
            .fetch_optional(&self.pool)
            .await
            .map(|row| row.map(drug_from_row))
            .map_err(Error::from)
    }

    /// OPD history — `opitemrece` rows without an admission number. Returns
    /// the rows and whether the source hit its cap (older rows exist).
    async fn fetch_opd_history(
        &self,
        hn: &str,
        icode: &str,
    ) -> Result<(Vec<DrugHistoryRecord>, bool), Error> {
        let rows: Vec<HistoryRow> = self
            .fetch_first_working(&[
                (HISTORY_OPD, &[hn, icode]),
                (HISTORY_OPD_FALLBACK, &[hn, icode]),
            ])
            .await?;
        let truncated = rows.len() == HISTORY_LIMIT;
        Ok((
            rows.into_iter()
                .map(|r| history_record(r, VisitType::Opd))
                .collect(),
            truncated,
        ))
    }

    /// IPD history — take-home rows in `opitemrece` plus in-stay rows in
    /// `iptitemrece` (AGENTS.md §6.3), run concurrently.
    async fn fetch_ipd_history(
        &self,
        hn: &str,
        icode: &str,
    ) -> Result<(Vec<DrugHistoryRecord>, bool), Error> {
        let (takehome, stay) = tokio::join!(
            self.fetch_ipd_takehome_history(hn, icode),
            self.fetch_ipd_stay_history(hn, icode),
        );
        let (mut records, takehome_truncated) = takehome?;
        let (stay_records, stay_truncated) = stay?;
        records.extend(stay_records);
        Ok((records, takehome_truncated || stay_truncated))
    }

    async fn fetch_ipd_takehome_history(
        &self,
        hn: &str,
        icode: &str,
    ) -> Result<(Vec<DrugHistoryRecord>, bool), Error> {
        let rows: Vec<HistoryRow> = self
            .fetch_first_working(&[
                (HISTORY_IPD_TAKEHOME, &[hn, icode]),
                (HISTORY_IPD_TAKEHOME_FALLBACK, &[hn, icode]),
            ])
            .await?;
        let truncated = rows.len() == HISTORY_LIMIT;
        Ok((
            rows.into_iter()
                .map(|r| history_record(r, VisitType::Ipd))
                .collect(),
            truncated,
        ))
    }

    /// In-stay IPD dispensing. A missing `iptitemrece` table (or a hospital
    /// that names it differently, AGENTS.md §6.3) is a documented schema
    /// variation — it yields no records instead of failing the whole lookup.
    async fn fetch_ipd_stay_history(
        &self,
        hn: &str,
        icode: &str,
    ) -> Result<(Vec<DrugHistoryRecord>, bool), Error> {
        let rows = match self
            .fetch_first_working(&[
                (HISTORY_IPD_STAY_TRADE, &[hn, icode]),
                (HISTORY_IPD_STAY, &[hn, icode]),
            ])
            .await
        {
            Ok(rows) => rows,
            // Even the plain tier failed with a schema error — the
            // `iptitemrece` table itself is absent on this instance.
            Err(Error::Database(ref err)) if is_schema_variation(err) => Vec::new(),
            Err(err) => return Err(err),
        };
        let truncated = rows.len() == HISTORY_LIMIT;
        Ok((
            rows.into_iter()
                .map(|r| history_record(r, VisitType::Ipd))
                .collect(),
            truncated,
        ))
    }

    /// Full merged timeline (OPD + IPD take-home + IPD in-stay), most recent
    /// first, plus whether any source hit its cap.
    async fn fetch_all_history(
        &self,
        hn: &str,
        icode: &str,
    ) -> Result<(Vec<DrugHistoryRecord>, bool), Error> {
        let (opd, ipd) = tokio::join!(
            self.fetch_opd_history(hn, icode),
            self.fetch_ipd_history(hn, icode),
        );
        let (opd_records, opd_truncated) = opd?;
        let (ipd_records, ipd_truncated) = ipd?;
        let records = merge_drug_history(opd_records, ipd_records);
        Ok((records, opd_truncated || ipd_truncated))
    }
}

/// True when the query failed because a table/column does not exist on this
/// instance — a documented per-hospital variation (AGENTS.md §6.3), not a
/// real failure.
///
/// Traps for the unwary: the generic `DatabaseError::code()` is the
/// **SQLSTATE** ("42S02"/"42S22"), not the MySQL error number — the numeric
/// codes (1146/1054) live on the MySQL-specific `number()`, so the error
/// must be downcast first.
fn is_schema_variation(err: &sqlx::Error) -> bool {
    err.as_database_error()
        .and_then(|db_err| db_err.try_downcast_ref::<sqlx::mysql::MySqlDatabaseError>())
        .is_some_and(|mysql_err| matches!(mysql_err.number(), 1146 | 1054))
}

fn patient_from_row((hn, cid, full_name_th, birth_date, sex): PatientRow) -> PatientSummary {
    PatientSummary {
        hn,
        cid,
        full_name_th: full_name_th.unwrap_or_default(),
        birth_date,
        sex,
    }
}

fn history_record(
    (
        visit_date,
        drug_code,
        drug_name,
        strength,
        trade_name,
        prescriber,
        department,
        route,
        quantity,
    ): HistoryRow,
    visit_type: VisitType,
) -> DrugHistoryRecord {
    DrugHistoryRecord {
        visit_date,
        visit_type,
        drug_code,
        drug_name,
        strength,
        trade_name,
        prescriber,
        department,
        quantity,
        route,
    }
}

#[async_trait]
impl HosxRepositoryTrait for HosxRepository {
    async fn ping(&self) -> Result<(), RepositoryError> {
        assert_read_only(PING_SQL).map_err(|_| Error::Guard)?;
        sqlx::query_as::<_, (i32,)>(PING_SQL)
            .fetch_one(&self.pool)
            .await
            .map(|_| ())
            .map_err(Error::from)?;
        Ok(())
    }

    async fn search_patients(
        &self,
        term: &str,
        kind: QueryKind,
    ) -> Result<Vec<PatientSummary>, RepositoryError> {
        let term = term.trim();
        let hits = match kind {
            QueryKind::Cid => self.fetch_patients(PATIENT_SEARCH_BY_CID, &[term]).await?,
            QueryKind::Hn => self.fetch_patients(PATIENT_SEARCH_BY_HN, &[term]).await?,
            QueryKind::Name => {
                // Prefix-match first to use indexes; only fall back to a
                // contains-match when the prefix found nothing (AGENTS.md §7.1).
                let prefix = format!("{term}%");
                let mut hits = self
                    .fetch_patients(PATIENT_SEARCH_NAME_PREFIX, &[&prefix, &prefix, &prefix])
                    .await?;
                if hits.is_empty() {
                    let contains = format!("%{term}%");
                    hits = self
                        .fetch_patients(
                            PATIENT_SEARCH_NAME_CONTAINS,
                            &[&contains, &contains, &contains],
                        )
                        .await?;
                }
                hits
            }
        };
        Ok(hits)
    }

    async fn search_drugs(&self, term: &str) -> Result<Vec<DrugItem>, RepositoryError> {
        let term = term.trim();
        if term.is_empty() {
            return Ok(Vec::new());
        }
        let prefix = format!("{term}%");
        let mut hits: Vec<DrugItem> = self
            .fetch_first_working(&[
                // Typed tier: name/trade-name/icode matching + drug-type filter.
                (DRUG_SEARCH_PREFIX_TYPED, &[&prefix, &prefix, &prefix]),
                // Trade tier: name/icode matching, no type filter.
                (DRUG_SEARCH_PREFIX_TRADE, &[&prefix, &prefix]),
                // Plain tier: every instance supports this.
                (DRUG_SEARCH_PREFIX_PLAIN, &[&prefix, &prefix]),
            ])
            .await?
            .into_iter()
            .map(drug_from_row)
            .collect();
        if hits.is_empty() {
            let contains = format!("%{term}%");
            hits = self
                .fetch_first_working(&[
                    (
                        DRUG_SEARCH_CONTAINS_TYPED,
                        &[&contains, &contains, &contains],
                    ),
                    (DRUG_SEARCH_CONTAINS_TRADE, &[&contains, &contains]),
                    (DRUG_SEARCH_CONTAINS_PLAIN, &[&contains, &contains]),
                ])
                .await?
                .into_iter()
                .map(drug_from_row)
                .collect();
        }
        Ok(hits)
    }

    async fn fetch_drug_history(
        &self,
        hn: &str,
        drug: &str,
    ) -> Result<HistoryVerdict, RepositoryError> {
        let hn = hn.trim();
        let drug = drug.trim();
        if hn.is_empty() || drug.is_empty() {
            // Blank input is an unresolvable term — never a "not found"
            // verdict (ROADMAP Phase 1 invariant).
            return Ok(HistoryVerdict::Unresolved {
                candidates: Vec::new(),
            });
        }
        // OPD + IPD run concurrently, then merge most-recent-first
        // (AGENTS.md §7.2). The resolution flow (ROADMAP Phase 1): an exact
        // hit (icode, generic name, trade name) is the only path to a
        // Resolved verdict; anything else surfaces the closest formulary
        // candidates for the operator to disambiguate. The resolved
        // `DrugItem` (name/strength) labels the verdict — an icode search
        // shows which drug it referred to.
        let exact = self.resolve_drug_item(drug).await?;
        let candidates = if exact.is_none() {
            rank_candidates(self.search_drugs(drug).await?, RESOLUTION_CANDIDATE_LIMIT)
        } else {
            Vec::new()
        };
        let resolution = classify_drug_resolution(exact, candidates);
        let (records, truncated) = match &resolution {
            DrugResolution::Exact { drug } => self.fetch_all_history(hn, &drug.icode).await?,
            DrugResolution::Candidates { .. } => (Vec::new(), false),
        };
        Ok(verdict_from_resolution(resolution, records, truncated))
    }

    async fn check_drugs(
        &self,
        hn: &str,
        drugs: &[String],
    ) -> Result<Vec<DrugCheckResult>, RepositoryError> {
        // The same single question asked N times in one pass — each drug
        // runs its own OPD+IPD fan-out concurrently (pool size 5 caps the
        // load; a pharmacy batch is 2-5 drugs). Completion order differs
        // from input order — results carry their term label, so the UI
        // never relies on position.
        let mut tasks = tokio::task::JoinSet::new();
        for drug in drugs {
            let repo = self.clone();
            let hn = hn.to_string();
            let drug = drug.clone();
            tasks.spawn(async move {
                let verdict = repo.fetch_drug_history(&hn, &drug).await;
                (drug, verdict)
            });
        }
        let mut results = Vec::with_capacity(drugs.len());
        while let Some(joined) = tasks.join_next().await {
            let (term, verdict) = joined
                .map_err(|err| RepositoryError::Query(format!("batch check task failed: {err}")))?;
            results.push(DrugCheckResult {
                term,
                verdict: verdict?,
            });
        }
        Ok(results)
    }

    async fn fetch_concurrent_medications(
        &self,
        hn: &str,
    ) -> Result<Vec<ConcurrentMedication>, RepositoryError> {
        let hn = hn.trim();
        if hn.is_empty() {
            return Ok(Vec::new());
        }
        let rows: Vec<(String, NaiveDate, String, Option<String>, Option<String>)> = self
            .fetch_first_working(&[(CONCURRENT_MEDS_TRADE, &[hn]), (CONCURRENT_MEDS, &[hn])])
            .await?;
        Ok(rows
            .into_iter()
            .map(
                |(drug_code, last_date, drug_name, strength, trade_name)| ConcurrentMedication {
                    drug_code,
                    drug_name,
                    strength,
                    trade_name,
                    last_date,
                },
            )
            .collect())
    }
}

fn drug_from_row((icode, name, strength, trade_name): DrugRow) -> DrugItem {
    DrugItem {
        icode,
        name,
        strength,
        trade_name,
    }
}

/// Ensures every executed statement passes the guard — regression test for
/// the "every query in this crate is a read statement" rule (AGENTS.md §12).
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ping_statement_is_read_only() {
        assert!(assert_read_only(PING_SQL).is_ok());
    }

    #[test]
    fn non_database_errors_are_not_schema_variations() {
        assert!(!is_schema_variation(&sqlx::Error::RowNotFound));
    }

    #[test]
    fn history_limit_stays_in_sync_with_the_sql() {
        // Every HISTORY_* statement carries `LIMIT 200` — keep this in lock
        // step with HISTORY_LIMIT or the truncation flag lies.
        for sql in [
            HISTORY_OPD,
            HISTORY_OPD_FALLBACK,
            HISTORY_IPD_TAKEHOME,
            HISTORY_IPD_TAKEHOME_FALLBACK,
            HISTORY_IPD_STAY,
            HISTORY_IPD_STAY_TRADE,
        ] {
            assert!(
                sql.ends_with(&format!("LIMIT {HISTORY_LIMIT}")),
                "HISTORY_LIMIT must match the SQL cap: {sql}"
            );
        }
    }
}
