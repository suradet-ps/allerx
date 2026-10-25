//! Client-side application state (signals shared between components).

use allerx_models::{DrugHistoryRecord, DrugItem, PatientSummary, SessionUser};
use leptos::prelude::*;

/// Live HOSxP reachability, mirrored from the backend's `connection_health`
/// command (ROADMAP Phase 3) - drives the top-bar status dot. The backend
/// keeps it fresh with a 30 s ping loop and every query outcome; this
/// frontend value is a polled copy, never computed from a file check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ConnectionHealth {
    /// No stored settings - the settings dialog is the flow.
    Unconfigured,
    /// A ping succeeded recently.
    Connected,
    /// HOSxP could not be reached.
    Disconnected,
}

impl ConnectionHealth {
    /// Thai label for the login screen and status dot.
    pub fn label_th(self) -> &'static str {
        match self {
            Self::Unconfigured => "ยังไม่ได้ตั้งค่าการเชื่อมต่อ",
            Self::Connected => "เชื่อมต่อ HOSxP แล้ว",
            Self::Disconnected => "HOSxP ไม่พร้อมใช้งาน",
        }
    }
}

/// One drug the pharmacist queued for checking (ROADMAP Phase 5). `icode`
/// is set when the drug was picked from the autocomplete (checked exactly);
/// free text keeps `None` and goes through resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrugChip {
    pub label: String,
    pub icode: Option<String>,
}

/// Per-drug verdict in the results list (ROADMAP Phase 5) - the same
/// three-state contract as before, now labelled by its drug term.
#[derive(Debug, Clone, PartialEq)]
pub struct DrugVerdict {
    /// The term the pharmacist submitted - labels this verdict.
    pub term: String,
    pub state: DrugVerdictState,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DrugVerdictState {
    /// History found; `drug` is the resolved formulary entry (its
    /// name/strength labels the verdict), records are most-recent-first,
    /// and `truncated` means older history exists beyond the per-source cap.
    Found {
        drug: DrugItem,
        records: Vec<DrugHistoryRecord>,
        truncated: bool,
    },
    /// History searched and definitively not found - the drug resolved (its
    /// identity is shown so the verdict never refers to an unknown item),
    /// but no dispensing rows exist.
    NotFound { drug: DrugItem },
    /// The drug term could not be matched to the formulary. `candidates`
    /// are the closest matches (empty = not in `drugitems` at all). Never
    /// render "ไม่พบประวัติ" in this state.
    Unresolved { candidates: Vec<DrugItem> },
}

/// Shared state for the single-page flow.
#[derive(Debug, Clone)]
pub struct AppState {
    /// Selected patient; `None` until one is picked from search results.
    pub patient: RwSignal<Option<PatientSummary>>,
    /// The current verdict-band state (DESIGN.md, verdict band).
    pub verdict: RwSignal<VerdictState>,
    /// Drugs queued in the sidebar for checking (ROADMAP Phase 5).
    pub drug_chips: RwSignal<Vec<DrugChip>>,
    /// Whether encrypted HOSxP connection settings exist on this machine.
    pub configured: RwSignal<bool>,
    /// Signed-in HOSxP officer, mirrored from the backend session.
    pub session: RwSignal<Option<SessionUser>>,
    /// True once the startup probe (settings, connection, session) finished.
    pub auth_checked: RwSignal<bool>,
    /// Bumped on every login/logout so an in-flight query started under a
    /// previous session can never write its result into the new session.
    pub auth_generation: RwSignal<u64>,
    /// Whether the connection settings dialog is open.
    pub settings_open: RwSignal<bool>,
    /// Whether the patient detail modal is open.
    pub detail_open: RwSignal<bool>,
    /// Polled live reachability (ROADMAP Phase 3) - top-bar dot source.
    pub health: RwSignal<ConnectionHealth>,
    /// Degraded-mode banner message (ROADMAP Phase 3): set when a query
    /// cannot reach HOSxP, cleared on the next success or by the operator.
    pub db_banner: RwSignal<Option<String>>,
    /// Monotonic counter of completed checks - the timeline uses it to
    /// detect "a new check replaced the view" even when the results content
    /// is identical (same terms, same counts, same truncation flags), so
    /// the per-drug filter always resets with a new check.
    pub check_seq: RwSignal<u64>,
}

/// The one loud thing on screen - only one state at a time, and the new
/// state fully replaces the old one (DESIGN.md, verdict band rule).
///
/// Since Phase 5 every check is a batch (a single drug is a batch of one),
/// so the state holds one labelled verdict per submitted drug. The
/// three-way split per drug is deliberate (ROADMAP Phase 1): a
/// "ไม่พบประวัติ" verdict is only ever produced when the drug is known.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum VerdictState {
    /// Query in flight / nothing searched yet - neutral gray, never implies
    /// an answer.
    #[default]
    Pending,
    /// Checked drugs with their verdicts (one entry per submitted term).
    Results { results: Vec<DrugVerdict> },
}

