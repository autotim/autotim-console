-- adopt-tenancy-migrations.sql  (one-shot, dev only)
--
-- Records tenancy migrations 0001-0003 as already applied in the
-- module_migrations ledger, WITHOUT re-running them, for a database
-- whose organizations/users tables were created by hand before the
-- migration runner existed (this dev host). On a clean database the
-- kernel runner applies these normally and this script is not used.
--
-- The checksums below are sha256 of each forward .sql file, identical
-- to what the kernel's checksum() computes from the include_str!'d
-- bytes — so after this runs, the next boot sees matching checksums
-- and skips re-application instead of aborting on a mismatch.
--
-- Run once, as a role that can INSERT into module_migrations:
--   psql -d autotim_dev -f scripts/adopt-tenancy-migrations.sql
--
-- Idempotent: ON CONFLICT (module_name, version) DO NOTHING means
-- running it twice is harmless.

INSERT INTO module_migrations (id, module_name, version, description, checksum)
VALUES
    ('cc22f870-03e6-4335-80b5-e5bb1a1364b9', 'tenancy', '0001', 'create organizations', '0d360de3c48c1fd8628d86d93355e8a2eabef88035609b7171a3e28efc61bb38'),
    ('aa9c3da4-cf22-4996-ab4c-f12c20522d3f', 'tenancy', '0002', 'create users', '4bb6383c500eedc98b2e7e126ffbb71fd68fbcb1eea066529ffa29b8ff3e65a4'),
    ('03543e47-5624-43d7-a90d-6277170ab8e8', 'tenancy', '0003', 'enable RLS on users', 'e6d9500376d544e3b65564b60eb5c98ff9742e91e75b5365ab7f543ffbcb28e2')
ON CONFLICT (module_name, version) DO NOTHING;