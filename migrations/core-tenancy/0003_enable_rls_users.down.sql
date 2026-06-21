-- 0003_enable_rls_users.down.sql  (rollback of 0003)
--
-- Reverses 0003: removes the tenant-isolation policy and disables RLS
-- on users. See doc 60 §"Database Migrations" (every migration ships a
-- forward and a rollback) and doc 12 for what is being undone.
--
-- Order matters: the policy must be dropped before (or it is dropped
-- implicitly with) RLS being disabled. DROP POLICY IF EXISTS is used so
-- the rollback is idempotent and safe to run even if 0003's forward
-- step was only partially applied.

DROP POLICY IF EXISTS tenant_isolation ON users;

ALTER TABLE users DISABLE ROW LEVEL SECURITY;