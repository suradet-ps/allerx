//! Integration tests against a **test/staging** HOSxP instance only
//! (AGENTS.md §12). Never run against production, never part of the default
//! `cargo test` run — gate with:
//!
//! ```text
//! cargo test --features integration-tests
//! ```
//!
//! Requires the environment variable `ALLERX_TEST_DATABASE_URL` (a plain
//! `mysql://` URL for the test instance) to be set.

#![cfg(feature = "integration-tests")]

use allerx_hosxp_connector::pool;
use allerx_hosxp_connector::readonly_guard::READ_ONLY_SESSION_SQL;
use sqlx::mysql::MySqlConnectOptions;
use sqlx::{MySqlPool, MySqlPoolOptions};

fn test_url() -> String {
    std::env::var("ALLERX_TEST_DATABASE_URL")
        .expect("set ALLERX_TEST_DATABASE_URL to a test/staging HOSxP instance")
}

fn test_pool(url: &str) -> MySqlPool {
    MySqlPoolOptions::new()
        .max_connections(2)
        .connect(url)
        .expect("connect to the test instance")
}

#[tokio::test]
async fn select_one_smoke_test_round_trips() {
    let url = test_url();
    let pool = test_pool(&url);
    let row: (i32,) = sqlx::query_as("SELECT 1")
        .fetch_one(&pool)
        .await
        .expect("SELECT 1 must succeed");
    assert_eq!(row.0, 1);
}

#[tokio::test]
async fn read_only_session_rejects_dml() {
    let url = test_url();
    let pool = test_pool(&url);
    let mut conn = pool.acquire().await.expect("acquire a connection");
    sqlx::query(READ_ONLY_SESSION_SQL)
        .execute(&mut *conn)
        .await
        .expect("SET SESSION TRANSACTION READ ONLY must succeed");

    // The session must refuse to create anything — proof the connector's
    // session mode really blocks writes (AGENTS.md §5.2).
    let result = sqlx::query("CREATE TEMPORARY TABLE _allerx_guard_probe (id INT)")
        .execute(&mut *conn)
        .await;
    assert!(result.is_err(), "read-only session must reject DDL");
}

#[tokio::test]
async fn connector_pool_applies_read_only_mode() {
    // Uses the connector's own pool builder (after_connect hook), then
    // verifies the session really rejects a write.
    let url = test_url();
    let options = MySqlConnectOptions::from_url(&url).expect("parse test URL");
    let cfg = allerx_hosxp_connector::config::HosxConfig::new(
        options.get_host().to_string(),
        options.get_port(),
        options.get_database().expect("database in URL").to_string(),
        options.get_username().expect("user in URL").to_string(),
        options.get_password().unwrap_or_default(),
    );
    let pool = pool::connect(&cfg).await.expect("connect via connector");
    let mut conn = pool.acquire().await.expect("acquire from connector pool");

    let result = sqlx::query("CREATE TEMPORARY TABLE _allerx_guard_probe (id INT)")
        .execute(&mut *conn)
        .await;
    assert!(
        result.is_err(),
        "connector pool must be read-only end to end"
    );
}
