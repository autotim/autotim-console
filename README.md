# Autotim Console

A modular, self-hosted infrastructure management platform.
Philosophy peers: Proxmox, Portainer, aaPanel, Coolify, Plesk.

> **Status:** scaffold. This repository contains the architecture foundation
> (v2) and a compiling workspace skeleton. Core services and modules are not
> implemented yet — see `docs/architecture/` for the design and the build
> sequence.

## Editions

| Edition | Repository | Binary | License |
|---------|-----------|--------|---------|
| Community | `autotim/autotim-console` (this repo) | `autotim` | AGPL-3.0 |
| Enterprise | private, not part of this repository | `autotim-ee` | Commercial |

> The `autotim/autotim` repository is reserved as the organization profile
> README only — it is **not** a code repository. See
> `docs/architecture/10-repository-and-build.md`.

## Stack

- **Backend:** Rust · Axum · Tokio · SQLx · PostgreSQL
- **Frontend:** Vue 3 · Vite · PrimeVue · Pinia · Vue Router · TypeScript (mobile-first)
- **Packaging:** single binary — the Vue build is embedded via `rust-embed`
- **Observability:** Prometheus · OpenTelemetry · Grafana

## Repository Layout

```text
crates/                  Rust workspace (Core, Infrastructure, SDK, binary)
  autotim-sdk/           stable module contract: Module trait + Core ports
  autotim-kernel/        runtime: config, registry, dependency-law validation
  autotim-core-*/        Core modules (tenancy, rbac, auth, secrets, async,
                          notifications, settings, audit, hosts, agent, …)
  autotim-infra-*/        Infrastructure modules (dns, ssl, mail, monitoring, …)
  autotim-infra-contracts/ shared Integration Provider traits (doc 42)
  autotim/               the `autotim` Community binary
frontend/                Vue 3 + Vite app (embedded into the binary)
config/                  Bootstrap config templates (.example.toml only — see docs/security/)
deploy/systemd/          systemd unit + environment file templates (.example only)
docs/architecture/       Architecture Foundation v2 (start at INDEX.md)
docs/security/           Public/private deployment boundary, security checklist
docs/history/            v1 critical review + improvement plan (design rationale)
migrations/              SQLx migrations (per module)
```

## Building

```bash
# Config (required — the binary refuses to start without it)
cp config/autotim.example.toml config/autotim.toml
# edit config/autotim.toml: point database.url at your local Postgres

# Frontend (produces frontend/dist, embedded by the binary)
cd frontend && pnpm install && pnpm build && cd ..

# Backend
cargo build --release
AUTOTIM_CONFIG=config/autotim.toml ./target/release/autotim
```

A placeholder `frontend/dist/index.html` is committed so a clean checkout
compiles before the first real frontend build. See
[`docs/security/public-private-boundary.md`](docs/security/public-private-boundary.md)
for the full configuration model and how production deployments differ.

## Documentation

Architecture lives in [`docs/architecture/`](docs/architecture/INDEX.md). The
five foundational decisions (tenancy, module model, async substrate,
authorization, secrets/storage boundaries) are summarized in the index.

## Contributing

External contributions require a signed CLA. See
[`CONTRIBUTING.md`](CONTRIBUTING.md).

## License

AGPL-3.0-only. See [`LICENSE`](LICENSE).
