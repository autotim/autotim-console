# 21 — RBAC / Authorization

## Model: RBAC + Scoped Grants

Authentication answers *who are you*; authorization answers *what may you do, and on what*. We use **role-based access control with scoped grants** — the v1 anti-pattern of embedding resource IDs into permission strings (`dns.zone.read:autotim.de`) is removed.

## Core Concepts

```text
Permission   smallest unit of authority         dns.zone.create
Role         a named set of permissions          DNS Admin
Grant        (subject, role|permission, scope)    bandit → DNS Admin @ org:X
Scope        where the grant applies              org | group | type | resource
```

A **grant** is the join of subject, authority, and scope. Scope always carries `organization_id`.

## Permission Naming (canonical & enforced)

Format: `module.resource.action`, lowercase, dot-separated, globally unique.

```text
dns.zone.read      dns.zone.create     dns.zone.update     dns.zone.delete
mail.domain.create ssl.certificate.renew
hosts.host.read    hosts.agent.restart
secrets.secret.read settings.global.update
```

Rules enforced by a CI linter:
- exactly three segments, lowercase, `[a-z0-9_]`,
- module segment matches the declaring module's name,
- declared as typed constants in `autotim-sdk`, never free-form strings at call sites.

This kills the v1 inconsistency (`host.` vs `hosts.`, 2- vs 3-segment) and prevents silent authz bugs.

## Permission Registry

Permissions are declared by modules (`Module::permissions()`) and collected into a platform registry at boot. The registry is the source of truth for: the authorization engine, the settings/roles UI, and audit. A permission referenced but not registered fails startup.

## Scope Model

```rust
pub enum Scope {
    Organization(OrganizationId),                 // all resources in a tenant
    Group { org: OrganizationId, group: GroupId },// e.g. a host group
    ResourceType { org: OrganizationId, kind: &'static str }, // all of a kind
    Resource { org: OrganizationId, kind: &'static str, id: Uuid }, // one resource
}
```

This expresses delegated administration cleanly:

```text
grant(user=alice, role="DNS Admin", scope=Organization(acme))
grant(user=bob,   permission="dns.zone.update",
                  scope=Resource{org:acme, kind:"zone", id:<uuid>})
grant(reseller,   role="Org Admin", scope=Organization(child-org))  // EE
```

No permission explosion, no full scans to answer "what can this user touch?".

## Built-in Roles

```text
Super Admin            (platform-wide; CE single-org = full control)
Organization Admin     (full control within one org — EE multi-tenant)
Infrastructure Admin
DNS Admin · Mail Admin · SSL Admin · Monitoring Admin
Read Only
Customer               (Customer Portal — EE)
```

Roles support **inheritance** (a role may include another role's permissions). Inheritance is resolved at evaluation and cached.

## Evaluation Flow

```text
Request
  → Authenticate (doc 22)         → subject + organization_id
  → Resolve grants for subject     (roles → permissions, + direct grants)
  → Check: required permission present at a scope covering the target?
  → Allow / Deny
```

The check is exposed once, centrally:

```rust
authorizer.require(&subject, perm::DNS_ZONE_UPDATE,
                   Scope::Resource{ org, kind: "zone", id }).await?;
```

Modules call this port; they never reimplement it.

## Decision Caching

Per-request permission resolution must not hammer Postgres. Resolved permission sets are cached per `(user_id, organization_id)` with event-driven invalidation:

```text
RoleAssigned / RoleRemoved / PermissionGranted / PermissionRevoked / RoleUpdated
   → publish event → cache invalidates affected subjects
```

This is the first real, motivating use of the event bus (doc 31). Cache is in-process at small scale; a shared cache (Redis) is a drop-in at multi-node scale.

## Resource Resolution

For `Resource`/`Group` scopes, the authorizer needs to know a target's organization and group. Modules expose a tiny **ownership lookup** (resource id → org/group) via a port, so the authorizer stays generic without joining into module tables.

## Multi-Tenancy Interaction

Authorization and RLS are **two independent controls** (defense in depth): the authorizer denies cross-tenant access at the app layer; RLS (doc 12) denies it at the database layer even if app logic is wrong.

## Audit

Every grant change is audited (`RoleAssigned`, `GrantRevoked`, …) and can raise security notifications. Authorization *denials* on sensitive permissions can be surfaced for anomaly detection (doc 24).

## Invariant

All access decisions pass through the Core authorizer, scoped to a tenant. No module may bypass it, and no permission lives outside the registry.
