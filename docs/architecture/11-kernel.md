# 11 — Kernel Architecture

## Purpose

Doc 13 (Module System) defines what a module *is* — the contract. This document defines the kernel: the runtime that hosts modules, the thing that actually starts when you run `./autotim`. It closes a gap left in v2: the kernel was implied everywhere (config, boot order, port wiring) but never specified on its own.

## Two Kinds of Configuration (the chicken-and-egg boundary)

A recurring source of confusion is conflating **bootstrap config** with **Settings** (doc 30). They are deliberately different systems, resolved in a strict order:

```text
Bootstrap Config        →  exists BEFORE any database connection
  (file + env vars)         bind address, DB connection string,
                             log level/format, Secrets key-provider
                             selection (doc 23), data directories

Settings                →  exists AFTER the database and Secrets
  (doc 30, DB-backed,        are up: schema-driven, per-organization,
   schema-driven)             validated, audited, UI-generated
```

You cannot store "which database to connect to" *in* the database. Bootstrap config is therefore intentionally minimal — just enough to reach a healthy Postgres connection and an unsealed (or sealable) Secrets store. Everything else (notification channel credentials, provider selection per doc 42, UI theme, password policy) lives in Settings.

```toml
# /etc/autotim/config.toml — bootstrap config, file + env override
[server]
bind = "0.0.0.0:8080"

[database]
url = "postgres://autotim@localhost/autotim"

[secrets]
key_provider = "passphrase"   # passphrase | os-keystore | kms (doc 23)

[logging]
level = "info"
format = "json"
```

Environment variables override file values (`AUTOTIM_DATABASE__URL`, …) for container/systemd-friendly deployment without editing files.

The concrete loading rules (base file + environment overlay + env vars), the startup validator that refuses to boot `production` with an unedited template placeholder, and where real config actually lives on a deployed host are specified in `docs/security/public-private-boundary.md` — that document also defines the public/private repository boundary this config model exists to support. Implementation: `autotim-kernel::config`.

## Boot Sequence

Order matters because later stages depend on earlier ones being healthy. A failure at any stage aborts startup with a clear error — the platform never starts into a half-initialized, silently-degraded state.

```text
1. Load bootstrap config (file + env)
2. Connect to PostgreSQL (fail fast if unreachable)
3. Initialize Secrets (doc 23): construct the configured Key Provider.
   If passphrase-unseal is configured and no unseal material is
   present, the kernel enters "sealed" state — it keeps running
   (so an operator can unseal it) but holds all secret-dependent
   ports inert until unsealed.
4. Construct Core ports: AuditSink, SettingsStore, EventBus (in-proc
   or broker adapter per config), JobQueue, Authorizer (RBAC),
   Notifier, TenantContext factory.
5. Register compiled-in modules (doc 13) with the Module Registry.
6. Validate the dependency law (doc 00) and Infra→Infra rules,
   sdk_version compatibility (doc 60), and tenant_scoped/org_id
   presence for each module.
7. Run pending migrations for enabled modules (doc 14, doc 60).
8. Call Module::on_enable() for each enabled module, in dependency
   order (a module's declared dependencies are enabled first).
9. Mount routes, event subscriptions, jobs, and frontend manifests
   for enabled modules.
10. Start the HTTP server; begin serving traffic.
```

Step 3's sealed-state behavior matters for self-hosted operators: the binary can start (so health checks and an unseal UI/CLI work) without ever holding secrets in an insecure fallback state.

## Port Wiring into ModuleContext

The kernel is the **only** place that constructs concrete implementations of the Core ports (doc 13's `Authorizer`, `SecretStore`, `EventBus`, `JobQueue`, `Notifier`, `AuditSink`, `SettingsStore`). Modules receive them as trait objects via `ModuleContext`, never construct or import them directly — this is what keeps the dependency law and "no module reimplements auth/authz/secrets" rule mechanically enforced rather than aspirational.

```text
Kernel
  ├── constructs PgRbacAuthorizer        → Arc<dyn Authorizer>
  ├── constructs EnvelopeSecretStore     → Arc<dyn SecretStore>
  ├── constructs InProcessEventBus       → Arc<dyn EventBus>   (or NatsEventBus, doc 31)
  ├── constructs PgJobQueue              → Arc<dyn JobQueue>
  ├── constructs JobBackedNotifier       → Arc<dyn Notifier>
  ├── constructs HashChainAuditSink      → Arc<dyn AuditSink>
  └── constructs PgSettingsStore         → Arc<dyn SettingsStore>
        │
        ▼
  ModuleContext { all of the above + scoped PgPool }
        │
        ▼
  passed to every Module::on_enable() / routes() / jobs handlers
```

## Graceful Shutdown & Signal Handling

On `SIGTERM`/`SIGINT`: stop accepting new HTTP connections, let in-flight requests drain (bounded timeout), call `Module::on_disable()` for each enabled module (routes/subscriptions/jobs unmount cleanly), stop the job workers after their current job completes (or re-queue if killed), close the event bus relay, then close the database pool. This is what makes upgrades (doc 60) and host reboots safe rather than abrupt.

## Error Handling & Panic Strategy

- Boot-time errors (steps 1–7 above) are fatal: log a clear, actionable message and exit non-zero. There is no "best effort" boot — a half-validated platform is a security and data-integrity risk.
- Post-boot, a panic inside one module's request handler is caught at the Axum layer and returned as a `500` for that request; it does not crash the process (consistent with doc 50's error-boundary principle on the frontend side — a failing module must not take down the platform).
- A panic inside a background job is caught by the job worker, recorded as a job failure (doc 31 retry/DLQ), and does not crash the worker pool.

## Relationship to Module Registry (doc 13) and Boot Validation (doc 42 §12)

The kernel *drives* the Module Registry (doc 13) and the Integration Provider boot validation (doc 42 §12) — it is the orchestrator; those documents define the rules it enforces. The kernel does not duplicate those rules here; it calls into them at the appropriate boot step (steps 6 and 7 above).

## Invariant

The kernel constructs every Core port exactly once, in a known order, and hands modules only the interface — never the implementation. If the kernel cannot reach a healthy, validated state, it does not pretend to.
