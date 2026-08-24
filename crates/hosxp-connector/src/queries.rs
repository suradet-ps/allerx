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

/// Drug autocomplete — typed tier: icode/name/trade-name matching plus the
/// drug-type filter (AGENTS.md §6.2), used first so autocomplete never
/// offers non-drug items and an icode search surfaces the drug with its
/// name and strength (pilot feedback: "1000152 → ไม่พบประวัติ" without
/// any drug identity).
///
/// // SCHEMA-UNVERIFIED: `drugitems.trade_name` and the `istype = '1'`
/// // type filter per AGENTS.md §6 — confirm both against the live instance
/// // (some instances call the type field `item_type = 'MED'`). If either
/// // column is absent the instance fails with 1054 and repository.rs falls
/// // back to [`DRUG_SEARCH_PREFIX_TRADE`] / [`DRUG_SEARCH_PREFIX_PLAIN`].
pub const DRUG_SEARCH_PREFIX_TYPED: &str = "SELECT icode, name, strength, trade_name \
     FROM drugitems WHERE (name LIKE ? OR trade_name LIKE ? OR icode LIKE ?) AND istype = '1' \
     ORDER BY name LIMIT 20";

/// Drug autocomplete — trade-name tier: same as typed but without the type
/// filter (used when the instance lacks `istype`/`item_type`).
///
/// // SCHEMA-UNVERIFIED: `drugitems.trade_name` per AGENTS.md §6.
pub const DRUG_SEARCH_PREFIX_TRADE: &str = "SELECT icode, name, strength, trade_name \
     FROM drugitems WHERE (name LIKE ? OR icode LIKE ?) ORDER BY name LIMIT 20";

/// Drug autocomplete — plain tier: no trade-name column, no type filter
/// (the safe baseline every instance supports).
pub const DRUG_SEARCH_PREFIX_PLAIN: &str = "SELECT icode, name, strength, NULL AS trade_name \
     FROM drugitems WHERE (name LIKE ? OR icode LIKE ?) ORDER BY name LIMIT 20";

/// Drug autocomplete fallback (contains-match) — typed tier.
///
/// // SCHEMA-UNVERIFIED: `drugitems.trade_name` and `istype` per AGENTS.md §6.
pub const DRUG_SEARCH_CONTAINS_TYPED: &str = "SELECT icode, name, strength, trade_name \
     FROM drugitems WHERE (name LIKE ? OR trade_name LIKE ? OR icode LIKE ?) AND istype = '1' \
     ORDER BY name LIMIT 20";

/// Drug autocomplete fallback (contains-match) — trade-name tier.
///
/// // SCHEMA-UNVERIFIED: `drugitems.trade_name` per AGENTS.md §6.
pub const DRUG_SEARCH_CONTAINS_TRADE: &str = "SELECT icode, name, strength, trade_name \
     FROM drugitems WHERE (name LIKE ? OR icode LIKE ?) ORDER BY name LIMIT 20";

/// Drug autocomplete fallback (contains-match) — plain tier.
pub const DRUG_SEARCH_CONTAINS_PLAIN: &str = "SELECT icode, name, strength, NULL AS trade_name \
     FROM drugitems WHERE (name LIKE ? OR icode LIKE ?) ORDER BY name LIMIT 20";

/// Resolves a drug term to its `drugitems` row — exact icode hit first.
/// Returns name/strength so the verdict can label which drug it refers to.
///
/// // SCHEMA-UNVERIFIED: `drugitems.trade_name` per AGENTS.md §6 — the
/// // fallback (`NULL AS trade_name`) covers instances without the column.
pub const DRUG_RESOLVE_BY_ICODE_TRADE: &str =
    "SELECT icode, name, strength, trade_name FROM drugitems WHERE icode = ? LIMIT 1";

/// Exact icode hit without the trade-name column — same shape.
pub const DRUG_RESOLVE_BY_ICODE: &str =
    "SELECT icode, name, strength, NULL AS trade_name FROM drugitems WHERE icode = ? LIMIT 1";

