//! Every SQL statement this crate may execute — compile-time constants only.
//!
//! Runtime values (HN, CID, names) are always bound as sqlx parameters,
//! never interpolated (AGENTS-RUST.md §14 override). Every statement here
//! must pass [`assert_read_only`] — enforced by the test at the bottom of
//! this module and by the guard on every execution path.

/// Patient lookup by exact national ID (13 digits) — AGENTS.md §6.1.
///
/// Columns confirmed against the live instance (`birthday`, not the
/// `birthdate` variant found on some other instances).
pub const PATIENT_SEARCH_BY_CID: &str = "SELECT hn, cid, CONCAT_WS(' ', pname, fname, lname) AS full_name_th, birthday AS birth_date, sex \
     FROM patient WHERE cid = ? LIMIT 20";

/// Patient lookup by exact hospital HN — AGENTS.md §6.1.
///
/// Columns confirmed against the live instance.
pub const PATIENT_SEARCH_BY_HN: &str = "SELECT hn, cid, CONCAT_WS(' ', pname, fname, lname) AS full_name_th, birthday AS birth_date, sex \
     FROM patient WHERE hn = ? LIMIT 20";

/// Name search, prefix-match first so existing indexes on `fname`/`lname`
/// are used (AGENTS.md §7.1); falls back to [`PATIENT_SEARCH_NAME_CONTAINS`]
/// when empty.
///
/// Columns confirmed against the live instance.
pub const PATIENT_SEARCH_NAME_PREFIX: &str = "SELECT hn, cid, CONCAT_WS(' ', pname, fname, lname) AS full_name_th, birthday AS birth_date, sex \
     FROM patient \
     WHERE fname LIKE ? OR lname LIKE ? OR CONCAT_WS(' ', pname, fname, lname) LIKE ? \
     LIMIT 20";
/// Name search fallback — contains-match used only when the prefix match
/// found nothing (AGENTS.md §7.1).
///
/// Columns confirmed against the live instance.
pub const PATIENT_SEARCH_NAME_CONTAINS: &str = "SELECT hn, cid, CONCAT_WS(' ', pname, fname, lname) AS full_name_th, birthday AS birth_date, sex \
     FROM patient \
     WHERE fname LIKE ? OR lname LIKE ? OR CONCAT_WS(' ', pname, fname, lname) LIKE ? \
     LIMIT 20";

/// Drug autocomplete — prefix match on the display name (AGENTS.md §7.2).
///
/// // SCHEMA-UNVERIFIED: `drugitems.name` and `drugitems.strength` per
/// // AGENTS.md §6 — confirm both against the live instance. Trade-name
/// // columns are intentionally not used yet.
pub const DRUG_SEARCH_PREFIX: &str =
    "SELECT icode, name, strength FROM drugitems WHERE name LIKE ? ORDER BY name LIMIT 20";

/// Drug autocomplete fallback — contains-match used only when the prefix
/// match found nothing.
///
/// // SCHEMA-UNVERIFIED: `drugitems.name` and `drugitems.strength` per
/// // AGENTS.md §6 — confirm against the live instance.
pub const DRUG_SEARCH_CONTAINS: &str =
    "SELECT icode, name, strength FROM drugitems WHERE name LIKE ? ORDER BY name LIMIT 20";

/// Resolves a drug term to its `icode` — exact icode hit first.
pub const DRUG_RESOLVE_BY_ICODE: &str = "SELECT icode FROM drugitems WHERE icode = ? LIMIT 1";

/// Resolves a drug term to its `icode` — exact display-name hit second.
pub const DRUG_RESOLVE_BY_NAME: &str = "SELECT icode FROM drugitems WHERE name = ? LIMIT 1";

