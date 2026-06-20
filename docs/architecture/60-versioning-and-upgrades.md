# 60 — Versioning & Upgrades

## Semantic Versioning

`MAJOR.MINOR.PATCH`. MAJOR = breaking change; MINOR = backward-compatible feature; PATCH = fix. The whole workspace shares the platform version per release; the `autotim-sdk` version is the **module compatibility contract**.

## Module Compatibility (compile-time)

Because modules are compile-time crates (doc 13), version compatibility is primarily a build-time concern: a module is built against an `sdk_version`. At boot the kernel rejects a module whose `sdk_version` is incompatible with the running SDK. This is simpler and safer than runtime version negotiation — a direct benefit of the modular-monolith decision.

```text
module dns  sdk_version = "1.x"   ✓ runs on SDK 1.4
module dns  sdk_version = "1.x"   ✗ refused on SDK 2.0 (breaking)
```

Agent ↔ control-plane compatibility **is** a runtime concern (separate processes) and is negotiated/validated on connect (doc 41).

## Upgrade Flow

```text
Check compatibility (SDK, migrations)
   → Backup (DB incl. audit + ciphertext; confirm KEK backup)
   → Run forward migrations (idempotent)
   → Validate health (DB, deps, APIs, jobs, event handlers, unseal)
   → Resume serving
```

Every upgrade is audited (who, when, from→to, result).

## Database Migrations

Versioned, tracked (`module_migrations` + checksums), **reversible**, idempotent (doc 14). Each migration ships a forward and a rollback. Partition/retention changes are migrations too.

## Rollback

Every upgrade defines a rollback path: rollback migrations + binary downgrade + health re-validation. Backups taken pre-upgrade are the safety net. The platform must never be left in an unknown state.

## Deprecation Policy

Features/permissions/events may be marked deprecated and remain available for a defined window before removal in a MAJOR release. Deprecations are documented in release notes and surfaced in logs.

## Breaking Changes

Require: MAJOR bump, migration guide, compatibility notes, and (for the SDK) a clear module-author upgrade path.

## Edition Parity

Community and Enterprise share Core and version together. An Enterprise upgrade is a Community Core upgrade plus enterprise crates; compatibility is validated for both in EE CI.

## Invariant

Every upgrade is reversible and audited.
