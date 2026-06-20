# 10 — Repository & Build Architecture

## Repository Identity

```text
github.com/autotim/autotim-console    public · AGPL v3 · binary: autotim
github.com/autotim/autotim              profile repo only (org README, no code)
```

> **Naming note:** GitHub treats a public repo whose name equals the org name (`autotim/autotim`) as a special **profile README repo** — its `README.md` renders on the organization's public profile page instead of behaving like a normal code repository. The code repository is therefore named `autotim-console`, not bare `autotim`, leaving `autotim/autotim` reserved as the small profile-only repo. The binary name (`autotim`) is unaffected — it is a Cargo `[[bin]]` setting, independent of the repo name.

A commercial Enterprise Edition exists, built and distributed separately under its own license. Its repository, source layout, and implementation are outside the scope of this documentation set.

## Public Workspace (Community)

```text
autotim-console/
├── Cargo.toml                  # workspace root
├── LICENSE                     # AGPL v3
├── CONTRIBUTING.md             # CLA reference
├── crates/
│   ├── autotim-kernel/         # runtime: config, DI, module trait, ports
│   ├── autotim-core-tenancy/
│   ├── autotim-core-registry/
│   ├── autotim-core-settings/
│   ├── autotim-core-audit/
│   ├── autotim-core-rbac/
│   ├── autotim-core-auth/
│   ├── autotim-core-security/
│   ├── autotim-core-secrets/
│   ├── autotim-core-async/     # jobs + outbox + event bus + scheduler (doc 31)
│   ├── autotim-core-notifications/ # channels as providers, templates (doc 32)
│   ├── autotim-core-hosts/
│   ├── autotim-core-agent/
│   ├── autotim-infra-dns/
│   ├── autotim-infra-ssl/
│   ├── autotim-infra-mail/
│   ├── autotim-infra-monitoring/
│   ├── autotim-infra-backup/
│   ├── autotim-sdk/            # stable contracts modules depend on
│   └── autotim/                # the Community binary (bin name: autotim)
├── migrations/                 # workspace-level SQLx migrations (per module subdir)
└── frontend/                   # Vue 3 + Vite (see doc 50)
```

## The `autotim-sdk` Crate

The single **stable contract** every module compiles against. It exposes:

- the `Module` trait and `ModuleManifest`,
- Core service **ports** (traits): `Authorizer`, `SecretStore`, `EventBus`, `JobQueue`, `Notifier`, `AuditSink`, `SettingsStore`, `TenantContext`,
- shared types: `OrganizationId`, `UserId`, `Permission`, `Scope`, error and result types.

Modules depend on `autotim-sdk` and on the *ports*, never on concrete Core crate internals. This is what keeps the dependency law enforceable and lets Core implementations evolve without breaking modules.

## Single-Binary Packaging

The Vue 3 production build is embedded into the Rust binary with `rust-embed`:

```rust
#[derive(rust_embed::RustEmbed)]
#[folder = "../../frontend/dist"]
struct WebAssets;
```

Axum serves embedded assets with SPA fallback (`index.html` for unknown non-API routes). Result: **one executable**, no external web server, no separate frontend deployment.

```text
build pipeline:
  pnpm build (frontend/) → frontend/dist
  cargo build --release  → embeds dist → target/release/autotim
```

## Filesystem Layout (runtime, bare-metal)

```text
/usr/local/bin/autotim            # the binary
/etc/autotim/config.toml          # configuration
/etc/autotim/master.key (or KMS)  # secrets unseal material (see doc 23)
/var/lib/autotim/                 # working data
/var/log/autotim/                 # logs (also stdout for journald)
/etc/systemd/system/autotim.service
```

## Build-Time Module Selection

Editions/features are selected via Cargo features, not runtime loading:

```toml
[features]
default = ["infra-dns", "infra-ssl", "infra-mail", "infra-monitoring"]
infra-dns = ["dep:autotim-infra-dns"]
```

At runtime, the `module_state` table decides which compiled modules are enabled (see doc 13). A module can be compiled-in but disabled.

## Versioning

The workspace is versioned with SemVer (doc 60). All crates in a release share the platform version; the `autotim-sdk` version is the compatibility contract for modules.