/// OPD dispensing history — `opitemrece` rows without an admission number
/// (AGENTS.md §6.2; reference join from §6.6).
///
/// `o.dep_code` confirmed against the live instance (some others use
/// `depcode`); `kskdepartment.depcode` still unverified.
pub const HISTORY_OPD: &str = "SELECT o.vstdate, o.icode, d.name, doc.name AS prescriber, dep.department, u.name1 AS route, CAST(o.qty AS CHAR) AS quantity \
     FROM opitemrece o \
     INNER JOIN drugitems d ON o.icode = d.icode \
     LEFT JOIN doctor doc ON o.doctor = doc.code \
     LEFT JOIN kskdepartment dep ON o.dep_code = dep.depcode \
     LEFT JOIN drugusage u ON o.drugusage = u.drugusage \
     WHERE o.hn = ? AND o.icode = ? AND o.qty > 0 AND o.an IS NULL \
     ORDER BY o.vstdate DESC, o.vsttime DESC \
     LIMIT 200";

/// IPD discharge/take-home medication — `opitemrece` rows carrying an
/// admission number instead of `vn` (AGENTS.md §6.3).
///
/// `o.dep_code` confirmed against the live instance; the `an IS NOT NULL`
/// branch must be confirmed on the live instance.
pub const HISTORY_IPD_TAKEHOME: &str = "SELECT o.vstdate, o.icode, d.name, doc.name AS prescriber, dep.department, u.name1 AS route, CAST(o.qty AS CHAR) AS quantity \
     FROM opitemrece o \
     INNER JOIN drugitems d ON o.icode = d.icode \
     LEFT JOIN doctor doc ON o.doctor = doc.code \
     LEFT JOIN kskdepartment dep ON o.dep_code = dep.depcode \
     LEFT JOIN drugusage u ON o.drugusage = u.drugusage \
     WHERE o.hn = ? AND o.icode = ? AND o.qty > 0 AND o.an IS NOT NULL \
     ORDER BY o.vstdate DESC, o.vsttime DESC \
     LIMIT 200";

/// IPD in-stay dispensing — `iptitemrece` (table naming varies by hospital,
/// AGENTS.md §6.3), keyed by admission number via `ipt`.
///
/// // SCHEMA-UNVERIFIED: `iptitemrece.idate/itime` and `ipt.hn` per
/// // AGENTS.md §6.3 — confirm the table name and date column on the live
/// // instance. A missing table (named differently) is tolerated at runtime
/// // and treated as "no in-stay records" (see repository.rs).
pub const HISTORY_IPD_STAY: &str = "SELECT i.idate, i.icode, d.name, doc.name AS prescriber, dep.department, u.name1 AS route, CAST(i.qty AS CHAR) AS quantity \
     FROM iptitemrece i \
     INNER JOIN ipt ON i.an = ipt.an \
     INNER JOIN drugitems d ON i.icode = d.icode \
     LEFT JOIN doctor doc ON i.doctor = doc.code \
     LEFT JOIN kskdepartment dep ON i.depcode = dep.depcode \
     LEFT JOIN drugusage u ON i.drugusage = u.drugusage \
     WHERE ipt.hn = ? AND i.icode = ? AND i.qty > 0 \
     ORDER BY i.idate DESC, i.itime DESC \
     LIMIT 200";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::readonly_guard::assert_read_only;

    #[test]
    fn all_patient_search_statements_are_read_only() {
        for sql in [
            PATIENT_SEARCH_BY_CID,
            PATIENT_SEARCH_BY_HN,
            PATIENT_SEARCH_NAME_PREFIX,
            PATIENT_SEARCH_NAME_CONTAINS,
            DRUG_SEARCH_PREFIX,
            DRUG_SEARCH_CONTAINS,
            DRUG_RESOLVE_BY_ICODE,
            DRUG_RESOLVE_BY_NAME,
            HISTORY_OPD,
            HISTORY_IPD_TAKEHOME,
            HISTORY_IPD_STAY,
        ] {
            assert!(assert_read_only(sql).is_ok(), "must be SELECT: {sql}");
        }
    }
}
