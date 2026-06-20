# Autotim Console — Repository Structure

Generated snapshot of the scaffold, in the agreed banded/logical order.

## Architecture documentation (docs/architecture/) — read in this order

```text
00  Overview & Principles            scene-setter, dependency law, golden rules
--- Foundation / Build / Runtime (10–19) ---
10  Repository & Build               workspace, two repos, single binary
11  Kernel                           bootstrap config vs settings, boot order
12  Tenancy                          organizations, org_id, RLS
13  Module System                    Module trait, manifest, lifecycle
14  Database                         conventions, UUID, partitioning, outbox
--- Security & Identity (20–29) ---
20  Security                         threat model, defense in depth
21  RBAC / Authorization             scoped grants, registry, caching
22  Authentication                   local, MFA, OIDC, passkeys
23  Secrets                          envelope encryption, KMS, rotation
24  Audit                            tamper-evident hash chain
--- Core Platform Services (30–39) ---
30  Settings                         schema-driven, scoped, secret refs
31  Async Substrate                  jobs, outbox, event bus, scheduler
32  Notifications                    channels as providers, templates
--- Boundary / Domain (40–49) ---
40  API                              REST, OpenAPI, idempotency
41  Agent                            gRPC stream, gateway tier
42  Integration Provider Contracts   provider traits, vendor adapters
--- Experience & Observability (50–59) ---
50  Frontend & Mobile-First          Vue 3 + PrimeVue, responsive
51  Observability                    Prometheus, OTel, Grafana
--- Process & Lifecycle (60–69) ---
60  Versioning & Upgrades            SemVer, migrations, rollback
61  Testing Strategy                 pyramid, tenant isolation, security
62  Development Workflow             Git, Conventional Commits, CI
```

## Code (crates/) — build order follows the doc phases

```text
autotim-sdk                  stable contract: Module trait + Core ports + types
autotim-kernel               runtime bootstrap + dependency-law validation
autotim-core-tenancy         organizations, org_id, RLS helpers
autotim-core-registry        module registry, manifest validation
autotim-core-settings        schema-driven settings store
autotim-core-audit           tamper-evident audit log
autotim-core-rbac            scoped grants, permission registry, cache
autotim-core-auth            local/MFA/OIDC/passkeys
autotim-core-security        sessions, rate limiting, policy
autotim-core-secrets         envelope encryption, pluggable KMS
autotim-core-async           jobs, outbox, event bus port, scheduler
autotim-core-notifications   channel providers, templates, preferences
autotim-core-hosts           host inventory
autotim-core-agent           agent registration, gRPC gateway
autotim-infra-contracts      shared Integration Provider traits (7 contracts)
autotim-infra-dns            DNS module + DnsProvider adapters
autotim-infra-ssl            SSL module + CertificateProvider adapters
autotim-infra-mail           Mail module + MailProvider adapters
autotim-infra-monitoring     Monitoring + Monitoring/Dashboard providers
autotim-infra-backup         Backup module
autotim-infra-network        Network + OverlayNetworkProvider adapters
autotim                      the `autotim` Community binary (rust-embed)
```

## Frontend (frontend/) — Vue 3 + Vite + PrimeVue, mobile-first

## Other
```text
LICENSE              AGPL-3.0 (full text)
CONTRIBUTING.md      CLA + architecture compliance rules
.github/workflows/   CI (fmt, clippy, build, test, frontend type-check)
migrations/          SQLx migrations (per module)
rust-toolchain.toml  pinned stable toolchain
```
