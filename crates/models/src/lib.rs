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
}

/// Complete answer to "has this patient had this drug before?" (AGENTS.md §4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DrugSearchHit {
    pub patient: PatientSummary,
    pub found: bool,
    /// Most recent first.
    pub records: Vec<DrugHistoryRecord>,
}
