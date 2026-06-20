# 00 — Overview & Principles

## Purpose

Autotim Console is a modular, self-hosted infrastructure management platform. It provides a unified control plane for managing servers, services, DNS, certificates, mail, monitoring, backups, and — in the Enterprise edition — customer-facing business operations.

The platform is **modular by architecture, not by convention**. A module behaves like a plug-in compiled into the platform: it registers its capabilities, respects Core contracts, and can be enabled or disabled without touching unrelated modules.

## Architectural Layers

```text
Core            foundational, domain-agnostic services
   ↑
Infrastructure  technical domains (DNS, SSL, Mail, Monitoring…)
   ↑
Business        commercial domains (Billing, CRM, Reseller…)
```

**Dependency law (non-negotiable):**

```text
Allowed:    Core → Core
            Infrastructure → Core
            Business → Core
            Business → Infrastructure
Forbidden:  Core → Infrastructure
            Core → Business
            Infrastructure → Business
```

Core must remain reusable and stable. Feature modules must be removable without breaking Core. Business modules must never become platform dependencies.

### Infrastructure → Infrastructure Dependencies

The dependency law above governs *layers*. It is silent on dependencies **within** the Infrastructure layer (e.g. SSL needing DNS for a DNS-01 challenge). That case is addressed explicitly:

```text
Infrastructure modules SHOULD communicate through events by default.

Direct Infrastructure → Infrastructure dependencies are allowed only when:
  - declared in the module manifest (Module::manifest().dependencies)
  - validated at boot (same mechanism as any other declared dependency, doc 13)
  - documented as optional or required
  - the failure mode is explicit (what happens if the dependency is disabled)
```

Example — certificate issuance via DNS-01:

```text
Preferred (event-driven, no coupling):
  SSL emits CertificateChallengeNeeded
  DNS subscribes, creates the TXT record
  DNS emits DnsChallengeReady
  SSL proceeds

Allowed (declared dependency, when tight integration is required):
  SSL declares dependency: ["dns"] in its manifest
  Boot validation fails if DNS is required but not enabled
  SSL's manifest states the fallback (e.g. HTTP-01) if DNS is optional
```

This preserves the disable-safety invariant: a module that is disabled should not silently break another module in an undocumented way. See doc 42 for how this applies to provider-backed Infrastructure modules.

## Core Modules

```text
Module Registry · Settings · Audit · Tenancy
Users · Roles · Permissions (RBAC)
Authentication · Security · Secrets
Async Substrate (Jobs · Events · Notifications)
Hosts · Agent
```

> **Boundary note (resolved from v1):** Hosts, Agent, and Notification *channels* are domain/integration concerns. We keep a thin core for each (Hosts inventory contract, Agent transport, Notification dispatch port) and push integrations (specific notification channels, agent capabilities) into adapters, so Core does not grow unbounded. See doc 32 for the Notification channel provider model and doc 42 for the general Integration Provider pattern it follows.

## Editions

Autotim ships in two editions built from the same Core:

- **Community (`autotim`, AGPL v3):** Core + all Infrastructure modules.
- **Enterprise (`autotim-ee`, Commercial):** adds Business modules and enterprise features (LDAP/AD/SAML, advanced multi-tenancy, compliance reporting).

Both editions share the identical Core. The Enterprise binary is the Community workspace plus the private enterprise crates.

## Design Priorities (in order)

1. **Security** — secure by default, least privilege, defense in depth, full auditability.
2. **Long-term modularity** — clean seams that survive 10 → 10,000 servers.
3. **Self-hosted simplicity** — a single binary, predictable operation, minimal moving parts.
4. **Mobile-first UX** — the console is fully usable from a phone.
5. **Maintainability** — one way to do async, one way to authorize, one place for secrets.

We explicitly do not optimize for short-term implementation speed at the cost of the five foundational decisions (tenancy, module model, async substrate, authorization model, secrets/storage boundaries).

## What "Modular Monolith" Means Here

- **One deployable binary.** Modules are compile-time Rust crates linked into the binary.
- Runtime control is enable/disable, recorded in a `module_state` table — not runtime code installation.
- **In-process by default**, distributed when needed: the event bus, RBAC, and job runner are accessed through ports (traits), so single-node and multi-node share the same module code.
- True out-of-process / WASM third-party plugins are a named future epic, not a v1 assumption.

This gives ~95% of plugin-style modularity at ~20% of the cost, and keeps the self-hosted single-binary advantage.

## Invariant

If a module is removed (disabled), Core keeps running, unrelated modules keep running, and no manual code changes are required.