/// Resolves a drug term to its `drugitems` row — exact display-name hit
/// second.
///
/// // SCHEMA-UNVERIFIED: `drugitems.trade_name` per AGENTS.md §6.
pub const DRUG_RESOLVE_BY_NAME_TRADE: &str =
    "SELECT icode, name, strength, trade_name FROM drugitems WHERE name = ? LIMIT 1";

/// Exact display-name hit without the trade-name column — same shape.
pub const DRUG_RESOLVE_BY_NAME: &str =
    "SELECT icode, name, strength, NULL AS trade_name FROM drugitems WHERE name = ? LIMIT 1";

/// Resolves a drug term to its `drugitems` row — exact trade-name hit third
/// (ROADMAP Phase 1, trade-name search).
///
/// // SCHEMA-UNVERIFIED: `drugitems.trade_name` per AGENTS.md §6 — a missing
/// // column is tolerated at runtime (treated as "no trade-name match").
pub const DRUG_RESOLVE_BY_TRADE_NAME: &str =
    "SELECT icode, name, strength, trade_name FROM drugitems WHERE trade_name = ? LIMIT 1";

/// OPD dispensing history — `opitemrece` rows without an admission number
/// (AGENTS.md §6.2; reference join from §6.6). Selects the trade name when
/// the column exists; [`HISTORY_OPD_FALLBACK`] covers instances without it.
///
/// `o.dep_code` confirmed against the live instance (some others use
/// `depcode`); `kskdepartment.depcode` still unverified.
///
/// // SCHEMA-UNVERIFIED: `drugitems.trade_name` per AGENTS.md §6.
pub const HISTORY_OPD: &str = "SELECT o.vstdate, o.icode, d.name, d.strength, d.trade_name, doc.name AS prescriber, dep.department, u.name1 AS route, CAST(o.qty AS CHAR) AS quantity \
     FROM opitemrece o \
     INNER JOIN drugitems d ON o.icode = d.icode \
     LEFT JOIN doctor doc ON o.doctor = doc.code \
     LEFT JOIN kskdepartment dep ON o.dep_code = dep.depcode \
     LEFT JOIN drugusage u ON o.drugusage = u.drugusage \
     WHERE o.hn = ? AND o.icode = ? AND o.qty > 0 AND o.an IS NULL \
     ORDER BY o.vstdate DESC, o.vsttime DESC \
     LIMIT 200";

/// OPD history without the trade-name column — same result shape, `NULL`
/// in its place (used when the instance fails [`HISTORY_OPD`] with 1054).
pub const HISTORY_OPD_FALLBACK: &str = "SELECT o.vstdate, o.icode, d.name, NULL AS strength, NULL AS trade_name, doc.name AS prescriber, dep.department, u.name1 AS route, CAST(o.qty AS CHAR) AS quantity \
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
///
/// // SCHEMA-UNVERIFIED: `drugitems.trade_name` per AGENTS.md §6.
pub const HISTORY_IPD_TAKEHOME: &str = "SELECT o.vstdate, o.icode, d.name, d.strength, d.trade_name, doc.name AS prescriber, dep.department, u.name1 AS route, CAST(o.qty AS CHAR) AS quantity \
     FROM opitemrece o \
     INNER JOIN drugitems d ON o.icode = d.icode \
     LEFT JOIN doctor doc ON o.doctor = doc.code \
     LEFT JOIN kskdepartment dep ON o.dep_code = dep.depcode \
     LEFT JOIN drugusage u ON o.drugusage = u.drugusage \
     WHERE o.hn = ? AND o.icode = ? AND o.qty > 0 AND o.an IS NOT NULL \
     ORDER BY o.vstdate DESC, o.vsttime DESC \
     LIMIT 200";

/// IPD take-home history without the trade-name column — same result shape.
pub const HISTORY_IPD_TAKEHOME_FALLBACK: &str = "SELECT o.vstdate, o.icode, d.name, NULL AS strength, NULL AS trade_name, doc.name AS prescriber, dep.department, u.name1 AS route, CAST(o.qty AS CHAR) AS quantity \
     FROM opitemrece o \
     INNER JOIN drugitems d ON o.icode = d.icode \
     LEFT JOIN doctor doc ON o.doctor = doc.code \
     LEFT JOIN kskdepartment dep ON o.dep_code = dep.depcode \
     LEFT JOIN drugusage u ON o.drugusage = u.drugusage \
     WHERE o.hn = ? AND o.icode = ? AND o.qty > 0 AND o.an IS NOT NULL \
     ORDER BY o.vstdate DESC, o.vsttime DESC \
     LIMIT 200";

