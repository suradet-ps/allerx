//! Client-side application state (signals shared between components).

use allerx_models::{DrugHistoryRecord, DrugItem, PatientSummary};
use leptos::prelude::*;

/// Shared state for the single-page flow.
#[derive(Debug, Clone)]
pub struct AppState {
    /// Selected patient; `None` until one is picked from search results.
    pub patient: RwSignal<Option<PatientSummary>>,
    /// The current verdict-band state (DESIGN.md, verdict band).
    pub verdict: RwSignal<VerdictState>,
    /// Whether encrypted HOSxP connection settings exist on this machine.
    pub configured: RwSignal<bool>,
    /// Whether the connection settings dialog is open.
    pub settings_open: RwSignal<bool>,
}

/// The one loud thing on screen — only one state at a time, and the new
/// state fully replaces the old one (DESIGN.md, verdict band rule).
///
/// The three-way split is deliberate (ROADMAP Phase 1, Gap G1): a
/// "ไม่พบประวัติ" verdict is only ever produced when the drug is known;
/// an unresolvable drug term renders as [`VerdictState::Unresolved`]
/// instead.
#[derive(Debug, Clone, Default)]
pub enum VerdictState {
    /// Query in flight / nothing searched yet — neutral gray, never implies
    /// an answer.
    #[default]
    Pending,
    /// History found; `records` are sorted most-recent-first. `truncated`
    /// is true when older history exists beyond the per-source cap — the
    /// timeline must not present itself as complete (ROADMAP Phase 1).
    Found { records: Vec<DrugHistoryRecord>, truncated: bool },
    /// History searched and definitively not found (drug resolved, no
    /// dispensing rows).
    NotFound,
    /// The drug term could not be matched to the formulary. `candidates`
    /// are the closest matches for the operator to disambiguate (empty =
    /// the term is not in `drugitems` at all). Never render "ไม่พบประวัติ"
    /// in this state.
    Unresolved { candidates: Vec<DrugItem> },
}

impl AppState {
    pub fn new() -> Self {
        Self {
            patient: RwSignal::new(None),
            verdict: RwSignal::new(VerdictState::Pending),
            configured: RwSignal::new(false),
            settings_open: RwSignal::new(false),
        }
    }
}
