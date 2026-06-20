# Migrations

SQLx migrations, **versioned, reversible, idempotent** (see
`docs/architecture/04-database.md` and `15-versioning-and-upgrades.md`).

Each module owns its migrations under a subdirectory named after the module,
tracked in `module_migrations` with checksums. Cross-module foreign keys are
forbidden (doc 13 integrity rule); reference other modules' entities by UUID.

```text
migrations/
  core-tenancy/      organizations, organization_id conventions, RLS policies
  core-rbac/         permissions registry, roles, grants
  core-audit/        audit_events (partitioned, hash-chained)
  core-async/        outbox, event_store, jobs, job_runs
  core-secrets/      secrets (envelope-encrypted)
  core-hosts/        hosts, host_interfaces, host_services
  ...
```

The first real migration set lands with the kernel + tenancy milestone.
