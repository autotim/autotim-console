# Autotim Console — Architecture Improvement Plan

**Companion to:** `ARCHITECTURE_REVIEW.md`
**Goal:** Turn the review findings into concrete, sequenced decisions and document changes — *before* any code is written.
**Principle:** Optimize for long-term modularity, self-hosted simplicity, and enterprise/multi-tenant readiness. Make the cheap-now/expensive-later decisions now; defer heavy distributed machinery behind clean interfaces.

---

## Part A — Five Decisions to Lock Before Coding

Each decision below lists **current design → proposed design → pros → cons → migration impact → compatibility impact.**

### Decision 1 — Tenancy

- **Current:** No tenant concept. UUIDs offered as the scalability answer. Multi-tenancy listed as "future."
- **Proposed:** Introduce an `organizations` entity now. Add an `organization_id` (UUID) column convention to **every tenant-scoped table** (users, roles, role grants, audit, secrets, hosts, agents, jobs, settings-with-module-scope, events). Enforce isolation with **Postgres Row-Level Security**. Ship v1 as single-tenant by seeding one default organization; the column and RLS exist from day one.
- **Pros:** Reseller/Customer-Portal become additive, not a rewrite. Clean isolation. Works for self-hosted single-tenant *and* future SaaS.
- **Cons:** Slightly more schema/query discipline up front; RLS adds a small mental overhead.
- **Migration impact:** Trivial if done now (greenfield). Effectively impossible to do cheaply later (backfill + uniqueness + every query).
- **Compatibility impact:** None now; protects all future modules.

### Decision 2 — Module Model

- **Current:** Doc 04 implies runtime install/uninstall; doc 16 implies compile-time Rust crates; doc 00 claims "independently deployable." Mutually inconsistent.
- **Proposed:** **Explicit modular monolith.** One deployable binary. Modules are **compile-time Rust crates** behind a stable internal `Module` trait. Runtime control is **enable/disable** via feature flags + a `module_state` table — *not* runtime install/uninstall of code. Reframe the lifecycle state machine accordingly (Registered → Migrated → Enabled → Running → Disabled). Reserve WASM/out-of-process plugins as a *named future epic*, not a v1 assumption. Remove "independently deployable" from doc 00.
- **Pros:** Single-binary self-hosted UX (a feature). No WASM/FFI safety/perf cost. In-process bus & RBAC stay valid. Far smaller maintenance surface.
- **Cons:** Adding/removing a module requires a release (acceptable for self-hosted). True third-party runtime plugins deferred.
- **Migration impact:** Simplifies the lifecycle and SDK docs; removes the runtime-migration-of-untrusted-code machinery.
- **Compatibility impact:** The `Module` trait + manifest remain the stable contract; future WASM host can implement the same contract.

### Decision 3 — Asynchronous Substrate

- **Current:** Separate queue/retry/DLQ in Events, Jobs, Notifications. In-process bus "swappable to broker with no code change." No outbox.
- **Proposed:**
  1. **One durable async substrate** (queue + worker pool + retry/backoff + DLQ). **Notifications and event delivery are built on it** (notification = job type; event fan-out = jobs).
  2. **Transactional outbox** mandatory: event rows written in the same DB transaction as the state change; a relay publishes them.
  3. **Event bus as a port (trait)** with two adapters: **in-process** (single-node) and **broker** (NATS/Redis Streams) for HA/multi-node. Deployment tier chooses the adapter.
- **Pros:** One correctness model instead of three. At-least-once becomes *real*. Horizontal scaling is a config/adapter change, not a rewrite.
- **Cons:** Slightly more design up front (outbox + ports).
- **Migration impact:** Folds doc 13 (Notifications delivery) and doc 05 (Event delivery) onto doc 12 (Jobs) infrastructure.
- **Compatibility impact:** Module-facing API (`publish`, `subscribe`, `enqueue`) stays stable across adapters.

### Decision 4 — Authorization Model

- **Current:** RBAC with resource IDs embedded in permission strings (`dns.zone.read:autotim.de`); no caching; inconsistent naming.
- **Proposed:** **RBAC + scoped grants.** A grant is `(subject, role_or_permission, scope)` where `scope ∈ {tenant, group, resource_type, resource_id}` and always carries `organization_id`. Maintain a **canonical permission registry** (typed constants, no free-form strings at call sites) with a **CI linter** enforcing `module.resource.action`, lowercase, dot-separated. Add **decision caching** per `(user, tenant)` with event-driven invalidation (`RoleAssigned`/`PermissionRevoked` bust the cache).
- **Pros:** Scales with resources; expresses delegated admin ("all zones for customer X"); composes with tenancy; cacheable; fewer authz bugs.
- **Cons:** The grant/scope model is a bit more design than flat strings.
- **Migration impact:** Replaces the `permission:resource` string hack before any module adopts it.
- **Compatibility impact:** Authorization is a Core service contract; modules call `authorize(subject, permission, scope)` — stable regardless of internal model growth.

