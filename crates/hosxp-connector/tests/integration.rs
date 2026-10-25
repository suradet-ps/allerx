//! Integration tests against a **test/staging** HOSxP instance only
//! (AGENTS.md §12). Never run against production, never part of the default
//! `cargo test` run - gate with:
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
use secrecy::SecretString;
use sqlx::MySqlPool;
use sqlx::mysql::MySqlPoolOptions;

fn test_url() -> String {
    std::env::var("ALLERX_TEST_DATABASE_URL")
        .expect("set ALLERX_TEST_DATABASE_URL to a test/staging HOSxP instance")
}

async fn test_pool(url: &str) -> MySqlPool {
    MySqlPoolOptions::new()
        .max_connections(2)
        .connect(url)
        .await
        .expect("connect to the test instance")
}

#[tokio::test]
async fn select_one_smoke_test_round_trips() {
    let url = test_url();
    let pool = test_pool(&url).await;
    let row: (i32,) = sqlx::query_as("SELECT 1")
        .fetch_one(&pool)
        .await
        .expect("SELECT 1 must succeed");
    assert_eq!(row.0, 1);
}

#[tokio::test]
async fn read_only_session_rejects_dml() {
    let url = test_url();
    let pool = test_pool(&url).await;
    let mut conn = pool.acquire().await.expect("acquire a connection");
    sqlx::query(READ_ONLY_SESSION_SQL)
        .execute(&mut *conn)
        .await
        .expect("SET SESSION TRANSACTION READ ONLY must succeed");

    // The session must refuse to create anything - proof the connector's
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
    // `MySqlConnectOptions` no longer exposes the parsed password, so the
    // URL is parsed directly to build the same config the connector takes.
    // Credentials are percent-decoded because `Url` keeps them encoded,
    // while sqlx decodes them when connecting from the URL.
    let parsed = url::Url::parse(&url).expect("valid mysql:// test URL");
    let decode = |raw: &str| {
        percent_encoding::percent_decode_str(raw)
            .decode_utf8()
            .expect("URL credentials must be valid UTF-8")
            .into_owned()
    };
    let cfg = allerx_hosxp_connector::config::HosxConfig::new(
        parsed.host_str().expect("host in URL").to_string(),
        parsed.port().unwrap_or(3306),
        parsed.path().trim_start_matches('/').to_string(),
        decode(parsed.username()),
        SecretString::from(decode(parsed.password().unwrap_or_default())),
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
