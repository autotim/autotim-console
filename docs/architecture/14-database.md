# 14 — Database Architecture & Conventions

## Engine

**PostgreSQL only**, for Core, Infrastructure, and Business modules. Accessed via SQLx with compile-time-checked queries (no full ORM; explicit SQL, fewer surprises).

> Time-series metrics do **not** live here. See doc 51: Prometheus/VictoriaMetrics for metrics, object storage for large blobs/backups.

## Conventions

| Concern | Rule |
|---------|------|
| Primary keys | UUID v7 (time-ordered) — index-friendly, distributed-safe |
| Naming | `snake_case` tables and columns; plural tables (`users`, `hosts`) |
| Timestamps | `created_at`, `updated_at` on every entity; `deleted_at` for soft delete |
| Foreign keys | Explicit **within a module**; **none across module boundaries** (UUID refs only) |
| Audit fields | `created_by`, `updated_by` (UUID) where meaningful |
| Tenancy | `organization_id uuid not null` on every tenant-scoped table (doc 12) |
| Money/exact | `numeric`, never float (Billing) |
| Enums | Postgres enums or check constraints; never magic integers |

UUID v7 is preferred over v4: time-ordering keeps B-tree indexes and inserts efficient at scale, avoiding the write amplification of random UUIDs.

## Module Ownership

Each table belongs to exactly one module. Cross-module queries go through ports/APIs or read-only views, never direct joins into another module's tables. This keeps modules disable-able and migrations independent.

## Core Platform Tables (selected)

```text
organizations            -- tenancy root (doc 12)
modules, module_state, module_migrations
users, user_identities   -- local + external (OIDC) identities
roles, permissions_registry, grants   -- RBAC (doc 21)
sessions, api_tokens, mfa_factors, webauthn_credentials  -- auth (doc 22)
secrets                  -- envelope-encrypted (doc 23)
settings                 -- schema-driven, scoped (global/module/user)
audit_events             -- tamper-evident, partitioned (doc 24)
outbox                   -- transactional outbox (doc 31)
event_store              -- persisted events (doc 31)
jobs, job_runs           -- async substrate (doc 31)
notifications            -- delivery records (doc 31)
hosts, host_interfaces, host_services
agents, agent_capabilities
```

## Transactional Outbox (correctness foundation)

State changes that must produce events write the event **in the same transaction**:

```sql
BEGIN;
  INSERT INTO hosts (...) VALUES (...);
  INSERT INTO outbox (id, organization_id, event_type, payload, created_at)
    VALUES (...);
COMMIT;
```

A relay worker reads `outbox`, publishes to the event bus, and marks rows dispatched. This makes at-least-once delivery real: no event is lost between commit and publish. Consumers must be idempotent (doc 31).

## Partitioning & Retention

High-volume tables are range-partitioned by time from the start:

- `audit_events` — monthly partitions; retention policy per compliance needs.
- `event_store` — monthly partitions; archival/pruning policy.
- `job_runs` — monthly partitions; prune completed after N days.

Partitioning is cheap to add now and painful to retrofit on a huge table later (echoing the tenancy lesson).

## Indexing

Index foreign keys, `organization_id` (often as the leading column of composite indexes for RLS-friendly queries), and frequently filtered/sorted columns (`status`, `hostname`, `email`, `created_at`). Review with `EXPLAIN` on hot paths.

## JSONB

Allowed for: settings values, event payloads, metadata, flexible capability sets. **Not** for core business data that needs relational integrity or frequent structured querying.

## Migrations

- SQLx migrations, **versioned, reversible, idempotent**.
- Stored per module under `migrations/<module>/`.
- Tracked in `module_migrations` with checksums; mismatched checksum aborts startup.
- Every schema change ships with a forward and a rollback (doc 60).

## Connection Management

- A single pooled application role, subject to RLS.
- A separate maintenance role for migrations / platform-global tasks (RLS-bypassing), never used for request handling.
- Pool sizing tuned per deployment; PgBouncer optional at higher tiers.

## Scaling Path

```text
10–100 servers   single Postgres
1,000            read replicas; enforce partition retention; dedicated pools
10,000           read replicas mandatory; archival of audit/events;
                 possible functional or tenant-based sharding
```

The conventions above (UUID v7, partitioning, no cross-module FKs, tenancy column) are exactly what make this path evolution rather than rewrite.

## Constraint

Database consistency and tenant isolation outrank developer convenience.
