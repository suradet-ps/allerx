//! In-memory, PII-free query timing ring buffer (ROADMAP Phase 2).
//!
//! Timing is diagnostic data, not patient data: command names, durations,
//! and an outcome flag only — **never** parameter values (HN, CID, names;
//! AGENTS.md §2). Nothing is persisted and nothing leaves this process.
//!
//! The buffer holds the last [`QueryStats::CAPACITY`] samples, so a dev or
//! an ops person can answer "how slow were the last N queries" without the
//! app ever touching a log file.

use std::collections::VecDeque;
use std::time::Instant;

use serde::Serialize;

/// Number of samples kept — a busy shift's recent queries; older samples
/// are evicted (newest first).
const CAPACITY: usize = 200;

/// One measured command invocation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuerySample {
    /// The command/source, e.g. `search_patients`, `fetch_drug_history`,
    /// `warm_up_pool`.
    pub command: String,
    /// Milliseconds since app launch (stats buffer creation).
    pub at_ms: u64,
    /// End-to-end duration of the command, including pool acquisition.
    pub elapsed_ms: u64,
    /// Whether the command returned `Ok`.
    pub ok: bool,
}

/// The ring buffer backing the `query_stats` Tauri command.
#[derive(Debug, Clone)]
pub struct QueryStats {
    buffer: VecDeque<QuerySample>,
    started: Instant,
}

impl Default for QueryStats {
    fn default() -> Self {
        Self::new()
    }
}

impl QueryStats {
    /// Creates an empty buffer; `at_ms` timestamps count from now.
    pub fn new() -> Self {
        Self {
            buffer: VecDeque::with_capacity(CAPACITY),
            started: Instant::now(),
        }
    }

    /// Records one sample, evicting the oldest when full.
    pub fn record(&mut self, command: &str, elapsed_ms: u64, ok: bool) {
        if self.buffer.len() == CAPACITY {
            self.buffer.pop_front();
        }
        self.buffer.push_back(QuerySample {
            command: command.to_string(),
            at_ms: self.started.elapsed().as_millis() as u64,
            elapsed_ms,
            ok,
        });
    }

    /// Clones the buffered samples (newest last) for serialization.
    pub fn snapshot(&self) -> Vec<QuerySample> {
        self.buffer.iter().cloned().collect()
    }

    /// Drops every sample — useful before starting a measurement session.
    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_in_order_with_timestamps() {
        let mut stats = QueryStats::new();
        stats.record("search_patients", 42, true);
        stats.record("fetch_drug_history", 133, false);

        let samples = stats.snapshot();
        assert_eq!(samples.len(), 2);
        assert_eq!(samples[0].command, "search_patients");
        assert_eq!(samples[0].elapsed_ms, 42);
        assert!(samples[0].ok);
        assert!(samples[0].at_ms <= samples[1].at_ms);
        assert_eq!(samples[1].command, "fetch_drug_history");
        assert_eq!(samples[1].elapsed_ms, 133);
        assert!(!samples[1].ok);
    }

    #[test]
    fn evicts_oldest_when_full() {
        let mut stats = QueryStats::new();
        for i in 0..(CAPACITY + 10) {
            stats.record("ping", i as u64, true);
        }
        let samples = stats.snapshot();
        assert_eq!(samples.len(), CAPACITY);
        // The first 10 samples were evicted; the newest is the last one.
        assert_eq!(samples[0].elapsed_ms, 10);
        assert_eq!(samples[CAPACITY - 1].elapsed_ms, (CAPACITY + 9) as u64);
    }

    #[test]
    fn clear_drops_everything() {
        let mut stats = QueryStats::new();
        stats.record("ping", 1, true);
        stats.clear();
        assert!(stats.snapshot().is_empty());
    }

    #[test]
    fn snapshot_is_a_clone_not_a_borrow() {
        let mut stats = QueryStats::new();
        stats.record("ping", 1, true);
        let first = stats.snapshot();
        stats.record("ping", 2, true);
        let second = stats.snapshot();
        assert_ne!(first.len(), second.len());
    }
}
