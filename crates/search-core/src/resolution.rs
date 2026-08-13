//! Drug term resolution — pure logic for mapping a typed drug term to an
//! icode, and for the verdict invariant "resolution failure ⇒ never a
//! 'not found' verdict" (ROADMAP Phase 1, Gap G1).
//!
//! The database work (exact lookups, candidate search) happens in
//! `hosxp-connector`; this module only classifies and ranks.

use allerx_models::{DrugItem, HistoryVerdict, ResolvedHistory};

/// The outcome of mapping a typed drug term to `drugitems` entries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DrugResolution {
    /// The term matched exactly (by icode, generic name, or trade name) —
    /// history lookup proceeds against this drug, whose name/strength
    /// identity labels the verdict.
    Exact { drug: DrugItem },
    /// No exact match; `items` are the closest formulary entries (possibly
    /// empty — the term is not in the formulary at all). The operator must
    /// disambiguate before any history lookup.
    Candidates { items: Vec<DrugItem> },
}

/// Classifies an exact hit (from exact icode/name/trade-name lookups)
/// against the candidate shortlist.
///
/// The invariant this function encodes: an exact hit is the only path to a
/// `Resolved` verdict. A failed resolution always lands in
/// [`DrugResolution::Candidates`], so no caller can accidentally produce a
/// "ไม่พบประวัติ" verdict for an unresolvable drug term.
pub fn classify_drug_resolution(
    exact: Option<DrugItem>,
    candidates: Vec<DrugItem>,
) -> DrugResolution {
    match exact {
        Some(drug) => DrugResolution::Exact { drug },
        None => DrugResolution::Candidates { items: candidates },
    }
}

/// Sorts candidates by name (stable, icode tie-break) and caps the list —
/// the disambiguation UI should show at most `limit` entries.
pub fn rank_candidates(mut items: Vec<DrugItem>, limit: usize) -> Vec<DrugItem> {
    items.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.icode.cmp(&b.icode)));
    items.truncate(limit);
    items
}

/// Builds the verdict contract from a resolution and (for exact hits) the
/// fetched timeline.
///
/// This is the single place where `HistoryVerdict` is constructed — the
/// "resolution failed ⇒ never Resolved/NotFound" invariant is enforced here
/// by construction: `Candidates` drops `records` and always yields
/// `Unresolved`.
pub fn verdict_from_resolution(
    resolution: DrugResolution,
    records: Vec<allerx_models::DrugHistoryRecord>,
    truncated: bool,
) -> HistoryVerdict {
    match resolution {
        DrugResolution::Exact { drug } => HistoryVerdict::Resolved {
            drug,
            history: ResolvedHistory { records, truncated },
        },
        DrugResolution::Candidates { items } => HistoryVerdict::Unresolved { candidates: items },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use allerx_models::DrugHistoryRecord;
    use allerx_models::VisitType::Opd;
    use chrono::NaiveDate;

    fn item(name: &str, icode: &str) -> DrugItem {
        DrugItem {
            icode: icode.into(),
            name: name.into(),
            strength: None,
            trade_name: None,
        }
    }

    fn record(drug_name: &str) -> DrugHistoryRecord {
        DrugHistoryRecord {
            visit_date: NaiveDate::from_ymd_opt(2024, 1, 1).expect("valid date in test"),
            visit_type: Opd,
            drug_code: "1-001".into(),
            drug_name: drug_name.into(),
            strength: Some("500 mg".into()),
            trade_name: None,
            prescriber: None,
            department: None,
            quantity: None,
            route: None,
        }
    }

    #[test]
    fn exact_hit_is_never_ambiguous_even_with_empty_candidates() {
        let resolution = classify_drug_resolution(Some(item("พาราเซตามอล", "1-001")), Vec::new());
        assert_eq!(
            resolution,
            DrugResolution::Exact {
                drug: item("พาราเซตามอล", "1-001")
            }
        );
    }

    #[test]
    fn failed_resolution_always_yields_candidates() {
        let resolution = classify_drug_resolution(None, vec![item("พาราเซตามอล", "1-001")]);
        assert_eq!(
            resolution,
            DrugResolution::Candidates {
                items: vec![item("พาราเซตามอล", "1-001")]
            }
        );
    }

    #[test]
    fn rank_sorts_by_name_then_icode_and_caps_the_list() {
        let items = vec![
            item("z", "1-003"),
            item("a", "1-002"),
            item("a", "1-001"),
            item("b", "1-004"),
        ];
        let ranked = rank_candidates(items, 3);
        let codes: Vec<_> = ranked.iter().map(|i| i.icode.as_str()).collect();
        assert_eq!(codes, vec!["1-001", "1-002", "1-004"]);
    }

    #[test]
    fn empty_candidates_rank_to_empty() {
        assert!(rank_candidates(Vec::new(), 10).is_empty());
    }

    #[test]
    fn exact_with_empty_records_is_a_definitive_not_found() {
        let verdict = verdict_from_resolution(
            DrugResolution::Exact {
                drug: item("พาราเซตามอล", "1-001"),
            },
            Vec::new(),
            false,
        );
        match verdict {
            HistoryVerdict::Resolved { drug, history } => {
                assert_eq!(drug.icode, "1-001");
                assert!(history.records.is_empty());
                assert!(!history.truncated);
            }
            other => panic!("expected Resolved, got {other:?}"),
        }
    }

    #[test]
    fn exact_with_records_passes_through_and_keeps_truncation_flag() {
        let verdict = verdict_from_resolution(
            DrugResolution::Exact {
                drug: item("พาราเซตามอล", "1-001"),
            },
            vec![record("พาราเซตามอล")],
            true,
        );
        match verdict {
            HistoryVerdict::Resolved { drug, history } => {
                assert_eq!(drug.name, "พาราเซตามอล");
                assert_eq!(history.records.len(), 1);
                assert!(history.truncated);
            }
            other => panic!("expected Resolved, got {other:?}"),
        }
    }

    #[test]
    fn candidates_never_become_a_not_found_verdict_even_with_stray_records() {
        // The invariant: a failed resolution must never surface as
        // Resolved-empty ("ไม่พบประวัติ"), even if a caller passes records.
        let verdict = verdict_from_resolution(
            DrugResolution::Candidates {
                items: vec![item("พาราเซตามอล", "1-001")],
            },
            vec![record("พาราเซตามอล")],
            false,
        );
        match verdict {
            HistoryVerdict::Unresolved { candidates } => {
                assert_eq!(candidates.len(), 1);
            }
            other => panic!("expected Unresolved, got {other:?}"),
        }
    }
}
