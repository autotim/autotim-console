-- 0001_create_organizations.down.sql  (rollback of 0001)
--
-- Reverses 0001: drops the organizations table, including the seeded
-- default organization. See doc 60.
--
-- This is the last tenancy rollback to run (migrations reverse in
-- order 0003 -> 0002 -> 0001); by this point users (0002) is already
-- gone, so the FK from users -> organizations no longer exists to block
-- the drop. DROP TABLE IF EXISTS keeps it idempotent. The explicit seed
-- DELETE is unnecessary before a DROP TABLE but is harmless and makes
-- the intent (this removes the default org too) self-documenting if the
-- DROP is ever softened to a data-only rollback.

DROP TABLE IF EXISTS organizations;