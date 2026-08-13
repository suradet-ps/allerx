//! In-memory repository for tests and placeholder frontend use.

use std::collections::HashMap;

use allerx_models::{
    ConcurrentMedication, DrugHistoryRecord, DrugItem, HistoryVerdict, PatientSummary,
};
use async_trait::async_trait;

use crate::error::RepositoryError;
use crate::query_kind::QueryKind;
use crate::repository::HosxRepository;
use crate::resolution::{classify_drug_resolution, rank_candidates, verdict_from_resolution};

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
    ) -> Result<HistoryVerdict, RepositoryError> {
        let needle = drug.trim().to_lowercase();
        if needle.is_empty() {
            // Blank input is an unresolvable term, never a "not found".
            return Ok(HistoryVerdict::Unresolved {
                candidates: Vec::new(),
            });
        }
        // Exact hit against the known drug list (icode or name) resolves.
        let exact = self
            .drugs
            .iter()
            .find(|d| d.icode.to_lowercase() == needle || d.name.to_lowercase() == needle);
        let records = if let Some(hit) = exact {
            let mut hits: Vec<DrugHistoryRecord> = self
                .history
                .iter()
                .filter(|r| r.drug_code.to_lowercase() == hit.icode.to_lowercase())
                .cloned()
                .collect();
            // Ordering is preserved most-recent-first so the contract
            // matches the real repository.
            hits.sort_by_key(|r| std::cmp::Reverse(r.visit_date));
            hits
        } else {
            Vec::new()
        };
        let resolution = classify_drug_resolution(
            exact.cloned(),
            rank_candidates(self.search_drugs(drug).await?, 10),
        );
        Ok(verdict_from_resolution(resolution, records, false))
    }

    async fn fetch_concurrent_medications(
        &self,
        _hn: &str,
    ) -> Result<Vec<ConcurrentMedication>, RepositoryError> {
        // Dedupe the mock's history per icode, keeping the latest date —
        // mirrors the real repository's GROUP BY semantics.
        let mut latest: HashMap<&str, (chrono::NaiveDate, &str, Option<&str>)> = HashMap::new();
        for record in &self.history {
            let entry = latest.entry(record.drug_code.as_str()).or_insert((
                record.visit_date,
                record.drug_name.as_str(),
                record.strength.as_deref(),
            ));
            if record.visit_date > entry.0 {
                *entry = (
                    record.visit_date,
                    record.drug_name.as_str(),
                    record.strength.as_deref(),
                );
            }
        }
        let mut meds: Vec<ConcurrentMedication> = latest
            .into_iter()
            .map(
                |(drug_code, (last_date, drug_name, strength))| ConcurrentMedication {
                    drug_code: drug_code.to_string(),
                    drug_name: drug_name.to_string(),
                    strength: strength.map(str::to_string),
                    trade_name: None,
                    last_date,
                },
            )
            .collect();
        meds.sort_by_key(|m| std::cmp::Reverse(m.last_date));
        meds.truncate(30);
        Ok(meds)
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
                strength: Some("500 mg".into()),
                trade_name: None,
            },
            DrugItem {
                icode: "1-002".into(),
                name: "แอมม็อกซิซิลลิน".into(),
                strength: Some("250 mg".into()),
                trade_name: None,
            },
        ]
    }

    fn record(date: NaiveDate, visit_type: allerx_models::VisitType) -> DrugHistoryRecord {
        DrugHistoryRecord {
            visit_date: date,
            visit_type,
            drug_code: "1-001".into(),
            drug_name: "พาราเซตามอล".into(),
            strength: Some("500 mg".into()),
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
        let repo = MockRepository::new(true)
            .with_drugs(sample_drugs())
            .with_history(history);
        let verdict = repo
            .fetch_drug_history("00012345", "1-001")
            .await
            .expect("mock history succeeds");
        match verdict {
            HistoryVerdict::Resolved { history, .. } => {
                let dates: Vec<_> = history.records.iter().map(|r| r.visit_date).collect();
                assert_eq!(dates, vec![date(2024, 5, 5), date(2024, 1, 1)]);
                assert!(!history.truncated);
            }
            other => panic!("expected Resolved, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn history_resolves_by_exact_name() {
        let history = vec![record(date(2024, 3, 3), Opd)];
        let repo = MockRepository::new(true)
            .with_drugs(sample_drugs())
            .with_history(history);
        let verdict = repo
            .fetch_drug_history("00012345", "พาราเซตามอล")
            .await
            .expect("mock history succeeds");
        assert!(matches!(verdict, HistoryVerdict::Resolved { .. }));
    }

    #[tokio::test]
    async fn known_drug_with_no_history_is_a_definitive_not_found() {
        let repo = MockRepository::new(true).with_drugs(sample_drugs());
        let verdict = repo
            .fetch_drug_history("00012345", "1-001")
            .await
            .expect("mock history succeeds");
        match verdict {
            HistoryVerdict::Resolved { history, .. } => assert!(history.records.is_empty()),
            other => panic!("expected Resolved, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn unknown_drug_returns_candidates_never_not_found() {
        let repo = MockRepository::new(true).with_drugs(sample_drugs());
        let verdict = repo
            .fetch_drug_history("00012345", "พารา")
            .await
            .expect("mock history succeeds");
        match verdict {
            HistoryVerdict::Unresolved { candidates } => {
                assert_eq!(candidates.len(), 1);
                assert_eq!(candidates[0].icode, "1-001");
            }
            other => panic!("expected Unresolved, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn unknown_drug_with_no_candidates_is_unresolved_empty() {
        let repo = MockRepository::new(true).with_drugs(sample_drugs());
        let verdict = repo
            .fetch_drug_history("00012345", "ไม่มีในระบบเลย")
            .await
            .expect("mock history succeeds");
        match verdict {
            HistoryVerdict::Unresolved { candidates } => assert!(candidates.is_empty()),
            other => panic!("expected Unresolved, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn blank_drug_is_unresolved_never_not_found() {
        let repo = MockRepository::new(true).with_drugs(sample_drugs());
        let verdict = repo
            .fetch_drug_history("00012345", " ")
            .await
            .expect("mock history succeeds");
        assert!(matches!(verdict, HistoryVerdict::Unresolved { .. }));
    }

    #[tokio::test]
    async fn check_drugs_returns_one_result_per_term_in_order() {
        let history = vec![record(date(2024, 3, 3), Opd)];
        let repo = MockRepository::new(true)
            .with_drugs(sample_drugs())
            .with_history(history);
        let drugs = vec!["พาราเซตามอล".to_string(), "ไม่มีในระบบเลย".to_string()];
        let results = repo
            .check_drugs("00012345", &drugs)
            .await
            .expect("mock batch succeeds");

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].term, "พาราเซตามอล");
        assert!(matches!(
            results[0].verdict,
            HistoryVerdict::Resolved { .. }
        ));
        assert_eq!(results[1].term, "ไม่มีในระบบเลย");
        assert!(matches!(
            results[1].verdict,
            HistoryVerdict::Unresolved { .. }
        ));
    }

    #[tokio::test]
    async fn check_drugs_empty_input_yields_empty_results() {
        let repo = MockRepository::new(true).with_drugs(sample_drugs());
        let results = repo
            .check_drugs("00012345", &[])
            .await
            .expect("mock batch succeeds");
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn concurrent_medications_dedupe_by_icode_and_sort_newest_first() {
        let history = vec![
            record(date(2024, 1, 1), Opd),
            record(date(2024, 5, 5), Ipd),
            record(date(2024, 4, 4), Opd),
        ];
        let repo = MockRepository::new(true).with_history(history);
        let meds = repo
            .fetch_concurrent_medications("00012345")
            .await
            .expect("mock meds succeeds");
        // All three records share icode 1-001 → one medication, newest date.
        assert_eq!(meds.len(), 1);
        assert_eq!(meds[0].drug_code, "1-001");
        assert_eq!(meds[0].last_date, date(2024, 5, 5));
    }
}