impl AppState {
    pub fn new() -> Self {
        Self {
            patient: RwSignal::new(None),
            verdict: RwSignal::new(VerdictState::Pending),
            drug_chips: RwSignal::new(Vec::new()),
            configured: RwSignal::new(false),
            session: RwSignal::new(None),
            auth_checked: RwSignal::new(false),
            auth_generation: RwSignal::new(0),
            settings_open: RwSignal::new(false),
            detail_open: RwSignal::new(false),
            health: RwSignal::new(ConnectionHealth::Unconfigured),
            db_banner: RwSignal::new(None),
            check_seq: RwSignal::new(0),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    /// Completes a successful login. Any prior patient/drug state belongs to
    /// the previous operator and is dropped.
    pub fn finish_login(&self, user: SessionUser) {
        self.clear_work();
        self.auth_generation.update(|generation| *generation += 1);
        self.session.set(Some(user));
    }

    /// Clears the session UI (the backend logout is performed by the
    /// caller). Also used after a connection change, which invalidates the
    /// session server-side.
    pub fn clear_session(&self) {
        self.auth_generation.update(|generation| *generation += 1);
        self.session.set(None);
        self.clear_work();
        self.settings_open.set(false);
    }

    /// Drops everything that belongs to the signed-in operator's current
    /// patient and drug check.
    fn clear_work(&self) {
        self.patient.set(None);
        self.verdict.set(VerdictState::Pending);
        self.drug_chips.set(Vec::new());
        self.detail_open.set(false);
        self.db_banner.set(None);
        self.check_seq.set(0);
    }
}

/// Merges every found drug's records into one chronological timeline,
/// newest first, with the truncation flag OR'd across results. Pure - the
/// timeline below a multi-drug verdict is one object, not per-drug noise.
pub fn merged_timeline(results: &[DrugVerdict]) -> (Vec<DrugHistoryRecord>, bool) {
    let mut records: Vec<DrugHistoryRecord> = results
        .iter()
        .flat_map(|r| match &r.state {
            DrugVerdictState::Found { records, .. } => records.clone(),
            _ => Vec::new(),
        })
        .collect();
    records.sort_by_key(|r| std::cmp::Reverse(r.visit_date));
    let truncated = results.iter().any(|r| {
        matches!(
            &r.state,
            DrugVerdictState::Found {
                truncated: true,
                ..
            }
        )
    });
    (records, truncated)
}

/// The resolved drug's display identity: "name (strength)" when a strength
/// is known - shown in verdict bands so a search by icode reveals which
/// drug a verdict refers to (pilot feedback).
pub fn drug_identity(drug: &DrugItem) -> String {
    match &drug.strength {
        Some(strength) => format!("{} ({strength})", drug.name),
        None => drug.name.clone(),
    }
}

/// A history record's display label: "name (strength)" - the timeline rows
/// always show the drug with its strength.
pub fn record_label(record: &DrugHistoryRecord) -> String {
    match &record.strength {
        Some(strength) => format!("{} ({strength})", record.drug_name),
        None => record.drug_name.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use allerx_models::VisitType;
    use chrono::NaiveDate;

    fn record(date: NaiveDate, drug_name: &str) -> DrugHistoryRecord {
        DrugHistoryRecord {
            visit_date: date,
            visit_type: VisitType::Opd,
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

    fn found(records: Vec<DrugHistoryRecord>, truncated: bool) -> DrugVerdict {
        DrugVerdict {
            term: "x".into(),
            state: DrugVerdictState::Found {
                drug: DrugItem {
                    icode: "1-001".into(),
                    name: "พาราเซตามอล".into(),
                    strength: Some("500 mg".into()),
                    trade_name: None,
                },
                records,
                truncated,
            },
        }
    }

    fn not_found(term: &str) -> DrugVerdict {
        DrugVerdict {
            term: term.into(),
            state: DrugVerdictState::NotFound {
                drug: DrugItem {
                    icode: "1-001".into(),
                    name: "พาราเซตามอล".into(),
                    strength: None,
                    trade_name: None,
                },
            },
        }
    }

    #[test]
    fn new_state_has_no_session_until_startup_finishes() {
        let state = AppState::new();
        assert!(state.session.get_untracked().is_none());
        assert!(!state.auth_checked.get_untracked());
    }

    #[test]
    fn finish_login_sets_the_session_and_clear_session_drops_work() {
        let state = AppState::new();
        state.patient.set(Some(PatientSummary {
            hn: "00012345".into(),
            cid: None,
            full_name_th: "ทดสอบ".into(),
            birth_date: None,
            sex: None,
        }));
        state.finish_login(SessionUser {
            officer_id: 7,
            login_name: "pharm".into(),
            display_name: "Pharmacist".into(),
            doctor_code: None,
            role: "STAFF".into(),
        });
        assert_eq!(
            state.session.get_untracked().map(|user| user.login_name),
            Some("pharm".to_string())
        );
        assert!(state.patient.get_untracked().is_none());
        let generation = state.auth_generation.get_untracked();
        state.clear_session();
        assert!(state.session.get_untracked().is_none());
        assert!(state.auth_generation.get_untracked() > generation);
    }

    #[test]
    fn merged_timeline_sorts_across_drugs_newest_first() {
        let d1 = NaiveDate::from_ymd_opt(2024, 1, 1).expect("valid date");
        let d2 = NaiveDate::from_ymd_opt(2024, 6, 6).expect("valid date");
        let d3 = NaiveDate::from_ymd_opt(2024, 3, 3).expect("valid date");
        let results = vec![
            found(vec![record(d1, "a"), record(d2, "a")], false),
            found(vec![record(d3, "b")], false),
        ];
        let (records, truncated) = merged_timeline(&results);
        let dates: Vec<_> = records.iter().map(|r| r.visit_date).collect();
        assert_eq!(dates, vec![d2, d3, d1]);
        assert!(!truncated);
    }

    #[test]
    fn merged_timeline_or_s_truncation_flags() {
        let d = NaiveDate::from_ymd_opt(2024, 1, 1).expect("valid date");
        let results = vec![found(vec![record(d, "a")], true), not_found("b")];
        let (records, truncated) = merged_timeline(&results);
        assert_eq!(records.len(), 1);
        assert!(truncated);
    }

    #[test]
    fn merged_timeline_skips_non_found_drugs() {
        let results = vec![not_found("b")];
        let (records, truncated) = merged_timeline(&results);
        assert!(records.is_empty());
        assert!(!truncated);
    }
}