### Decision 5 — Secrets Root-Key & Storage Boundaries

- **Current:** "Encryption at rest, master key protection" with no key location/unseal; metrics implied in Postgres.
- **Proposed:**
  - **Envelope encryption** (per-secret DEK encrypted by a KEK) with a **pluggable key provider**: dev (passphrase-derived KEK + unseal-on-start), OS-keystore/TPM, external KMS / HashiCorp Vault.
  - **`SecretRotated` event** so consumers refresh on rotation.
  - **Storage boundary, explicit:** Postgres = control-plane metadata; **TSDB (Prometheus/VictoriaMetrics) = metrics**; object store = large blobs/backups; broker (when enabled) = events. Agent metrics never land in Postgres.
- **Pros:** Encryption-at-rest is real; enterprise KMS path exists; metrics scale.
- **Cons:** Key-provider abstraction + unseal flow to design.
- **Migration impact:** Defines the secret schema and the metrics boundary before modules emit data.
- **Compatibility impact:** Secret API (`get/put/rotate`) stable across providers.

---

## Part B — Target Architecture (One Picture)

```text
                 ┌───────────────────────────────────────────────┐
                 │            Autotim Console (single binary)     │
                 │                                                │
   OIDC/Local ─▶ │  Auth ─▶ RBAC(scoped grants + cache) ─▶ API    │
                 │                    │                           │
                 │   Core services:   ▼                           │
                 │   Settings · Secrets(envelope+KMS) · Audit     │
                 │   (tamper-evident, partitioned)                │
                 │                    │                           │
                 │   Async substrate (queue+worker+retry+DLQ)     │
                 │     ├─ Jobs                                     │
                 │     ├─ Notifications (channel adapters)        │
                 │     └─ Event delivery (outbox → bus PORT)      │
                 │            bus port → [in-proc | NATS/Redis]   │
                 │                    │                           │
                 │   Compile-time modules (enable/disable):       │
                 │     Hosts · Agent-core │ DNS · SSL · Mail ...   │
                 └─────────┬───────────────────────┬──────────────┘
                           │                        │
        Postgres (metadata, RLS by org_id)   Agent Gateway tier
                           │                  (agent-initiated gRPC
        TSDB (metrics) · Object store (blobs/backups)  streams, sharded,
                                              backoff+jitter on reconnect)
```

Single node for ≤100 servers; the gateway tier, broker adapter, read replicas, and TSDB scale out independently as you approach 1k–10k.

---

## Part C — Per-Document Change List

