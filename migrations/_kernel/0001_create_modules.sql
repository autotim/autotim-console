-- 0001_create_modules.sql  (kernel bootstrap migration)
--
-- The Module Registry's "what is compiled into this binary, right now"
-- table. See docs/architecture/13-module-system.md §"Module Registry".
--
-- This is a KERNEL bootstrap migration, not a module migration: it is
-- applied unconditionally and idempotently by the kernel at boot,
-- BEFORE any module is consulted. It cannot itself be tracked in
-- module_migrations (0003) because module_migrations does not exist
-- until these bootstrap migrations have run — that is the chicken/egg
-- reason kernel migrations are a separate, self-contained set under
-- migrations/_kernel/ rather than owned by a module.
--
-- PLATFORM-GLOBAL, NOT TENANT-SCOPED. The set of compiled-in modules
-- is a property of the binary, identical for every organization, so
-- this table deliberately carries no organization_id (doc 12 lists the
-- registry tables among the platform-global exemptions). The
-- tenant-column CI check (scripts/check-tenant-columns.sh) must exempt
-- it for exactly this reason.
--
-- RUNTIME-ONLY, NOT HISTORICAL. This table reflects current compiled
-- state only: at boot the kernel upserts the modules it sees. A module
-- that is no longer compiled in simply stops appearing here. The
-- historical record of what ran lives in module_migrations (append-only,
-- 0003) and the tamper-evident audit log (doc 24, ModuleEnabled/…),
-- which are the sources of truth for "what happened" — this table is
-- only "what is".
--
-- No DEFAULT on any uuid column (doc 14): UUID v7 is generated in Rust
-- and supplied explicitly on INSERT.

CREATE TABLE IF NOT EXISTS modules (
    name         text PRIMARY KEY,
    version      text NOT NULL,
    layer        text NOT NULL
                     CHECK (layer IN ('core', 'infrastructure', 'business')),
    sdk_version  text NOT NULL,
    enabled      boolean NOT NULL DEFAULT true,
    created_at   timestamptz NOT NULL DEFAULT now(),
    updated_at   timestamptz NOT NULL DEFAULT now()
);

COMMENT ON TABLE modules IS
    'Module Registry (doc 13): compiled-in modules in the running '
    'binary. Platform-global (no organization_id). Runtime projection '
    'of compiled state, upserted at boot; history lives in '
    'module_migrations and the audit log, not here.';