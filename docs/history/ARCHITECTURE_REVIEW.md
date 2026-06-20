# Autotim Console — Architecture Review Report

**Scope:** Architecture Foundation v1 (documents 00–20)
**Reviewer role:** Principal Architect — self-hosted control planes, infrastructure management platforms, multi-tenant & enterprise systems
**Verdict:** The foundation is well-organized and the dependency discipline (Core → Infra → Business) is sound. However, **several decisions that are cheap to make now and extremely expensive to retrofit later are currently unspecified or wrong.** This document is deliberately critical. It does not confirm the design; it stress-tests it.

---

## 1. Executive Summary

| # | Finding | Severity | Cost to fix now | Cost to fix later |
|---|---------|----------|-----------------|-------------------|
| F1 | **No tenancy model**, yet the roadmap requires Reseller Management + Customer Portal (inherently multi-tenant) | Critical | Low | Catastrophic (touches every table, query, RBAC check, API) |
| F2 | **Runtime plugin lifecycle (install/uninstall live) contradicts compile-time Rust-crate plugin model** | Critical | Low (pick one) | High (architectural fork) |
| F3 | **"Independently deployable modules" contradicts modular-monolith reality** (shared Postgres, in-process bus, in-process RBAC) | High | Low (wording + intent) | High |
| F4 | **In-process event bus cannot span multiple app instances** — breaks the moment you scale horizontally (≈1k+ servers) | High | Medium (clean interface now) | High |
| F5 | **No transactional outbox** — "at-least-once" is not actually guaranteed; events can be lost on crash | High | Low | High (data-integrity bugs) |
| F6 | **Audit log is "immutable" by convention only** — no tamper-evidence, no partitioning, no retention | High | Low | High (compliance + storage) |
| F7 | **RBAC encodes resource IDs into permission strings** (`dns.zone.read:autotim.de`) — this is ABAC/ReBAC pretending to be RBAC; it will not scale or compose with tenancy | High | Medium | Very High |
| F8 | **Secrets: no root-key / unseal / envelope-encryption design** — encryption-at-rest is theater if the master key sits next to the DB | High | Medium | High |
| F9 | **Metrics/time-series implied to live in Postgres** — wrong store; will not survive 1k+ servers | High | Low | High |
| F10 | **Scheduler singleton problem unaddressed** — multiple app instances will fire cron jobs N times | Medium | Low | Medium |
| F11 | **Three independent queue+retry+DLQ stacks** (Jobs, Events, Notifications) — duplicated machinery | Medium (overengineering) | Low | Medium |
| F12 | **Agent transport/connection model undefined at scale** — 10k persistent connections, NAT traversal, reconnect storms | High | Medium | High |
| F13 | **Cross-module foreign keys vs. "removable plugin" promise** are in direct conflict | Medium | Low | High |
| F14 | **No RBAC decision caching** — permission evaluation hits Postgres on every request | Medium | Low | Medium |
| F15 | **No idempotency keys** on mutating APIs while running at-least-once retries | Medium | Low | Medium |
| F16 | **Permission naming is already inconsistent across docs** (`host.` vs `hosts.`, 2-level vs 3-level) | Medium | Low | High (silent authz bugs) |
| F17 | **Hosts, Agent, Notifications placed in Core** stretch the "Core = foundational, domain-agnostic" boundary | Low–Medium | Low | Medium |
| F18 | **No DR/backup story for the control plane itself** (Postgres, secret master key, event store) | Medium | Low | High |

The rest of this report explains each, then analyzes behavior at 10 / 100 / 1,000 / 10,000 servers.

---

## 2. The Single Most Important Decision: Tenancy (F1)

The roadmap repeatedly lists **Reseller Management**, **Customer Portal**, **Billing**, and **CRM** as future Business modules. Three of these are *inherently multi-tenant*. The RBAC document lists multi-tenancy only under "Future Support," and the database document offers UUIDs as the scaling answer.

**UUIDs are not tenancy.** Tenancy is a data-isolation and authorization boundary. If `users`, `roles`, `permissions`, `audit_logs`, `secrets`, `hosts`, `agents`, `jobs`, and `settings` are built without an organization/tenant boundary, then introducing one later means rewriting:

- every table (add `organization_id`),
- every query (filter by tenant),
- every RBAC check (scope to tenant),
- every API route (tenant resolution),
- every audit and secret access path (tenant isolation),
- all existing production data (backfill + uniqueness constraints).

