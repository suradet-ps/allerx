//! Production [`HosxRepository`] implementation over the read-only pool.

use async_trait::async_trait;
use chrono::NaiveDate;
use sqlx::MySqlPool;

use crate::error::Error;
use crate::queries::{
    DRUG_RESOLVE_BY_ICODE, DRUG_RESOLVE_BY_NAME, DRUG_SEARCH_CONTAINS, DRUG_SEARCH_PREFIX,
    HISTORY_IPD_STAY, HISTORY_IPD_TAKEHOME, HISTORY_OPD, PATIENT_SEARCH_BY_CID,
    PATIENT_SEARCH_BY_HN, PATIENT_SEARCH_NAME_CONTAINS, PATIENT_SEARCH_NAME_PREFIX,
};
use crate::readonly_guard::{PING_SQL, assert_read_only};
use allerx_models::{DrugHistoryRecord, DrugItem, PatientSummary, VisitType};
use allerx_search_core::{
    HosxRepository as HosxRepositoryTrait, QueryKind, RepositoryError, merge_drug_history,
};

/// One `patient` row as `(hn, cid, full_name_th, birth_date, sex)`,
/// matching the SELECT column order in `queries.rs`.
type PatientRow = (
    String,
    Option<String>,
    Option<String>,
    Option<NaiveDate>,
    Option<String>,
);

/// One history row as `(visit_date, drug_code, drug_name, prescriber,
/// department, route, quantity)` — same shape for OPD and both IPD sources.
type HistoryRow = (
    NaiveDate,
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);

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

    /// Resolves a drug term to its `icode`: exact icode hit, then exact
    /// display-name hit. `None` means the term is not in `drugitems`.
    async fn resolve_drug_icode(&self, drug: &str) -> Result<Option<String>, Error> {
        let by_icode = self.fetch_single_icode(DRUG_RESOLVE_BY_ICODE, drug).await?;
        if let Some(icode) = by_icode {
            return Ok(Some(icode));
        }
        self.fetch_single_icode(DRUG_RESOLVE_BY_NAME, drug).await
    }

    async fn fetch_single_icode(&self, sql: &str, param: &str) -> Result<Option<String>, Error> {
        assert_read_only(sql).map_err(|_| Error::Guard)?;
        sqlx::query_as::<_, (String,)>(sql)
            .bind(param)
            .fetch_optional(&self.pool)
            .await
            .map(|row| row.map(|(icode,)| icode))
            .map_err(Error::from)
    }

    /// OPD history — `opitemrece` rows without an admission number.
    async fn fetch_opd_history(
        &self,
        hn: &str,
        icode: &str,
    ) -> Result<Vec<DrugHistoryRecord>, Error> {
        let rows: Vec<HistoryRow> = self.guarded_fetch(HISTORY_OPD, &[hn, icode]).await?;
        Ok(rows
            .into_iter()
            .map(|r| history_record(r, VisitType::Opd))
            .collect())
    }

    /// IPD history — take-home rows in `opitemrece` plus in-stay rows in
    /// `iptitemrece` (AGENTS.md §6.3), run concurrently.
    async fn fetch_ipd_history(
        &self,
        hn: &str,
        icode: &str,
    ) -> Result<Vec<DrugHistoryRecord>, Error> {
        let (takehome, stay) = tokio::join!(
            self.fetch_ipd_takehome_history(hn, icode),
            self.fetch_ipd_stay_history(hn, icode),
        );
        let mut all = takehome?;
        all.extend(stay?);
        Ok(all)
    }

    async fn fetch_ipd_takehome_history(
        &self,
        hn: &str,
        icode: &str,
    ) -> Result<Vec<DrugHistoryRecord>, Error> {
        let rows: Vec<HistoryRow> = self
            .guarded_fetch(HISTORY_IPD_TAKEHOME, &[hn, icode])
            .await?;
        Ok(rows
            .into_iter()
            .map(|r| history_record(r, VisitType::Ipd))
            .collect())
    }

    /// In-stay IPD dispensing. A missing `iptitemrece` table (or a hospital
    /// that names it differently, AGENTS.md §6.3) is a documented schema
    /// variation — it yields no records instead of failing the whole lookup.
    async fn fetch_ipd_stay_history(
        &self,
        hn: &str,
        icode: &str,
    ) -> Result<Vec<DrugHistoryRecord>, Error> {
        assert_read_only(HISTORY_IPD_STAY).map_err(|_| Error::Guard)?;
        let rows = match self
            .raw_fetch::<HistoryRow>(HISTORY_IPD_STAY, &[hn, icode])
            .await
        {
            Ok(rows) => rows,
            Err(err) if is_schema_variation(&err) => Vec::new(),
            Err(err) => return Err(Error::Database(err)),
        };
        Ok(rows
            .into_iter()
            .map(|r| history_record(r, VisitType::Ipd))
            .collect())
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
    (visit_date, drug_code, drug_name, prescriber, department, route, quantity): HistoryRow,
    visit_type: VisitType,
) -> DrugHistoryRecord {
    DrugHistoryRecord {
        visit_date,
        visit_type,
        drug_code,
        drug_name,
        // `drugitems.trade_name` is not selected yet — column unverified
        // (AGENTS.md §6).
        trade_name: None,
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
            .guarded_fetch(DRUG_SEARCH_PREFIX, &[&prefix])
            .await?
            .into_iter()
            .map(|(icode, name, strength): (String, String, Option<String>)| DrugItem {
                icode,
                name,
                strength,
            })
            .collect();
        if hits.is_empty() {
            let contains = format!("%{term}%");
            hits = self
                .guarded_fetch(DRUG_SEARCH_CONTAINS, &[&contains])
                .await?
                .into_iter()
                .map(|(icode, name, strength): (String, String, Option<String>)| DrugItem {
                    icode,
                    name,
                    strength,
                })
                .collect();
        }
        Ok(hits)
    }

    async fn fetch_drug_history(
        &self,
        hn: &str,
        drug: &str,
    ) -> Result<Vec<DrugHistoryRecord>, RepositoryError> {
        let hn = hn.trim();
        let drug = drug.trim();
        if hn.is_empty() || drug.is_empty() {
            return Ok(Vec::new());
        }
        // OPD + IPD run concurrently, then merge most-recent-first
        // (AGENTS.md §7.2).
        let Some(icode) = self.resolve_drug_icode(drug).await? else {
            return Ok(Vec::new());
        };
        let (opd, ipd) = tokio::join!(
            self.fetch_opd_history(hn, &icode),
            self.fetch_ipd_history(hn, &icode),
        );
        Ok(merge_drug_history(opd?, ipd?))
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
}
