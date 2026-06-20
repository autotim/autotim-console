-- 0001_create_organizations.sql
--
-- Tenancy root entity. See docs/architecture/12-tenancy.md.
--
-- Community Edition: a single 'default' organization is seeded below.
-- Everything in CE belongs to it; operators never see tenancy UI.
-- Enterprise Edition: multiple organizations, optional parent_id
-- hierarchy for resellers (doc 12 §"The organizations Entity").
--
-- PK note: no DEFAULT on id. Doc 14 requires UUID v7 (time-ordered)
-- on every primary key; the Postgres major version in dev/staging
-- (13.x) predates the native uuidv7() generator (PG18+). Application
-- code (autotim-sdk, uuid crate with the "v7" feature) always
-- generates and supplies the id explicitly on INSERT — Postgres never
-- generates primary keys for this platform.

CREATE TABLE organizations (
    id          uuid PRIMARY KEY,
    parent_id   uuid REFERENCES organizations(id),
    slug        text NOT NULL UNIQUE,
    name        text NOT NULL,
    status      text NOT NULL DEFAULT 'active'
                    CHECK (status IN ('active', 'suspended')),
    created_at  timestamptz NOT NULL DEFAULT now(),
    updated_at  timestamptz NOT NULL DEFAULT now()
);

COMMENT ON TABLE organizations IS
    'Tenant boundary (doc 12). parent_id is null in CE; used for the '
    'reseller hierarchy in EE.';

-- CE seed: the single default organization every CE row belongs to.
-- Fixed UUID so application code and other migrations can reference it
-- without a runtime lookup.
INSERT INTO organizations (id, parent_id, slug, name, status)
VALUES (
    '00000000-0000-0000-0000-000000000001',
    NULL,
    'default',
    'Default Organization',
    'active'
);