This is one of the most expensive refactors in software, and it is the one this roadmap *guarantees you will need.*

**Recommendation:** Decide the tenancy strategy now, even if you ship single-tenant first. Pick one:

1. **Shared schema + `organization_id` on every tenant-scoped table** (+ Postgres Row-Level Security). Best fit for self-hosted + future SaaS. Cheap to add now, hard to add later.
2. **Schema-per-tenant.** Stronger isolation, heavier operations, harder cross-tenant reporting (resellers need cross-tenant views).
3. **Database-per-tenant.** Strongest isolation, worst density and operations.

For a self-hosted control plane that also wants reseller/portal capabilities, **option 1 with RLS** is almost certainly correct. The minimum viable action today is: introduce the `organizations` concept and a nullable/`default-org` `organization_id` column convention now, so the seam exists. You do not need to build the reseller UI; you need the column and the query discipline.

---

## 3. The Foundational Contradiction: What *Is* a Module? (F2, F3)

The documents describe two incompatible systems simultaneously:

- **Module Lifecycle (doc 04)** describes a **runtime plugin system**: upload a module, run migrations, install, enable, disable, **uninstall live**, with a full state machine.
- **Plugin SDK (doc 16)** describes plugins as **Rust crates** (compile-time), with "signed packages / bundles" as *future*.

These cannot both be true with the current tech choices. Rust crates are linked at build time. You cannot "upload and uninstall" a Rust crate at runtime without one of:

