-- 0002_create_module_state.sql  (kernel bootstrap migration)
--
-- Enable/disable state for each module. See doc 13 §"Module Registry"
-- and §"Lifecycle". Separated from the `modules` table (0001) because
-- the two have DIFFERENT LIFECYCLES, and that difference is the whole
-- point of this table:
--
--   modules (0001)  = "what is compiled in", a runtime projection,
--                     repopulated at boot; a module that is no longer
--                     compiled in disappears from it.
--   module_state    = operator intent that must SURVIVE across boots
--                     and across a module temporarily disappearing.
--                     Disabling DNS must stay disabled after a restart;
--                     if DNS is dropped from one release and returns in
--                     a later one, the prior "disabled" intent should
--                     still be here.
--
-- Because the lifecycles differ, there is deliberately NO foreign key
-- to modules(name): a FK (with or without ON DELETE CASCADE) would
-- couple the two lifecycles — cascading would erase operator intent
-- the moment a module is recompiled out, and restricting would block
-- the boot-time reprojection of `modules`. The kernel reconciles the
-- two sets in code at boot (seed state for newly-seen modules; leave
-- stale state untouched). This matches doc 13's cross-module integrity
-- rule: reference by name, not by enforced FK across independent
-- lifecycles.
--
-- PLATFORM-GLOBAL, NOT TENANT-SCOPED (doc 12): a module is enabled or
-- disabled for the whole platform, not per organization. Exempt from
-- the tenant-column CI check.
--
-- enabled_by is nullable: NULL means the row was seeded by the kernel
-- at boot (first time a compiled-in module was seen), not toggled by a
-- human operator. A non-null value references the user who flipped it
-- once a real enable/disable API exists (doc 13). No FK to users
-- either: users is tenant-scoped and module-owned; a platform-global
-- table must not hard-FK into a module's tenant data.
--
-- No DEFAULT on uuid columns (doc 14): generated in Rust.

CREATE TABLE IF NOT EXISTS module_state (
    module_name  text PRIMARY KEY,
    enabled      boolean NOT NULL DEFAULT true,
    enabled_at   timestamptz NOT NULL DEFAULT now(),
    enabled_by   uuid,
    updated_at   timestamptz NOT NULL DEFAULT now()
);

COMMENT ON TABLE module_state IS
    'Operator enable/disable intent per module (doc 13). Survives '
    'across boots and across a module being recompiled out; not '
    'repopulated from compiled state, and intentionally not FK-linked '
    'to modules (independent lifecycles). enabled_by NULL = seeded by '
    'the kernel at boot, not toggled by an operator.';