//! AllerX domain types.
//!
//! This crate is the shared data model only — no sqlx, no I/O, no business
//! logic (AGENTS.md §3–§4). Every layer (connector, search-core, Tauri
//! shell, frontend) builds on these types.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// Where a drug administration happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VisitType {
    /// Outpatient visit.
    Opd,
    /// Inpatient admission (keyed by `an`).
    Ipd,
}

/// A searchable patient summary (AGENTS.md §4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatientSummary {
    pub hn: String,
    /// National ID — masked on display, see DESIGN.md.
    pub cid: Option<String>,
    pub full_name_th: String,
    pub birth_date: Option<NaiveDate>,
    pub sex: Option<String>,
}

/// One prior administration of a drug (AGENTS.md §4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DrugHistoryRecord {
    pub visit_date: NaiveDate,
    pub visit_type: VisitType,
    pub drug_code: String,
    /// Generic name.
    pub drug_name: String,
    pub trade_name: Option<String>,
    pub prescriber: Option<String>,
    pub department: Option<String>,
    pub quantity: Option<String>,
    pub route: Option<String>,
}

/// A drug item from `drugitems`, used for autocomplete (AGENTS.md §7.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DrugItem {
    pub icode: String,
    /// Display name (generic / trade as configured by the hospital).
    pub name: String,
    /// Dose strength as printed on the item, e.g. "500 mg" — helps the
    /// pharmacist pick the right presentation.
    pub strength: Option<String>,
    /// Trade/brand name — helps disambiguate candidates and match a search
    /// term the pharmacist knows by its brand (ROADMAP Phase 1).
    /// `None` when the live instance lacks the column.
    pub trade_name: Option<String>,
}

/// Complete answer to "has this patient had this drug before?" (AGENTS.md §4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DrugSearchHit {
    pub patient: PatientSummary,
    pub found: bool,
    /// Most recent first.
    pub records: Vec<DrugHistoryRecord>,
}

/// A resolved history lookup: the drug was matched to a `drugitems` icode
/// and the dispensing rows were fetched (possibly zero).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedHistory {
    /// Most recent first. Empty means "patient has no dispensing history for
    /// this drug" — a legitimate, definitive "ไม่พบประวัติ".
    pub records: Vec<DrugHistoryRecord>,
    /// True when any source hit its per-source `LIMIT`, i.e. older history
    /// exists but is not returned. The UI must not present the list as
    /// complete when this is set.
    pub truncated: bool,
}

/// The answer to "has this patient had this drug, and when?" — the backend
/// contract for the verdict band (ROADMAP Phase 1).
///
/// The two variants are deliberately distinct: a verdict of "not found" is
/// only ever produced for a **resolved** drug. When the drug term cannot be
/// mapped to a `drugitems` entry, the lookup is [`HistoryVerdict::Unresolved`]
/// and the UI must never render "ไม่พบประวัติ" (a false negative is a
/// patient-safety event).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HistoryVerdict {
    /// The drug is known; `drug` is the exact formulary entry the term
    /// resolved to (name/strength identity — the UI shows what drug a
    /// verdict refers to), and `history.records` holds the dispensing
    /// timeline (empty = genuinely never dispensed).
    Resolved {
        drug: DrugItem,
        history: ResolvedHistory,
    },
    /// The drug term could not be resolved to an icode. `candidates` are the
    /// closest `drugitems` matches for the operator to disambiguate; empty
    /// means the term is not in the formulary at all.
    Unresolved { candidates: Vec<DrugItem> },
}

/// One checked drug in a batch lookup (ROADMAP Phase 5) — the term the
/// pharmacist submitted plus that drug's verdict, so a multi-drug check
/// can label each result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DrugCheckResult {
    /// The term submitted for this drug — the UI's label for the verdict.
    pub term: String,
    /// The verdict: resolved (possibly empty) or unresolved.
    pub verdict: HistoryVerdict,
}

/// One drug dispensed to a patient recently (ROADMAP Phase 5) — the
/// "ยาที่ได้รับล่าสุด" snapshot, deduped per icode with the latest date.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConcurrentMedication {
    pub drug_code: String,
    pub drug_name: String,
    pub trade_name: Option<String>,
    pub last_date: NaiveDate,
}