- **WASM plugins** (sandboxed, but huge complexity: host/guest ABI, async, DB access, performance),
- **Dynamic libraries (.so/FFI)** (ABI instability, memory-safety holes, the opposite of why you chose Rust),
- a **separate-process / sidecar** model (then it's microservices, not a monolith, and "in-process event bus" / "in-process RBAC" no longer apply).

Simultaneously, doc 00 claims modules are **"independently deployable,"** while every other document assumes a **modular monolith** (one shared Postgres, in-process event bus, in-process RBAC service, compiled-in crates). A compiled-in crate is *not* independently deployable.

**This is the most important structural decision after tenancy.** My strong recommendation for a self-hosted-first product:

> **Adopt an explicit modular monolith: one deployable binary, compile-time modules, runtime enable/disable via feature flags + a `module_state` table. Drop "independently deployable" and drop runtime install/uninstall.**

Rationale: For self-hosted operators, *a single binary is a feature, not a limitation* (GitLab, Grafana, Gitea all win on this). It removes the WASM/FFI complexity, removes runtime migration-of-untrusted-code risk, and keeps the in-process bus/RBAC valid. You keep modularity through **compile-time module boundaries + enable/disable flags**, which delivers ~95% of the benefit at ~20% of the cost. Reserve true out-of-process extensibility (WASM marketplace) as a genuine future epic, not a v1 assumption.

If you genuinely require third-party runtime plugins from day one, then you must accept the WASM-host investment now and rewrite the SDK doc around it — but I would advise against it for v1.

---

## 4. Asynchronous Substrate: Bus, Jobs, Notifications (F4, F5, F11)

### 4.1 Three stacks, one problem (F11 — overengineering)
The Event Bus (doc 05), Job System (doc 12), and Notifications (doc 13) **each** independently specify: a queue, workers, retry with exponential backoff, and a dead-letter queue. That is three implementations of the same durable-async machinery.

**Recommendation:** Build **one** durable async substrate (a queue + worker pool + retry + DLQ), then model the others on top:
- **Notifications = a job type** ("deliver notification via channel X").
- **Event delivery = jobs** fanned out from the event store (with the outbox below).

This collapses three maintenance surfaces into one and makes correctness reasoning tractable.

### 4.2 At-least-once is not free — you need the outbox (F5)
Doc 05 promises at-least-once delivery and idempotent consumers (good), but never specifies **how the event is reliably published**. The naive pattern — write state to DB, then publish to the bus — loses events if the process crashes between the two. For a control plane, a lost `CertificateExpired` or `AgentDisconnected` is an operational incident.

**Recommendation:** Mandate the **transactional outbox pattern**: the event row is written **in the same DB transaction** as the state change, and a relay publishes it afterward. This is the only way the at-least-once promise is real. Document it as a Core requirement.

### 4.3 The in-process bus has a hard ceiling (F4)
Doc 05 says "initial implementation may use In-Process Event Bus … future upgrades may use Redis Streams/NATS/Kafka **without changing module code**." The *interface* promise is achievable; the *implementation swap* is not transparent:

- An **in-process bus only delivers to subscribers in the same OS process.** The moment you run two app instances behind a load balancer (needed at ~1k servers for HA, and earlier for availability), an event published on instance A never reaches a subscriber that happens to be active on instance B. **In-process event bus and horizontal scaling are mutually exclusive.**

**Recommendation:** Define the event-bus **port (interface)** rigorously now, ship an in-process **adapter** for single-node deployments, and ship a broker adapter (NATS/Redis Streams) for HA/multi-node. Make this an explicit deployment tier, not a "later, no code change" hand-wave. The outbox (4.2) is what makes both adapters reliable.

---

## 5. RBAC: Wrong Model for the Stated Requirements (F7, F14, F16)

### 5.1 Resource IDs in permission strings is an anti-pattern (F7)
Doc 06 gives `dns.zone.read:autotim.de` and `host.manage:mail-server`. Encoding the *resource instance* into the permission string means:

- the permission space grows with the number of zones/hosts/customers (unbounded),
- you cannot express "all zones owned by customer X" or "all hosts in group Y,"
- it does not compose with tenancy (the customer dimension is missing),
- revocation and listing ("what can this user touch?") become full scans.

This is not RBAC; it is ad-hoc ABAC. **Decide the authorization model explicitly:**

- **RBAC + scoped grants:** a grant is `(subject, role/permission, scope)` where scope is `tenant | group | resource_type | resource_id`. Clean, cacheable, covers delegated admin.
- **ReBAC (Zanzibar-style):** relationship tuples (`user:bandit#editor@zone:autotim.de`). Most powerful, heaviest to build. Justified only if fine-grained sharing becomes central.

For this platform, **RBAC + scoped grants, with the scope including `organization_id`,** is the right balance. Get the *shape* right now; the engine can grow.

### 5.2 Caching (F14)
Every request runs Authentication → load roles → load permissions → evaluate. Hitting Postgres for this on every API call is a bottleneck and a coupling point. **Recommendation:** cache resolved permission sets per (user, tenant) with **event-driven invalidation** (`RoleAssigned`, `PermissionRevoked` → bust cache). This is a clean, motivating first real use of the event bus.

### 5.3 Naming is already inconsistent (F16)
Across docs: `host.agent.restart` (00) vs `hosts.agent.restart` (06) vs `host.manage:mail-server` (06); `module.resource.action` is stated but examples violate arity and singular/plural. A typo'd permission silently denies (or worse, a missing check silently allows). **Recommendation:** a single canonical registry, a compile-time/CI **linter** enforcing the convention, and permissions declared as typed constants — never free-form strings at call sites.

---

## 6. Secrets: Encryption-at-Rest Needs a Root-Key Story (F8)

Doc 10 says "encryption at rest, master key protection, key rotation" but never says **where the master key lives or how it is unsealed.** On a self-hosted single box, if the master key sits in a config file or env var beside the database, an attacker who reads the disk reads everything — encryption-at-rest becomes theater.

**Recommendation:** Specify **envelope encryption** (per-secret DEK encrypted by a KEK) and a **pluggable key provider** with at least:
- a dev/simple provider (passphrase-derived KEK, unseal on start),
- an OS-keystore / TPM option,
- an external KMS / HashiCorp Vault option (already on the roadmap).

Also specify **rotation propagation**: when a secret rotates, how do referencing settings/consumers refresh? (Event `SecretRotated` → consumers re-fetch.) Without this, rotation breaks live integrations.

---

## 7. Data & Storage: One Postgres for Everything Won't Hold (F9, F13, F18)

### 7.1 Time-series does not belong in the relational DB (F9)
Agent metrics collection and "Collect Metrics" jobs (docs 07, 12) imply metric data flowing into the platform. Doc 18 correctly mentions Prometheus for `/metrics` scraping — **keep platform metrics there (or VictoriaMetrics/Mimir).** Never store agent time-series in Postgres; it will dominate write volume and storage and kill query performance by ~1k servers. Make the boundary explicit: **Postgres = control-plane metadata; TSDB = metrics; object store = large blobs/backups; (optional) broker = events.**

### 7.2 Cross-module FKs vs. removable plugins (F13)
Doc 11 mandates explicit foreign keys; docs 00/04 promise modules are removable without breaking others; doc 08/11 also say "avoid cross-module coupling, prefer APIs/views." These conflict: a hard FK from `dns.zones.host_id → hosts.hosts.id` makes Hosts non-removable without breaking DNS, and couples their migrations.

**Recommendation:** Pick a rule and state it: **cross-module references are by UUID without enforced FKs**, integrity maintained by the owning module + events (`HostDeleted` → DNS reacts), *or* declare certain Core modules (Hosts, Agent) as **hard platform dependencies** that are never removable (which contradicts "everything is a plugin"). Be explicit; do not leave it implied.

### 7.3 Control-plane DR (F18)
There is a Backup *module* for managed infrastructure, but nothing about backing up the **control plane's own** Postgres, **secret master key**, and event store. For enterprise/self-hosted this is mandatory. Document RPO/RTO and how the master key is recovered without exposing plaintext.

---

## 8. Agents at Scale: The Transport Is Undefined (F12)

Doc 07 has a solid *security* posture (mTLS, token rotation, capability validation) but is silent on the **connection model**, which is the hard scaling problem:

- Self-hosted customers run agents behind NAT/firewalls → **agents must dial out** and hold a persistent channel (gRPC bidirectional stream or WebSocket); the control plane cannot reach in.
- 10k persistent connections **cannot terminate on a single node** → you need a horizontally scalable **agent-gateway tier** with fan-out to the core.
- Control-plane restart triggers **10k simultaneous reconnects** (thundering herd) → mandatory **jitter + exponential backoff** on agents.
- Heartbeat storms → batch/aggregate; do not write every heartbeat to Postgres synchronously.

**Recommendation:** Specify the transport (recommended: agent-initiated gRPC stream), the gateway-tier scaling model, reconnect backoff/jitter, and heartbeat write-batching now. These are abstractions, not implementations — but if they are wrong, the 1k→10k transition is a rewrite.

---

## 9. Other Correctness & Maintainability Gaps

- **Scheduler singleton (F10):** With ≥2 app instances, who runs cron? If all do, jobs fire N times. Mandate a **distributed lock / leader election** for the scheduler (Postgres advisory lock is fine at small scale).
- **Idempotency keys (F15):** At-least-once + job retries mean mutating endpoints (job submission, cert renewal, agent commands) need client-supplied idempotency keys to avoid duplicate side effects.
- **Core boundary creep (F17):** **Hosts, Agent, Notifications** are domain/integration concerns sitting in Core. Notifications bakes Telegram/Discord/Matrix/SMTP integrations *into Core*, growing Core's surface unboundedly. Recommendation: keep a thin **Notification core** (queue, templates, preferences, dispatch port) in Core, and make **channel providers pluggable adapters**. Reconsider whether Hosts/Agent are truly Core or the first *Infrastructure* modules; if kept in Core, acknowledge they are domain modules and accept the precedent consciously.
- **Audit feedback loops:** If every event publication is audited and audit writes can emit events, you risk loops and write amplification. Define precisely what is audited and ensure audit never re-enters the bus.
- **Tracing across async boundaries:** `request_id` is mentioned, but correlation IDs must **propagate through events, jobs, and agent commands**, or distributed tracing is useless exactly where you need it.
- **OIDC provider naming:** Auth names "Authentik OIDC" specifically. Keep the *implementation* generic OIDC; enterprises will demand Entra ID / Okta / Keycloak / Ping. Authentik is a tested provider, not a coupling.
- **API surface (overengineering):** REST + GraphQL + gRPC all promised. Ship **REST + OpenAPI** only; defer the rest behind the contract.

---

## 10. Scale Analysis: 10 / 100 / 1,000 / 10,000 Servers

### 10 servers — *the design's sweet spot*
- **Works well:** Single binary, single Postgres, in-process bus, Postgres job queue, single control-plane node, agents with persistent connections. Modular monolith is ideal here. Operationally simple — a genuine self-hosted advantage.
- **Watch:** The risks at this tier are **correctness, not capacity.** Get the cheap-now/expensive-later items right *here*: tenancy column, outbox, audit tamper-evidence + partitioning, scheduler singleton, metrics in a TSDB, RBAC scope shape, permission linter.
- **Reproject:** Nothing structurally.

### 100 servers
- **Works well:** Still comfortable on one strong node. 100 agent connections trivial.
- **Becomes problematic:** Audit/event tables grow — begin **time-partitioning + retention** now. RBAC per-request DB hits start to matter — add **caching**. If Customer Portal/Reseller has launched, **tenancy must already exist** (see F1).
- **Reproject:** None, if the seams are in place.

### 1,000 servers
- **Works well:** Stateless app tier scales horizontally behind a load balancer — *if* it is actually stateless.
- **Becomes problematic / must reproject:**
  - **In-process event bus breaks** the instant you run >1 instance (F4) → switch to broker adapter.
  - **Scheduler fires N× ** without leader election (F10).
  - **Single Postgres** feels job-queue contention and audit/event volume → consider read replicas, dedicated pools, partition/retention enforced.
  - **Agent gateway** needs to be a scalable tier, not the app node (F12).
  - HA (active/passive at least) becomes a requirement, not a nicety.
- **Introduce:** Message broker, RBAC decision cache, time-partitioned audit/events with retention, dedicated metrics TSDB (should already exist), connection-gateway tier.

### 10,000 servers
- **Must reproject if seams weren't designed:**
  - **Distributed agent-gateway fleet** with fan-out; persistent-connection sharding (F12).
  - **Real broker** (NATS/Kafka) as the only viable event transport (F4/F5; outbox essential).
  - **TSDB at scale** (VictoriaMetrics/Mimir) — Postgres for metrics is dead here (F9).
  - **Control-plane Postgres**: read replicas mandatory; partitioning/archival for audit & events; possibly functional sharding by domain or by tenant.
  - **Multi-tenancy with real isolation** is non-negotiable for resellers (F1).
  - **RBAC** needs cached decisions and possibly a dedicated authz service / Zanzibar-style store (F7).
  - **Dedicated job-execution fleet** separate from the API tier.
- **The whole game:** If the *abstractions* (event port, job port, secret/key provider, tenancy column, agent transport, metrics boundary) are right from day one, this tier is **swap-implementations**, not **rewrite-callers**. If they are wrong, it is a rewrite. That is precisely why the v1 decisions in §2–§8 matter now.

---

## 11. What's Missing / Overengineered / Underengineered (Summary)

**Missing (underengineered):**
Tenancy (F1) · transactional outbox (F5) · audit tamper-evidence + retention/partitioning (F6) · secrets root-key/unseal/envelope (F8) · metrics storage boundary (F9) · agent transport & gateway scaling (F12) · scheduler singleton (F10) · idempotency keys (F15) · RBAC scope model + caching (F7/F14) · control-plane DR (F18) · cross-module integrity rule (F13).

**Overengineered (for a 10–100 server v1):**
Runtime plugin install/uninstall lifecycle when compile-time modules + enable/disable suffice (F2) · three separate queue/retry/DLQ stacks (F11) · event replay + DLQ + event versioning machinery at v1 (build the interface, defer the machinery) · REST + GraphQL + gRPC all at once.

**Hard to maintain over time:**
Resource IDs in permission strings (F7) · inconsistent permission naming with no linter (F16) · cross-module FKs in a "pluggable" system (F13) · duplicated async substrates (F11) · WASM/FFI runtime-plugin complexity if pursued (F2) · Core surface growth via in-Core channel integrations (F17).

**What is genuinely good (keep):**
The Core → Infra → Business dependency law and its examples · UUID primary keys · centralized Settings/Secrets/Audit/RBAC intent · event-driven decoupling *as a principle* · schema-driven settings UI · permission-aware frontend with error boundaries and lazy loading · SemVer + migration discipline · the testing pyramid and Conventional-Commits workflow (docs 17/19/20 are solid and low-risk).

---

## 12. Closing Assessment

The skeleton is good and the discipline is above average for a v1. The danger is not in what the documents say — it is in what they **leave unsaid at the exact seams that are cheap to get right now and ruinous to retrofit later.** Five decisions dominate everything else and should be made *before* code:

1. **Tenancy** (shared schema + `organization_id` + RLS).
2. **Module model** (modular monolith, single binary, compile-time modules + enable/disable; drop runtime install/uninstall and "independently deployable").
3. **Async substrate** (one durable queue + outbox; event-bus as a port with in-process and broker adapters).
4. **Authorization model** (RBAC + scoped grants including tenant; canonical permission registry + linter; decision caching).
5. **Secrets root-key & metrics-store boundaries** (envelope encryption + pluggable KMS; TSDB for metrics, never Postgres).

Make these five now, and the 10→10,000 path becomes evolution rather than rewrite. The detailed, sequenced plan is in `ARCHITECTURE_IMPROVEMENT_PLAN.md`.
