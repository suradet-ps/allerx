//! Defense-in-depth SQL guard (AGENTS.md §5.3).
//!
//! This is **not** the security boundary — DB user grants and session
//! read-only mode are. This layer exists to catch mistakes early: any
//! outgoing statement that is not a single `SELECT`/`WITH` is rejected.

/// Statement executed on every new pooled connection to make the session
/// reject any DML (AGENTS.md §5.2).
pub const READ_ONLY_SESSION_SQL: &str = "SET SESSION TRANSACTION READ ONLY";

/// The M0 smoke test statement.
pub const PING_SQL: &str = "SELECT 1";

/// The guard rejected a statement.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("rejected non-read-only SQL statement")]
pub struct GuardError;

/// Accepts only a single read statement, case-insensitively, after leading
/// SQL comments. Anything else — `INSERT`, `UPDATE`, `DELETE`, DDL, multiple
/// statements — is rejected.
///
/// # Examples
///
/// ```
/// use allerx_hosxp_connector::readonly_guard::assert_read_only;
///
/// assert!(assert_read_only("SELECT 1").is_ok());
/// assert!(assert_read_only("with x as (select 1) select * from x").is_ok());
/// assert!(assert_read_only("  -- note\nSELECT 1").is_ok());
/// assert!(assert_read_only("DELETE FROM patient").is_err());
/// ```
pub fn assert_read_only(sql: &str) -> Result<(), GuardError> {
    let Some(rest) = strip_leading_comments(sql) else {
        return Err(GuardError);
    };
    let rest = rest.trim_start();
    let keyword_end = rest
        .find(|c: char| !c.is_ascii_alphanumeric())
        .unwrap_or(rest.len());
    let keyword = rest[..keyword_end].to_ascii_uppercase();
    let followed_by_boundary = rest[keyword_end..]
        .chars()
        .next()
        .is_some_and(|c| c.is_whitespace() || c == '(');
    let single_statement = !has_semicolon_outside_quotes(rest);

    if (keyword == "SELECT" || keyword == "WITH") && followed_by_boundary && single_statement {
        Ok(())
    } else {
        Err(GuardError)
    }
}

/// True when the statement contains a `;` outside string literals (`'`/`"`)
/// or quoted identifiers (backticks) — i.e. multiple statements, or a
/// stray trailing semicolon. Failing closed is the safe direction here.
fn has_semicolon_outside_quotes(sql: &str) -> bool {
    let mut quote = None;
    for c in sql.chars() {
        if let Some(active) = quote {
            if c == active {
                quote = None;
            }
        } else if matches!(c, '\'' | '"' | '`') {
            quote = Some(c);
        } else if c == ';' {
            return true;
        }
    }
    false
}

/// Skips leading `--` line comments and `/* ... */` block comments,
/// returning the first non-comment prefix, or `None` if an unterminated
/// block comment swallows the rest of the statement.
fn strip_leading_comments(mut sql: &str) -> Option<&str> {
    loop {
        sql = sql.trim_start();
        if let Some(rest) = sql.strip_prefix("--") {
            sql = rest.split_once('\n').map(|(_, after)| after).unwrap_or("");
        } else if let Some(rest) = sql.strip_prefix("/*") {
            sql = &rest[rest.find("*/")? + 2..];
        } else {
            return Some(sql);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_select_variants() {
        assert!(assert_read_only("SELECT 1").is_ok());
        assert!(assert_read_only("select * from patient").is_ok());
        assert!(assert_read_only("  SELECT hn FROM patient").is_ok());
        assert!(assert_read_only("SELECT\nvstdate FROM opitemrece").is_ok());
        assert!(assert_read_only("SELECT(1)").is_ok());
    }

    #[test]
    fn accepts_with_variants() {
        assert!(assert_read_only("WITH x AS (SELECT 1) SELECT * FROM x").is_ok());
        assert!(assert_read_only("with x as (select 1) select * from x").is_ok());
    }

    #[test]
    fn accepts_statements_with_leading_comments() {
        assert!(assert_read_only("-- generated query\nSELECT 1").is_ok());
        assert!(assert_read_only("/* header */ SELECT 1").is_ok());
        assert!(assert_read_only("-- first\n-- second\nSELECT 1").is_ok());
    }

    #[test]
    fn rejects_write_and_ddl_statements() {
        for sql in [
            "INSERT INTO patient (hn) VALUES ('x')",
            "UPDATE patient SET fname = 'x'",
            "DELETE FROM patient",
            "DROP TABLE patient",
            "ALTER TABLE patient ADD COLUMN x INT",
            "CREATE TABLE x (id INT)",
            "GRANT SELECT ON *.* TO 'u'",
            "TRUNCATE TABLE patient",
        ] {
            assert!(assert_read_only(sql).is_err(), "should reject: {sql}");
        }
    }

    #[test]
    fn rejects_keyword_prefix_masquerading_as_select() {
        assert!(assert_read_only("SELECTED 1").is_err());
        assert!(assert_read_only("SELECTION").is_err());
    }

    #[test]
    fn rejects_empty_and_whitespace_only_input() {
        assert!(assert_read_only("").is_err());
        assert!(assert_read_only("   ").is_err());
    }

    #[test]
    fn rejects_multiple_statements_in_one_string() {
        assert!(assert_read_only("SELECT 1; DELETE FROM patient").is_err());
        assert!(assert_read_only("SELECT 1;").is_err());
    }

    #[test]
    fn rejects_unterminated_block_comment() {
        assert!(assert_read_only("/* never closed SELECT 1").is_err());
    }
}
