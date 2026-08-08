//! In-memory repository for tests and placeholder frontend use.

use allerx_models::{DrugHistoryRecord, DrugItem, PatientSummary};
use async_trait::async_trait;

use crate::error::RepositoryError;
use crate::query_kind::QueryKind;
use crate::repository::HosxRepository;

/// A [`HosxRepository`] that never touches a database.
#[derive(Debug, Clone)]
pub struct MockRepository {
    ping_ok: bool,
    patients: Vec<PatientSummary>,
    drugs: Vec<DrugItem>,
    history: Vec<DrugHistoryRecord>,
}

impl MockRepository {
    /// Creates a mock whose [`ping`](Self::ping) succeeds when `ping_ok` is `true`.
    pub fn new(ping_ok: bool) -> Self {
        Self {
            ping_ok,
            patients: Vec::new(),
            drugs: Vec::new(),
            history: Vec::new(),
        }
    }

    /// Sets the patients [`search_patients`](Self::search_patients) filters.
    pub fn with_patients(mut self, patients: Vec<PatientSummary>) -> Self {
        self.patients = patients;
        self
    }

    /// Sets the drugs [`search_drugs`](Self::search_drugs) filters.
    pub fn with_drugs(mut self, drugs: Vec<DrugItem>) -> Self {
        self.drugs = drugs;
        self
    }

    /// Sets the records [`fetch_drug_history`](Self::fetch_drug_history) filters.
    pub fn with_history(mut self, history: Vec<DrugHistoryRecord>) -> Self {
        self.history = history;
        self
    }
}

#[async_trait]
impl HosxRepository for MockRepository {
    async fn ping(&self) -> Result<(), RepositoryError> {
        if self.ping_ok {
            Ok(())
        } else {
            Err(RepositoryError::Connection)
        }
    }

    async fn search_patients(
        &self,
        term: &str,
        _kind: QueryKind,
    ) -> Result<Vec<PatientSummary>, RepositoryError> {
        let needle = term.trim().to_lowercase();
        if needle.is_empty() {
            return Ok(Vec::new());
        }
        Ok(self
            .patients
            .iter()
            .filter(|p| {
                p.hn.to_lowercase().contains(&needle)
                    || p.cid
                        .as_deref()
                        .is_some_and(|cid| cid.to_lowercase().contains(&needle))
                    || p.full_name_th.to_lowercase().contains(&needle)
            })
            .cloned()
            .collect())
    }

    async fn search_drugs(&self, term: &str) -> Result<Vec<DrugItem>, RepositoryError> {
        let needle = term.trim().to_lowercase();
        if needle.is_empty() {
            return Ok(Vec::new());
        }
        Ok(self
            .drugs
            .iter()
            .filter(|d| {
                d.icode.to_lowercase().contains(&needle) || d.name.to_lowercase().contains(&needle)
            })
            .cloned()
            .collect())
    }

    async fn fetch_drug_history(
        &self,
        _hn: &str,
        drug: &str,
    ) -> Result<Vec<DrugHistoryRecord>, RepositoryError> {
        let needle = drug.trim().to_lowercase();
        if needle.is_empty() {
            return Ok(Vec::new());
        }
        let mut hits: Vec<DrugHistoryRecord> = self
            .history
            .iter()
            .filter(|r| {
                r.drug_code.to_lowercase() == needle || r.drug_name.to_lowercase().contains(&needle)
            })
            .cloned()
            .collect();
        // The mock has no patient table to filter HN against — it filters on
        // the drug only. Ordering is preserved most-recent-first so the
        // contract matches the real repository.
        hits.sort_by_key(|r| std::cmp::Reverse(r.visit_date));
        Ok(hits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use allerx_models::VisitType::{Ipd, Opd};
    use chrono::NaiveDate;

    fn sample_patients() -> Vec<PatientSummary> {
        vec![
            PatientSummary {
                hn: "00012345".into(),
                cid: Some("1101701234567".into()),
                full_name_th: "สมชาย ใจดี".into(),
                birth_date: None,
                sex: Some("1".into()),
            },
            PatientSummary {
                hn: "00054321".into(),
                cid: None,
                full_name_th: "สมหญิง รักดี".into(),
                birth_date: None,
                sex: Some("2".into()),
            },
        ]
    }

    fn sample_drugs() -> Vec<DrugItem> {
        vec![
            DrugItem {
                icode: "1-001".into(),
                name: "พาราเซตามอล".into(),
            },
            DrugItem {
                icode: "1-002".into(),
                name: "แอมม็อกซิซิลลิน".into(),
            },
        ]
    }

    fn record(date: NaiveDate, visit_type: allerx_models::VisitType) -> DrugHistoryRecord {
        DrugHistoryRecord {
            visit_date: date,
            visit_type,
            drug_code: "1-001".into(),
            drug_name: "พาราเซตามอล".into(),
            trade_name: None,
            prescriber: None,
            department: None,
            quantity: None,
            route: None,
        }
    }

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).expect("valid date in test")
    }

