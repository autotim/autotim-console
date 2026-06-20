# 12 — Tenancy Architecture

## Why This Exists From Day One

The roadmap includes **Reseller Management** and **Customer Portal** — inherently multi-tenant. Adding tenancy after data exists means rewriting every table, query, RBAC check, and API route. We therefore introduce tenancy in v1, even though v1 ships effectively single-tenant.

## Model: Shared Schema + `organization_id` + Row-Level Security

Chosen over schema-per-tenant and database-per-tenant because it offers the best fit for self-hosted single-tenant and future SaaS/reseller multi-tenant, with cross-tenant reporting (resellers need it) and high density.

```text
organization (tenant boundary)
   ├── users (membership)
   ├── roles, grants
   ├── hosts, agents
   ├── secrets, settings (module/user scope)
   ├── audit events
   └── jobs, notifications, domain data (dns zones, certs, …)
```

## The `organizations` Entity

```text
organization
├── id (uuid, pk)
├── parent_id (uuid, null)   -- reseller hierarchy (EE); null in CE
├── slug (unique)
├── name
├── status (active | suspended)
├── created_at / updated_at
```

- **Community edition:** a single `default` organization is seeded at install. Everything belongs to it. Operators never see tenancy UI.
- **Enterprise edition:** multiple organizations, optional `parent_id` hierarchy for resellers.

## The `organization_id` Convention

Every tenant-scoped table carries `organization_id uuid not null` (FK to `organizations`). Platform-global tables (e.g. `modules`, `module_migrations`, the permission registry) do not.

Uniqueness becomes tenant-scoped:

```sql
-- a hostname is unique within an organization, not globally
unique (organization_id, hostname)
```

A module's manifest declares `tenant_scoped: true/false`. The kernel verifies tenant-scoped modules' tables carry `organization_id` (CI check).

## Tenant Context Propagation

Tenancy is resolved once per request and flows everywhere:

```text
Request → Auth → resolve organization_id (from session/token/membership)
        → TenantContext (request-scoped)
        → set Postgres session variable: SET LOCAL app.org_id = '<uuid>'
        → RLS enforces isolation automatically
```

Across async boundaries the `organization_id` travels in the event/job/outbox envelope and the agent command envelope, so jobs and event handlers run in the correct tenant context.

## Row-Level Security (defense in depth)

RLS is the safety net: even a buggy query cannot leak across tenants.

```sql
ALTER TABLE hosts ENABLE ROW LEVEL SECURITY;

CREATE POLICY tenant_isolation ON hosts
  USING (organization_id = current_setting('app.org_id')::uuid);
```

A dedicated **superuser/maintenance role** bypasses RLS only for migrations and platform-global operations, never for request handling. Application connections run as a role subject to RLS.

## Resolution Order for `organization_id`

1. Explicit organization context in the session (operator switched org — EE).
2. The user's single membership (CE / simple EE).
3. Reseller acting on a child org (EE; requires a scoped grant — doc 21).

## Interaction With RBAC

Authorization scopes always include the tenant. A grant is meaningless without it:

```text
grant(user=bandit, permission=dns.zone.create, scope=org:<uuid>)
```

A reseller managing children holds grants scoped to child organizations. See doc 21.

## Migration Impact

- Now (greenfield): trivial — the column and RLS policies are part of initial migrations.
- Later (if skipped): effectively a rewrite. This document exists to prevent that.

## Invariant

Every tenant-scoped row knows its organization, and the database — not just the application — refuses to cross that line.
