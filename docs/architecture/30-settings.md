# 30 — Settings Architecture

## Purpose

Centralized, schema-driven configuration for the entire platform. Every module registers its settings here — no module implements its own independent settings system. This document was present in Foundation v1 but was lost in the v2 rewrite (folded only partially into docs 13 and 23); this is the repair.

Settings is distinct from bootstrap config (doc 11 §"Two Kinds of Configuration"). Bootstrap config gets you to a healthy database; Settings is everything configurable *after* that — and it is DB-backed, validated, audited, and tenant-scoped.

## Scope Levels

```text
Global         platform-wide default (e.g. password policy baseline)
Organization   per-tenant override (doc 12 — Tenancy); the level CE
               operates at exclusively, against the single default org
Module         a module's own configuration (e.g. dns.default_ttl)
User           per-user preference (e.g. ui.theme, notification quiet hours)
```

Resolution order for a given key: **User → Module (org-scoped) → Organization → Global**, first match wins. This lets an operator set an org-wide default while individual users override their own preferences (e.g. theme), without every module reinventing that precedence logic.

## Schema-Based Design

Every setting is declared, not freeform. A module's manifest contributes a settings schema (doc 13 — `Module::settings_schema()`):

```rust
SettingDef {
    key: "dns.default_ttl",
    value_type: SettingType::Integer,
    default: json!(3600),
    scope: SettingScope::Organization,
    validation: Validation::Range { min: 60, max: 604_800 },
    description: "Default TTL (seconds) for new DNS records.",
    secret_ref: false,
}
```

Supported types: `string · integer · boolean · float · enum · json · secret`. The `secret` type is special — see below.

## Validation

Enforced server-side before persistence: min/max length, numeric range, regex, allowed-values (enum), required fields. **Invalid settings are never saved** — the API returns a structured `422` (doc 40) with the specific field-level violation, never a partial write.

## Secret-Type Settings (never plaintext)

This is the most security-relevant rule in this document: a setting of type `secret` stores **only a reference** (a Secrets `Uuid`, doc 23), never a value.

```text
smtp.password   (type: secret)  →  stored as: secret_ref: <uuid>
network.netbird.api_token       →  stored as: secret_ref: <uuid>
```

The Settings UI renders these as "configured / not configured", never the value (doc 50). Reading the actual value is a separate, RBAC-authorized, audited action through the Secrets port (doc 23) — Settings never round-trips a secret value through its own read path. Doc 42 §3 (Integration Provider credentials) and doc 22 (OIDC client secret) both rely on this rule.

## The `SettingsStore` Port

Modules access settings only through the Core port (doc 11 §"Port Wiring"), never by querying the underlying table directly:

```rust
#[async_trait]
pub trait SettingsStore: Send + Sync {
    async fn get(&self, org: OrganizationId, key: &str) -> SdkResult<Option<Value>>;
    async fn set(&self, org: OrganizationId, key: &str, value: Value) -> SdkResult<()>;
}
```

`set()` runs the registered schema's validation before persisting and emits a `SettingChanged` event (consumed by, e.g., the Secrets module's `SecretRotated` propagation pattern from doc 23, or a provider re-resolving its configuration per doc 42 §3).

## UI Generation from Schema

The Settings UI (doc 50) is generated from the schema, not hand-built per module: the `value_type` and `validation` drive the rendered control (text input, number with min/max, toggle, select, secret-configure-button). This is what lets a new module's settings page appear automatically without bespoke frontend work, mirroring the dynamic navigation/route registration principle in doc 50.

## Import & Export

Settings support export (for backup/migration between environments) and import with the same validation path as a normal write — an import is just a batch of `set()` calls, never a raw table load. Secret-type values export as their reference only; the underlying secret must be migrated separately through the Secrets module's own backup path (doc 23 §"Backup & Recovery").

## Audit

Every create/update/delete of a setting is audited (doc 24): actor, organization, key, old/new value (with secret-type values redacted to their reference, never the underlying plaintext). Settings changes are exactly the kind of "configuration drift" event operators need in the tamper-evident audit trail.

## Constraint

One schema-driven store, one validation path, one place secrets never leak into.