    #[tokio::test]
    async fn ping_ok_when_configured() {
        assert!(MockRepository::new(true).ping().await.is_ok());
    }

    #[tokio::test]
    async fn ping_fails_when_configured() {
        assert_eq!(
            MockRepository::new(false).ping().await,
            Err(RepositoryError::Connection)
        );
    }

    #[tokio::test]
    async fn search_by_hn_returns_only_that_patient() {
        let repo = MockRepository::new(true).with_patients(sample_patients());
        let hits = repo
            .search_patients("00054321", QueryKind::Hn)
            .await
            .expect("mock search succeeds");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].hn, "00054321");
    }

    #[tokio::test]
    async fn search_by_cid_returns_only_that_patient() {
        let repo = MockRepository::new(true).with_patients(sample_patients());
        let hits = repo
            .search_patients("1101701234567", QueryKind::Cid)
            .await
            .expect("mock search succeeds");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].full_name_th, "สมชาย ใจดี");
    }

    #[tokio::test]
    async fn search_by_name_matches_substring_case_insensitively() {
        let repo = MockRepository::new(true).with_patients(sample_patients());
        let hits = repo
            .search_patients("รัก", QueryKind::Name)
            .await
            .expect("mock search succeeds");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].hn, "00054321");
    }

    #[tokio::test]
    async fn search_with_no_match_returns_empty_list() {
        let repo = MockRepository::new(true).with_patients(sample_patients());
        let hits = repo
            .search_patients("ไม่มีใครชื่อนี้", QueryKind::Name)
            .await
            .expect("mock search succeeds");
        assert!(hits.is_empty());
    }

    #[tokio::test]
    async fn search_blank_term_returns_empty_list() {
        let repo = MockRepository::new(true).with_patients(sample_patients());
        let hits = repo
            .search_patients("   ", QueryKind::Name)
            .await
            .expect("mock search succeeds");
        assert!(hits.is_empty());
    }

    #[tokio::test]
    async fn search_drugs_matches_by_name_or_icode() {
        let repo = MockRepository::new(true).with_drugs(sample_drugs());
        let by_name = repo
            .search_drugs("พารา")
            .await
            .expect("mock search succeeds");
        assert_eq!(by_name.len(), 1);
        assert_eq!(by_name[0].icode, "1-001");

        let by_code = repo
            .search_drugs("1-002")
            .await
            .expect("mock search succeeds");
        assert_eq!(by_code.len(), 1);
        assert_eq!(by_code[0].name, "แอมม็อกซิซิลลิน");
    }

    #[tokio::test]
    async fn search_drugs_blank_or_no_match_returns_empty() {
        let repo = MockRepository::new(true).with_drugs(sample_drugs());
        assert!(
            repo.search_drugs("  ")
                .await
                .expect("mock search succeeds")
                .is_empty()
        );
        assert!(
            repo.search_drugs("ไม่มีในระบบ")
                .await
                .expect("mock search succeeds")
                .is_empty()
        );
    }

    #[tokio::test]
    async fn history_matches_by_icode_and_sorts_most_recent_first() {
        let history = vec![record(date(2024, 1, 1), Opd), record(date(2024, 5, 5), Ipd)];
        let repo = MockRepository::new(true).with_history(history);
        let hits = repo
            .fetch_drug_history("00012345", "1-001")
            .await
            .expect("mock history succeeds");
        let dates: Vec<_> = hits.iter().map(|r| r.visit_date).collect();
        assert_eq!(dates, vec![date(2024, 5, 5), date(2024, 1, 1)]);
    }

    #[tokio::test]
    async fn history_no_match_or_blank_drug_returns_empty() {
        let repo = MockRepository::new(true).with_history(vec![record(date(2024, 1, 1), Opd)]);
        assert!(
            repo.fetch_drug_history("00012345", "9-999")
                .await
                .expect("mock history succeeds")
                .is_empty()
        );
        assert!(
            repo.fetch_drug_history("00012345", " ")
                .await
                .expect("mock history succeeds")
                .is_empty()
        );
    }
}
