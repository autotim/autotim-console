# 13 — Module System

## Model

Modules are **compile-time Rust crates** that implement a stable `Module` trait from `autotim-sdk`. There is no runtime code installation. Runtime control is **enable / disable**, persisted in `module_state`.

This resolves the v1 contradiction (runtime install/uninstall vs. Rust crates) decisively in favor of the model that fits a single-binary self-hosted platform.

## The `Module` Trait

```rust
#[async_trait]
pub trait Module: Send + Sync {
    fn manifest(&self) -> &ModuleManifest;

    /// Schema changes owned by this module. Run on install/upgrade.
    fn migrations(&self) -> &[Migration];

    /// Permissions this module contributes to the RBAC registry.
    fn permissions(&self) -> &[PermissionDef];

    /// Settings schema injected into the central Settings UI.
    fn settings_schema(&self) -> Option<SettingsSchema>;

    /// REST routes, mounted under the module's namespace.
    fn routes(&self, ctx: &ModuleContext) -> axum::Router;

    /// Event subscriptions.
    fn subscriptions(&self) -> &[EventType];

    /// Background job handlers registered with the Core async substrate.
    fn jobs(&self) -> &[JobHandler];

    /// Frontend manifest (navigation, routes, widgets) — see doc 50.
    fn frontend(&self) -> Option<FrontendManifest>;

    async fn on_enable(&self, ctx: &ModuleContext) -> Result<()>;
    async fn on_disable(&self, ctx: &ModuleContext) -> Result<()>;
    async fn health(&self, ctx: &ModuleContext) -> HealthReport;
}
```

`ModuleContext` hands the module only ports (traits): `Authorizer`, `SecretStore`, `EventBus`, `JobQueue`, `Notifier`, `AuditSink`, `SettingsStore`, `TenantContext`, and a scoped `PgPool` handle. A module never receives another module's internals.

## Module Manifest

```rust
pub struct ModuleManifest {
    pub name: &'static str,         // "dns"
    pub version: &'static str,      // SemVer
    pub layer: Layer,               // Core | Infrastructure | Business
    pub description: &'static str,
    pub dependencies: &'static [&'static str], // module names
    pub sdk_version: &'static str,  // compatibility contract
    pub tenant_scoped: bool,        // does it own org-scoped data?
}
```

`dependencies` are validated against the dependency law at startup: a Core module declaring an Infrastructure dependency fails the build check in CI and refuses to start.

## Lifecycle (simplified, no runtime code install)

```text
Compiled-in
    │  (present in the binary)
    ▼
Migrated      ── run owned migrations (idempotent, reversible)
    ▼
Enabled       ── on_enable: register routes, subs, jobs, frontend, perms
    ▼
Running       ── serves requests, handles events, runs jobs
    ▼
Disabled      ── on_disable: unmount routes/subs/jobs; data preserved
```

- **Enable/disable** flips a row in `module_state` and (de)registers the module's runtime surface. Data and configuration are preserved on disable.
- Upgrade = new binary version → run new migrations → health-validate → resume. Rollback path defined per migration (doc 60).
- There is **no uninstall-at-runtime**. Removing a module means a release that no longer compiles it in; its tables are handled by an explicit data-retention migration.

Every lifecycle transition emits an audit event (`ModuleEnabled`, `ModuleDisabled`, `ModuleUpgraded`).

## Module Registry

The registry is the in-memory + persisted source of truth:

- `modules` — name, version, layer, sdk_version, enabled.
- `module_state` — enabled/disabled, enabled_at, enabled_by.
- `module_migrations` — applied migrations per module, with checksum.

At boot the kernel: loads compiled modules → validates dependency law + SemVer compatibility → runs pending migrations for enabled modules → registers permissions into the RBAC registry → mounts routes/subscriptions/jobs/frontend for enabled modules.

## Dependency & Compatibility Validation

At boot the kernel fails fast on:

- a dependency that is not compiled in or not enabled,
- a dependency-direction violation (layer law),
- an `sdk_version` mismatch (module built against an incompatible SDK),
- a circular dependency.

## Cross-Module Integrity Rule

Modules reference each other's entities **by UUID only**. There are no enforced foreign keys across module boundaries. Integrity is maintained by the owning module plus events:

```text
Hosts emits HostDeleted
    → DNS subscribes, reacts (orphan zones flagged/removed per policy)
```

This is what makes a module disable-able without breaking others, and keeps migrations independent. Within a single module, normal FKs are used freely.
