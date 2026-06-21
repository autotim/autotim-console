-- 0002_create_users.down.sql  (rollback of 0002)
--
-- Reverses 0002: drops the users table. The idx_users_organization_id
-- index and the FK to organizations are dropped implicitly with the
-- table. See doc 60.
--
-- This assumes 0003's rollback (disabling RLS / dropping the policy)
-- has already run, since migrations roll back in reverse order. DROP
-- TABLE IF EXISTS keeps it idempotent.

DROP TABLE IF EXISTS users;