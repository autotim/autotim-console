-- 0003_create_module_migrations.sql  (kernel bootstrap migration)
--
-- Per-module applied-migration ledger with checksums. See doc 14
-- §"Migrations" ("Tracked in module_migrations with checksums;
-- mismatched checksum aborts startup") and doc 13 §"Module Registry".
--
-- This is the table the kernel's migration runner (a later commit)
-- consults at boot for every enabled module: for each Migration the
-- module declares (autotim-sdk Migration { version, up, down, ... }),
-- the runner computes sha256(up) and compares:
--   - no row for (module_name, version)      -> apply up, insert row
--   - row exists, checksum matches            -> already applied, skip
--   - row exists, checksum DIFFERS            -> abort boot (a migration
--                                                file was edited after
--                                                being applied — doc 14)
--
-- APPEND-ONLY / HISTORICAL. Unlike `modules` (0001), rows here are
-- never deleted when a module is recompiled out: this is the durable
-- proof that a migration ran, which a future data-retention migration
-- (doc 13 — removing a module) needs as its starting point.
--
-- PLATFORM-GLOBAL, NOT TENANT-SCOPED (doc 12): schema is a property of
-- the database/binary, not of a tenant. Exempt from the tenant-column
-- CI check. (Module migrations may CREATE tenant-scoped tables, but the
-- record THAT they were applied is global — a tenant does not "own" the
-- fact that the users table exists.)
--
-- checksum stored as text (hex sha256). The algorithm lives in the
-- kernel runner, not here; the column is algorithm-agnostic storage.
--
-- No DEFAULT on uuid columns (doc 14): the surrogate id is generated in
-- Rust. (Identity is (module_name, version); id exists only as a stable
-- single-column handle for ordering/joins.)

CREATE TABLE IF NOT EXISTS module_migrations (
    id           uuid PRIMARY KEY,
    module_name  text NOT NULL,
    version      text NOT NULL,
    description  text NOT NULL,
    checksum     text NOT NULL,
    applied_at   timestamptz NOT NULL DEFAULT now(),
    UNIQUE (module_name, version)
);

CREATE INDEX IF NOT EXISTS module_migrations_module_name_idx
    ON module_migrations (module_name);

COMMENT ON TABLE module_migrations IS
    'Append-only ledger of applied module migrations with sha256 '
    'checksums (doc 14). Identity is (module_name, version). A checksum '
    'mismatch on a previously-applied migration aborts boot. Rows are '
    'never deleted when a module is recompiled out — durable proof a '
    'migration ran.';