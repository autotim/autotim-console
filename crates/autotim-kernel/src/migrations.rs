//! Module migration runner (doc 13 boot step 7, doc 14 §"Migrations",
//! doc 60).
//!
//! At boot, for every enabled module, the kernel applies the module's
//! declared migrations (`Module::migrations()`) that have not yet been
//! recorded in the `module_migrations` ledger (created by the kernel
//! bootstrap migrations under `migrations/_kernel/`, doc 13).
//!
//! ## Checksums
//!
//! Each applied migration is recorded with a SHA-256 checksum of its
//! forward (`up`) SQL. On a later boot:
//!   - no ledger row for (module, version) -> apply, then record;
//!   - row exists, checksum matches          -> already applied, skip;
//!   - row exists, checksum DIFFERS           -> abort boot. A migration
//!     file that was already applied has been edited; silently
//!     re-applying or ignoring it would diverge the schema from the
//!     ledger (doc 14: "mismatched checksum aborts startup").
//!
//! ## Atomicity
//!
//! Each migration's `up` SQL and its ledger INSERT run in the SAME
//! transaction. Either both commit or neither does: a migration that
//! fails partway leaves no ledger row, so the next boot retries it
//! rather than skipping it. (Caveat: a migration containing a statement
//! that cannot run inside a transaction — e.g. CREATE INDEX
//! CONCURRENTLY — would fail here. None of the platform's migrations do
//! this today; if one ever needs to, it gets its own non-transactional
//! path. Documented so it is a deliberate choice, not a latent bug.)
//!
//! ## Connection
//!
//! Migrations run on a connection owned by a role with DDL/owner
//! privileges (CREATE TABLE, ALTER TABLE ... ENABLE ROW LEVEL
//! SECURITY), NOT the RLS-subject application role (`autotim_app`,
//! doc 12). The caller supplies that connection; this runner does not
//! choose roles. Today, dev runs migrations as the database owner; the
//! application pool connects separately as the RLS-subject role.

use autotim_sdk::{Migration, Module};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum MigrationError {
    #[error("database error applying {module}:{version}: {source}")]
    Database {
        module: String,
        version: String,
        #[source]
        source: sqlx::Error,
    },

    #[error(
        "checksum mismatch for {module}:{version} — the migration was \
         edited after being applied (ledger {recorded}, computed \
         {computed}); refusing to boot (doc 14)"
    )]
    ChecksumMismatch {
        module: String,
        version: String,
        recorded: String,
        computed: String,
    },

    #[error("failed to read migration ledger: {0}")]
    Ledger(#[source] sqlx::Error),
}

/// SHA-256 of a migration's forward SQL, lowercase hex.
///
/// Computed over the exact bytes of `up` (which a module supplies via
/// `include_str!` of its `.sql` file) with no normalization, so the
/// value matches `sha256sum <file>` exactly. This is what lets an
/// out-of-band "adopt existing migrations" step (for a database whose
/// tables predate the ledger) record checksums that the runner will
/// then accept without re-applying.
pub fn checksum(up: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(up.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[derive(sqlx::FromRow)]
struct LedgerRow {
    version: String,
    checksum: String,
}

/// Apply all pending migrations for a single module against `pool`.
///
/// `pool` must be connected as a role with owner/DDL privileges (see
/// module docs). Returns the number of migrations newly applied.
pub async fn run_module_migrations(
    pool: &PgPool,
    module: &dyn Module,
) -> Result<usize, MigrationError> {
    let module_name = module.manifest().name;
    let migrations = module.migrations();
    if migrations.is_empty() {
        return Ok(0);
    }

    // Load this module's existing ledger rows once.
    let recorded: Vec<LedgerRow> =
        sqlx::query_as("SELECT version, checksum FROM module_migrations WHERE module_name = $1")
            .bind(module_name)
            .fetch_all(pool)
            .await
            .map_err(MigrationError::Ledger)?;

    let mut applied = 0usize;

    for migration in migrations {
        let Migration {
            version,
            description,
            up,
            down: _,
        } = migration;
        let computed = checksum(up);

        if let Some(row) = recorded.iter().find(|r| r.version == *version) {
            // Already applied — verify integrity, then skip.
            if row.checksum != computed {
                return Err(MigrationError::ChecksumMismatch {
                    module: module_name.to_string(),
                    version: version.to_string(),
                    recorded: row.checksum.clone(),
                    computed,
                });
            }
            continue;
        }

        // Not yet applied: run `up` and record the ledger row in one
        // transaction.
        let mut tx = pool.begin().await.map_err(|e| MigrationError::Database {
            module: module_name.to_string(),
            version: version.to_string(),
            source: e,
        })?;

        sqlx::raw_sql(up)
            .execute(&mut *tx)
            .await
            .map_err(|e| MigrationError::Database {
                module: module_name.to_string(),
                version: version.to_string(),
                source: e,
            })?;

        sqlx::query(
            "INSERT INTO module_migrations \
             (id, module_name, version, description, checksum) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(Uuid::now_v7())
        .bind(module_name)
        .bind(*version)
        .bind(*description)
        .bind(&computed)
        .execute(&mut *tx)
        .await
        .map_err(|e| MigrationError::Database {
            module: module_name.to_string(),
            version: version.to_string(),
            source: e,
        })?;

        tx.commit().await.map_err(|e| MigrationError::Database {
            module: module_name.to_string(),
            version: version.to_string(),
            source: e,
        })?;

        applied += 1;
    }

    Ok(applied)
}
