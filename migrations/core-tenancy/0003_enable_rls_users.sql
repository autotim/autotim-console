-- 0003_enable_rls_users.sql
--
-- Row-Level Security on the first real tenant-scoped table. See
-- docs/architecture/12-tenancy.md §"Row Level Security (defense in
-- depth)". RLS is the safety net: even a buggy application query
-- cannot leak rows across organizations, independent of whatever the
-- app-layer authorizer (doc 21) does or fails to do.
--
-- Session variable contract: the connection serving a request must
-- run `SET LOCAL app.org_id = '<uuid>'` before touching this table.
-- That wiring (extracting organization_id from TenantContext and
-- issuing the SET LOCAL per request/transaction) belongs to the
-- kernel's request pipeline (doc 11), not to this migration — this
-- migration only establishes the policy the database enforces once
-- that variable is set.
--
-- current_setting('app.org_id') with no SET LOCAL in effect raises an
-- error rather than silently returning NULL, which is the conservative
-- (fail-closed) default we want: a connection that forgot to set the
-- tenant context should error, not see zero rows and look "fine".

ALTER TABLE users ENABLE ROW LEVEL SECURITY;

-- No BYPASSRLS role is granted here. Doc 12 calls for a dedicated
-- superuser/maintenance role to bypass RLS for migrations and
-- platform-global operations — that role does not exist yet (no
-- connection pool / role separation has been wired in the kernel
-- yet, see doc 11 boot sequence step 2). Creating it now, unused,
-- would be a role nobody can audit the use of. It is created in the
-- same commit that introduces the real connection pool and the
-- maintenance/application role split.

CREATE POLICY tenant_isolation ON users
    USING (organization_id = current_setting('app.org_id')::uuid);

COMMENT ON POLICY tenant_isolation ON users IS
    'Doc 12 defense-in-depth: rows are visible/writable only within '
    'the organization set via SET LOCAL app.org_id for the current '
    'transaction. Requires the session variable to be set; there is '
    'no implicit fallback to "all rows" or "no rows".';