# Autotim Console — Architecture Foundation v2

A modular, self-hosted infrastructure management platform.
Philosophy peers: Proxmox, Portainer, aaPanel, Coolify, Plesk.

## Confirmed Stack

| Layer | Choice |
|-------|--------|
| Backend | Rust · Axum · Tokio · SQLx (compile-checked queries) |
| Database | PostgreSQL only · UUID primary keys · Row-Level Security |
| Frontend | Vue 3 · Vite · PrimeVue · Pinia · Vue Router · TypeScript |
| Packaging | **Single binary** — Vue build embedded via `rust-embed` |
| Deployment | Bare-metal first · Linux · systemd · Podman optional |
| Metrics | Prometheus + OpenTelemetry + Grafana (never Postgres) |
| Events | In-process bus → NATS/Redis Streams (behind a stable port) |
| API | REST + OpenAPI first · GraphQL optional later |

## Editions & Licensing (Open Core)

| | Repo | Binary | License |
|--|------|--------|---------|
| Community | `github.com/autotim/autotim-console` (public) | `autotim` | AGPL v3 |
| Enterprise | private, not part of this documentation set | `autotim-ee` | Commercial |

CLA required before accepting external contributions. The `autotim/autotim` repo is reserved separately as the org profile README only (doc 10).

## The Five Foundational Decisions (resolved in v2)

1. **Tenancy** — `organizations` + `organization_id` everywhere + Postgres RLS (doc 12).
2. **Module model** — modular monolith, compile-time crates, enable/disable at runtime; **no** runtime install/uninstall (doc 13).
3. **Async substrate** — one durable queue + **transactional outbox**; event bus is a port with in-proc/broker adapters; notifications run on it (doc 31, doc 32).
4. **Authorization** — RBAC + **scoped grants** (incl. tenant), canonical permission registry + linter, decision caching (doc 21).
5. **Secrets & storage boundaries** — envelope encryption + pluggable KMS + unseal; metrics in TSDB, not Postgres (docs 23, 51).

## Document Index — Numbering Scheme

Documents are numbered in **bands**, not strict sequence, so a future addition slots into its band without renumbering anything else (the lesson learned when Kernel/Settings/Notifications were added after the fact). Within a band, reading top-to-bottom already follows build order.

```text
00        Overview & Principles            (scene-setter, stands alone)
10–19     Foundation / Build / Runtime
20–29     Security & Identity
30–39     Core Platform Services
40–49     Boundary / Domain
50–59     Experience & Observability
60–69     Process & Lifecycle
```

| # | Document | Notes |
|---|----------|-------|
| 00 | Overview & Principles | Vision, layers, dependency law, core invariants |
| 10 | Repository & Build | Cargo workspace, two repos, single binary, org profile-repo naming |
| 11 | Kernel | Bootstrap config vs Settings, boot sequence, port wiring, shutdown |
| 12 | Tenancy | Organizations, org_id, RLS |
| 13 | Module System | Module trait, manifest, lifecycle, registry |
| 14 | Database | Conventions, UUID, RLS, partitioning, outbox, migrations |
| 20 | **Security** | Threat model, defense in depth — deep |
| 21 | RBAC / Authorization | Scoped grants, registry, caching |
| 22 | Authentication | Local, MFA/TOTP, passkeys, OIDC |
| 23 | Secrets | Envelope encryption, KMS, rotation |
| 24 | Audit | Tamper-evident hash chain, partitioned, retention |
| 30 | Settings | Schema-driven, scoped, secret references, UI generation |
| 31 | Async Substrate | Jobs, outbox, event bus port, scheduler |
| 32 | Notifications | Channels as Integration Providers, templates, preferences |
| 40 | API | REST, OpenAPI, idempotency, tenant routing |
| 41 | Agent | gRPC stream transport, gateway tier, security |
| 42 | Integration Provider Contracts | Provider traits per Infra module, vendor adapters, Infra→Infra rule |
| 50 | **Frontend & Mobile-First** | Vue 3 + PrimeVue, responsive — deep |
| 51 | Observability | Prometheus, OTel, Grafana, metrics boundary |
| 60 | Versioning & Upgrades | SemVer, migrations, rollback |
| 61 | Testing Strategy | Pyramid, tenant isolation, security tests |
| 62 | Development Workflow | Git, Conventional Commits, CI |

## Recommended Reading & Build Order

Same 22 documents, grouped into implementation phases — this is the order to read (and build) in, top to bottom.

```text
Phase 1 — Foundation
  00  Overview & Principles
  11  Kernel
  12  Tenancy
  10  Repository & Build

Phase 2 — Core Services
  13  Module System
  30  Settings
  24  Audit
  21  RBAC / Authorization
  22  Authentication
  23  Secrets

Phase 3 — Async & Communication
  31  Async Substrate
  32  Notifications

Phase 4 — Boundary & Domain
  14  Database
  41  Agent
  40  API
  42  Integration Provider Contracts

Phase 5 — Experience
  50  Frontend & Mobile-First
  51  Observability

Phase 6 — Operate & Ship
  60  Versioning & Upgrades
  61  Testing Strategy
  62  Development Workflow

(20 — Security spans every phase; read it alongside Phase 1–2 and
 re-check it against each subsequent phase, per its "Security is a
 Core responsibility" invariant.)
```

## Invariants (apply everywhere)

- Core → Infrastructure → Business. Core never depends on Infra or Business.
- Infrastructure → Infrastructure is event-driven by default; a direct dependency is allowed only if declared, validated at boot, and documented (doc 00).
- Every tenant-scoped row carries `organization_id`; isolation enforced by RLS (doc 12).
- No module implements its own auth, authz, secrets, scheduler, or settings store.
- Cross-module references are by UUID; integrity maintained via events, not cross-module FKs.
- A provider is not a module — a module owns a domain, a provider implements access to an external system for it (doc 42).
- Security is a Core responsibility, enforced centrally, the same for every module.
- The UI is mobile-first and permission-aware by default.
