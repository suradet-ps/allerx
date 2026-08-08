//! AllerX search business logic.
//!
//! This crate owns the *contract* (the [`HosxRepository`] trait) and the
//! pure logic that must be testable without a database: input-kind
//! detection, history merging/ordering. The only crate allowed to talk to
//! MySQL is `hosxp-connector`, which implements this contract.

pub mod error;
pub mod history;
pub mod mock;
pub mod query_kind;
pub mod repository;

pub use error::RepositoryError;
pub use history::merge_drug_history;
pub use mock::MockRepository;
pub use query_kind::{QueryKind, detect_query_kind};
pub use repository::HosxRepository;
