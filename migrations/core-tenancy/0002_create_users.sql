-- 0002_create_users.sql
--
-- Minimal user entity, scoped to a tenant. See docs/architecture/12-tenancy.md
-- (organization_id convention) and docs/architecture/14-database.md
-- (PK/timestamp conventions).
--
-- Deliberately minimal at this stage: no password hash, no identity
-- linking (local/OIDC), no MFA, no sessions. Those belong to doc 22
-- (Authentication) and land in their own migration when that module
-- is implemented. This table exists now only so organization_id and
-- Row-Level Security have a real, non-trivial table to be proven
-- against (doc 12 §"Why This Exists From Day One").
--
-- PK note: same as 0001 — no DEFAULT on id; the application supplies
-- a v7 UUID explicitly on INSERT (PG13 has no native uuidv7()).

CREATE TABLE users (
    id              uuid PRIMARY KEY,
    organization_id uuid NOT NULL REFERENCES organizations(id),
    status          text NOT NULL DEFAULT 'active'
                        CHECK (status IN ('active', 'disabled', 'locked')),
    created_at      timestamptz NOT NULL DEFAULT now(),
    updated_at      timestamptz NOT NULL DEFAULT now()
);

COMMENT ON TABLE users IS
    'Tenant-scoped user entity (doc 12). Identity (password, OIDC, '
    'MFA, email) is added by doc 22''s migration, not here.';

-- organization_id is the leading column so RLS-filtered lookups and
-- composite indexes stay efficient at scale (doc 14 §"Indexing").
CREATE INDEX idx_users_organization_id ON users (organization_id);