//! Merging and ordering of medication history (AGENTS.md §7.2).

use allerx_models::{DrugHistoryRecord, VisitType};

/// Merges OPD and IPD history into one timeline, most recent first.
///
/// Records sharing the same date keep OPD before IPD (stable tie-break).
/// The returned order is information itself — the timeline is read
/// top-to-bottom in the UI (DESIGN.md).
pub fn merge_drug_history(
    opd: Vec<DrugHistoryRecord>,
    ipd: Vec<DrugHistoryRecord>,
) -> Vec<DrugHistoryRecord> {
    let mut all = opd;
    all.extend(ipd);
    all.sort_by(|a, b| {
        b.visit_date
            .cmp(&a.visit_date)
            .then_with(|| visit_type_rank(a.visit_type).cmp(&visit_type_rank(b.visit_type)))
    });
    all
}

/// OPD sorts before IPD on equal dates.
fn visit_type_rank(visit_type: VisitType) -> u8 {
    match visit_type {
        VisitType::Opd => 0,
        VisitType::Ipd => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use allerx_models::VisitType::{Ipd, Opd};
    use chrono::NaiveDate;

    fn record(date: NaiveDate, visit_type: VisitType, drug_name: &str) -> DrugHistoryRecord {
        DrugHistoryRecord {
            visit_date: date,
            visit_type,
            drug_code: "1-001".into(),
            drug_name: drug_name.into(),
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

    #[test]
    fn merges_and_sorts_most_recent_first() {
        let opd = vec![
            record(date(2024, 1, 1), Opd, "a"),
            record(date(2024, 3, 3), Opd, "b"),
        ];
        let ipd = vec![record(date(2024, 2, 2), Ipd, "c")];

        let merged = merge_drug_history(opd, ipd);

        let dates: Vec<_> = merged.iter().map(|r| r.visit_date).collect();
        assert_eq!(
            dates,
            vec![date(2024, 3, 3), date(2024, 2, 2), date(2024, 1, 1)]
        );
    }

    #[test]
    fn equal_dates_keep_opd_before_ipd() {
        let d = date(2024, 5, 5);
        let merged = merge_drug_history(
            vec![record(d, Opd, "opd")],
            vec![record(d, Ipd, "ipd"), record(d, Ipd, "ipd2")],
        );

        let types: Vec<_> = merged.iter().map(|r| r.visit_type).collect();
        assert_eq!(types, vec![Opd, Ipd, Ipd]);
    }

    #[test]
    fn empty_inputs_produce_empty_output() {
        assert!(merge_drug_history(vec![], vec![]).is_empty());
    }

    #[test]
    fn single_side_passes_through_unchanged() {
        let opd = vec![record(date(2024, 1, 1), Opd, "a")];
        assert_eq!(merge_drug_history(opd.clone(), vec![]), opd);
        let ipd = vec![record(date(2024, 1, 1), Ipd, "b")];
        assert_eq!(merge_drug_history(vec![], ipd.clone()), ipd);
    }
}
