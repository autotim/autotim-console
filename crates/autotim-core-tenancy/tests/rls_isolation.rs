//! Row-Level Security integration test for `users` (doc 12, doc 61
//! §"Security & Correctness Tests" — "Tenant isolation: queries and
//! APIs cannot read/write across organization_id; RLS holds even
//! with a deliberately wrong query").
//!
//! ## Why `#[ignore]`
//!
//! This test opens a real connection to Postgres. GitHub Actions CI
//! does not run a Postgres service yet (no infrastructure for it
//! exists in `ci.yml` as of this commit), so this test would fail
//! every push with a connection error, not a real signal. It is
//! `#[ignore]`d and run explicitly, locally, against a real dev
//! database:
//!
//! ```text
//! cargo test -p autotim-core-tenancy -- --ignored
//! ```
//!
//! Adding a Postgres service to CI is a separate decision, deferred
//! to the commit that wires a real connection pool into the kernel
//! (doc 11 boot step 2) — at that point many tests will need
//! Postgres, not just this one, and the CI change is worth doing
//! once for all of them together.
//!
//! ## Connection
//!
//! Connects as `autotim_app`, the non-superuser role created during
//! manual RLS verification (doc 12 §"Row Level Security" —
//! "Application connections run as a role subject to RLS"). The
//! `bandit` superuser role used elsewhere on this host would bypass
//! RLS unconditionally and silently validate nothing.
//!
//! The connection string is read from `AUTOTIM_TEST_DATABASE_URL` so
//! the credential doesn't need to be hardcoded in source; falls back
//! to the same dev value used in `autotim-deployment`'s
//! `environments/dev/config.toml` if the variable is unset, since
//! this test only ever runs locally by a developer who already has
//! that database.

use sqlx::PgPool;
use uuid::Uuid;

const DEFAULT_DEV_URL: &str = "postgres://autotim_app:dev_only_change_me@localhost/autotim_dev";

const DEFAULT_ORG: &str = "00000000-0000-0000-0000-000000000001";

async fn test_pool() -> PgPool {
    let url =
        std::env::var("AUTOTIM_TEST_DATABASE_URL").unwrap_or_else(|_| DEFAULT_DEV_URL.to_string());
    PgPool::connect(&url)
        .await
        .expect("failed to connect to Postgres for RLS integration test")
}

#[tokio::test]
#[ignore]
async fn querying_users_without_tenant_context_fails_closed() {
    let pool = test_pool().await;
    let mut conn = pool.acquire().await.expect("failed to acquire connection");

    // No SET LOCAL app.org_id has been issued on this connection.
    // The policy's USING clause calls current_setting('app.org_id')
    // with no default, which raises an error rather than silently
    // returning zero or all rows. This is the fail-closed behavior
    // doc 12 requires: a caller that forgot to set tenant context
    // must not be able to mistake an empty result for "no rows in my
    // org" — it should error loudly instead.
    let result: Result<Vec<(Uuid,)>, sqlx::Error> = sqlx::query_as("SELECT id FROM users")
        .fetch_all(&mut *conn)
        .await;

    assert!(
        result.is_err(),
        "expected an error with no tenant context set, got: {result:?}"
    );
}

#[tokio::test]
#[ignore]
async fn querying_users_with_correct_org_returns_rows() {
    let pool = test_pool().await;
    let mut tx = pool.begin().await.expect("failed to begin transaction");

    sqlx::query(&format!("SET LOCAL app.org_id = '{DEFAULT_ORG}'"))
        .execute(&mut *tx)
        .await
        .expect("failed to set tenant context");

    let rows: Vec<(Uuid,)> = sqlx::query_as("SELECT id FROM users")
        .fetch_all(&mut *tx)
        .await
        .expect("query failed with correct tenant context set");

    assert!(
        !rows.is_empty(),
        "expected at least the seeded user in the default organization"
    );

    tx.rollback().await.expect("rollback failed");
}

#[tokio::test]
#[ignore]
async fn querying_users_with_wrong_org_returns_no_rows() {
    let pool = test_pool().await;
    let mut tx = pool.begin().await.expect("failed to begin transaction");

    // A different, valid-shaped UUID that does not match any
    // organization a seeded user belongs to. The policy must filter
    // these rows out entirely — not error, not return them.
    let other_org = "11111111-1111-1111-1111-111111111111";
    sqlx::query(&format!("SET LOCAL app.org_id = '{other_org}'"))
        .execute(&mut *tx)
        .await
        .expect("failed to set tenant context");

    let rows: Vec<(Uuid,)> = sqlx::query_as("SELECT id FROM users")
        .fetch_all(&mut *tx)
        .await
        .expect("query failed with a valid (if foreign) tenant context set");

    assert!(
        rows.is_empty(),
        "RLS leak: rows from a different organization were visible: {rows:?}"
    );

    tx.rollback().await.expect("rollback failed");
}