/// IPD in-stay dispensing — `iptitemrece` (table naming varies by hospital,
/// AGENTS.md §6.3), keyed by admission number via `ipt`. Selects the trade
/// name when the column exists; [`HISTORY_IPD_STAY`] covers the rest.
///
/// // SCHEMA-UNVERIFIED: `iptitemrece.idate/itime` and `ipt.hn` per
/// // AGENTS.md §6.3 — confirm the table name and date column on the live
/// // instance. A missing table (named differently) is tolerated at runtime
/// // and treated as "no in-stay records" (see repository.rs).
pub const HISTORY_IPD_STAY_TRADE: &str = "SELECT i.idate, i.icode, d.name, d.strength, d.trade_name, doc.name AS prescriber, dep.department, u.name1 AS route, CAST(i.qty AS CHAR) AS quantity \
     FROM iptitemrece i \
     INNER JOIN ipt ON i.an = ipt.an \
     INNER JOIN drugitems d ON i.icode = d.icode \
     LEFT JOIN doctor doc ON i.doctor = doc.code \
     LEFT JOIN kskdepartment dep ON i.depcode = dep.depcode \
     LEFT JOIN drugusage u ON i.drugusage = u.drugusage \
     WHERE ipt.hn = ? AND i.icode = ? AND i.qty > 0 \
     ORDER BY i.idate DESC, i.itime DESC \
     LIMIT 200";

/// IPD in-stay history without the trade-name column — same result shape.
pub const HISTORY_IPD_STAY: &str = "SELECT i.idate, i.icode, d.name, NULL AS strength, NULL AS trade_name, doc.name AS prescriber, dep.department, u.name1 AS route, CAST(i.qty AS CHAR) AS quantity \
     FROM iptitemrece i \
     INNER JOIN ipt ON i.an = ipt.an \
     INNER JOIN drugitems d ON i.icode = d.icode \
     LEFT JOIN doctor doc ON i.doctor = doc.code \
     LEFT JOIN kskdepartment dep ON i.depcode = dep.depcode \
     LEFT JOIN drugusage u ON i.drugusage = u.drugusage \
     WHERE ipt.hn = ? AND i.icode = ? AND i.qty > 0 \
     ORDER BY i.idate DESC, i.itime DESC \
     LIMIT 200";

/// Recent concurrent medications — dispensing rows in the last 30 days,
/// deduped per icode with the latest date (ROADMAP Phase 5). Filtered to
/// the drug category per AGENTS.md §6.2.
///
/// // SCHEMA-UNVERIFIED: `drugitems.trade_name` and the `'1%'` drug-category
/// // assumption per AGENTS.md §6 — confirm against the live instance. The
/// // trade-name column degrades via [`CONCURRENT_MEDS`] on 1054.
pub const CONCURRENT_MEDS_TRADE: &str = "SELECT o.icode, MAX(o.vstdate) AS last_date, d.name, d.strength, d.trade_name \
     FROM opitemrece o \
     INNER JOIN drugitems d ON o.icode = d.icode \
     WHERE o.hn = ? AND o.qty > 0 AND d.icode LIKE '1%' \
       AND o.vstdate >= DATE_SUB(CURDATE(), INTERVAL 30 DAY) \
     GROUP BY o.icode, d.name, d.strength, d.trade_name \
     ORDER BY last_date DESC, o.icode \
     LIMIT 30";