| Doc | Change required | Driver |
|-----|-----------------|--------|
| 00 core-module-architecture | Remove "independently deployable"; state "modular monolith, single binary." Note Hosts/Agent/Notifications are domain/integration modules consciously placed in Core (or reclassify). | F2/F3/F17 |
| 01 core-dependency-rules | Add a tenancy boundary note; add "cross-module references by UUID, integrity via events" rule. | F1/F13 |
| 02 module-registration-contract | Manifest gains `organization_scope`, typed-permission references, outbox/event declarations. | F1/F5/F16 |
| 03 core-database-architecture | Add `Organization` entity; add `organization_id` to tenant-scoped entities; add `Outbox`, `EventStore`, `ModuleState` tables; state RLS. | F1/F5 |
| 04 module-lifecycle | Reframe to compile-time module + enable/disable; drop runtime install/uninstall; keep migrate/enable/disable/upgrade. | F2 |
| 05 event-bus | Add transactional outbox; define bus as a port with in-proc + broker adapters; mark replay/DLQ/versioning as deferred machinery behind the port. | F4/F5/F11 |
| 06 rbac | Replace `permission:resource` with scoped grants incl. `organization_id`; add registry + linter + decision caching; fix naming. | F1/F7/F14/F16 |
| 07 agent | Specify agent-initiated gRPC stream transport, scalable gateway tier, reconnect backoff/jitter, heartbeat batching. | F12 |
| 08 api | Add idempotency keys for mutating endpoints; REST+OpenAPI now, GraphQL/gRPC deferred; tenant resolution in routing. | F1/F15 |
| 09 settings | Confirm secret-reference indirection; add tenant scope level. | F1/F8 |
| 10 secrets | Add envelope encryption, pluggable KEK/KMS, unseal flow, `SecretRotated` event. | F8 |
| 11 database-conventions | Add `organization_id` convention + RLS; state cross-module integrity rule (UUID refs, no enforced cross-module FKs); partitioning for audit/events. | F1/F6/F13 |
| 12 job-system | Becomes the single async substrate; add scheduler singleton (leader election / advisory lock); host notification + event-delivery job types. | F10/F11 |
| 13 notifications | Channel providers become pluggable adapters; delivery runs on the Jobs substrate; thin Notification core stays in Core. | F11/F17 |
| 14 security | Add control-plane DR (master key recovery), idempotency, rate-limit placement, correlation-ID propagation through async. | F15/F18 |
| 15 frontend | Keep; add supply-chain note for any future UI plugins (sandbox/sign) tied to Decision 2. | F2 |
| 16 plugin-sdk | Rewrite around compile-time `Module` trait + manifest; WASM/marketplace explicitly future. | F2 |
| 17 versioning | Clarify compile-time vs runtime version semantics under modular monolith; module compat = build-time. | F2 |
| 18 observability | Make metrics-store boundary explicit (TSDB, not Postgres); correlation IDs across events/jobs/agents. | F9 |
| 19 testing | Keep. Add: multi-tenant isolation tests, outbox/idempotency tests, authz-scope tests. | F1/F5/F7 |
| 20 development-workflow | Keep. Add architecture-compliance CI checks (dependency direction, permission linter). | F16 |

---

## Part D — Proposed Commit Plan (documentation first, no code yet)

> Per project rules: small, logical, Conventional-Commits, English. This phase only edits architecture docs.

1. `docs(architecture): add architecture review report and improvement plan`
2. `docs(core): adopt modular-monolith model; remove independently-deployable claim` *(Decision 2; docs 00, 16, 17, 04)*
3. `docs(tenancy): introduce organization model and org_id/RLS convention` *(Decision 1; docs 01, 03, 11, 08, 09)*
4. `docs(rbac): replace resource-string permissions with scoped grants and registry` *(Decision 4; doc 06)*
5. `docs(async): unify jobs/notifications/events and add transactional outbox` *(Decision 3; docs 05, 12, 13)*
6. `docs(secrets): add envelope encryption, pluggable KMS, and rotation events` *(Decision 5; doc 10)*
7. `docs(agent): define agent-initiated transport and scalable gateway tier` *(F12; doc 07)*
8. `docs(observability): separate metrics TSDB from control-plane Postgres` *(F9; docs 18, 11)*
9. `docs(api): add idempotency keys and tenant-aware routing` *(F15/F1; doc 08)*

For larger items (Decisions 1–3), I will propose the detailed doc diffs for approval **before** editing — per the "explain reason / impact / migration / wait for approval" rule.

---

## Part E — Recommended Build Sequence (after docs are approved)

Foundational order, adjusted from doc 00 to put the now-or-never seams first:

1. **Core foundation + tenancy** (`organizations`, `organization_id`, RLS, Module trait, module_state).
2. **Module Registry + Settings** (schema-driven).
3. **Audit** (tamper-evident, partitioned) — needed by everything else.
4. **RBAC (scoped grants + registry + cache)** + **Auth** (generic OIDC, Authentik as a tested provider).
5. **Secrets** (envelope + pluggable KMS).
6. **Async substrate** (Jobs + outbox + scheduler singleton), then **Notifications** and **Event delivery** on top.
7. **Hosts**, then **Agent** (gateway transport).
8. First **Infrastructure** module (e.g. DNS) as the real validation of the whole contract.

---

## Part F — Next Milestone & Remaining Work Before Release

- **Next milestone:** Approve Decisions 1–5; apply doc commits 2–6; produce the detailed diffs for the three foundational docs (00/16/04, 01/03/11, 05/12/13).
- **Remaining before a v1 release is meaningful:** working tenancy + RLS, RBAC scoped grants with cache, Auth (OIDC), Audit (tamper-evident), Secrets (envelope), the unified async substrate with outbox, Hosts + Agent transport, and one end-to-end Infrastructure module proving the plugin contract.
- **Suggested commit message for this deliverable:**
  `docs(architecture): add critical review and improvement plan for Foundation v1`