/// Recent concurrent medications without the trade-name column — same shape.
pub const CONCURRENT_MEDS: &str = "SELECT o.icode, MAX(o.vstdate) AS last_date, d.name, NULL AS strength, NULL AS trade_name \
     FROM opitemrece o \
     INNER JOIN drugitems d ON o.icode = d.icode \
     WHERE o.hn = ? AND o.qty > 0 AND d.icode LIKE '1%' \
       AND o.vstdate >= DATE_SUB(CURDATE(), INTERVAL 30 DAY) \
     GROUP BY o.icode, d.name, d.strength \
     ORDER BY last_date DESC, o.icode \
     LIMIT 30";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::readonly_guard::assert_read_only;

    /// Every statement the crate can execute. Both guard tests iterate this
    /// list — add new constants here.
    const ALL_STATEMENTS: [&str; 23] = [
        PATIENT_SEARCH_BY_CID,
        PATIENT_SEARCH_BY_HN,
        PATIENT_SEARCH_NAME_PREFIX,
        PATIENT_SEARCH_NAME_CONTAINS,
        DRUG_SEARCH_PREFIX_TYPED,
        DRUG_SEARCH_PREFIX_TRADE,
        DRUG_SEARCH_PREFIX_PLAIN,
        DRUG_SEARCH_CONTAINS_TYPED,
        DRUG_SEARCH_CONTAINS_TRADE,
        DRUG_SEARCH_CONTAINS_PLAIN,
        DRUG_RESOLVE_BY_ICODE,
        DRUG_RESOLVE_BY_ICODE_TRADE,
        DRUG_RESOLVE_BY_NAME,
        DRUG_RESOLVE_BY_NAME_TRADE,
        DRUG_RESOLVE_BY_TRADE_NAME,
        HISTORY_OPD,
        HISTORY_OPD_FALLBACK,
        HISTORY_IPD_TAKEHOME,
        HISTORY_IPD_TAKEHOME_FALLBACK,
        HISTORY_IPD_STAY,
        HISTORY_IPD_STAY_TRADE,
        CONCURRENT_MEDS,
        CONCURRENT_MEDS_TRADE,
    ];

    /// The table surface documented in docs/deployment.md Part A ("Which
    /// tables does AllerX read?") — the DBA grants are written against it.
    const DOCUMENTED_TABLES: [&str; 8] = [
        "patient",
        "drugitems",
        "opitemrece",
        "iptitemrece",
        "ipt",
        "doctor",
        "kskdepartment",
        "drugusage",
    ];

    #[test]
    fn all_statements_are_read_only() {
        for sql in ALL_STATEMENTS {
            assert!(assert_read_only(sql).is_ok(), "must be SELECT: {sql}");
        }
    }

    /// Identifiers immediately following a standalone `FROM` or `JOIN`
    /// keyword — the tables a statement reads or joins.
    fn referenced_tables(sql: &str) -> Vec<String> {
        let lower = sql.to_ascii_lowercase();
        let bytes = lower.as_bytes();
        let mut tables = Vec::new();
        for keyword in ["from", "join"] {
            let mut start = 0;
            while let Some(rel) = lower[start..].find(keyword) {
                let at = start + rel;
                let end = at + keyword.len();
                let word_bounded = (at == 0 || !bytes[at - 1].is_ascii_alphanumeric())
                    && (end == bytes.len() || !bytes[end].is_ascii_alphanumeric());
                if word_bounded {
                    let ident: String = lower[end..]
                        .chars()
                        .skip_while(|c| c.is_whitespace())
                        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                        .collect();
                    if !ident.is_empty() {
                        tables.push(ident);
                    }
                }
                start = end;
            }
        }
        tables
    }

    #[test]
    fn every_referenced_table_is_on_the_documented_access_surface() {
        let mut seen: Vec<String> = Vec::new();
        for sql in ALL_STATEMENTS {
            for table in referenced_tables(sql) {
                assert!(
                    DOCUMENTED_TABLES.contains(&table.as_str()),
                    "table `{table}` is not in docs/deployment.md Part A \
                     (\"Which tables does AllerX read?\") — update that \
                     section and the DBA grants in the same change"
                );
                if !seen.iter().any(|s| s == &table) {
                    seen.push(table);
                }
            }
        }
        for documented in DOCUMENTED_TABLES {
            assert!(
                seen.iter().any(|s| s == documented),
                "table `{documented}` is documented but no longer referenced \
                 by any statement — drop its grant row from deployment.md"
            );
        }
    }
}
