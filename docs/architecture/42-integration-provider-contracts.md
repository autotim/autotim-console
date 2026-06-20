# 42 — Integration Provider Contracts

## 1. Purpose

Infrastructure modules manage technical domains (DNS, certificates, monitoring, mail, ingress, overlay networking) that are, in practice, implemented by external systems: PowerDNS, NetBird, Prometheus, Traefik, Stalwart, and others the community will want tomorrow (Tailscale, Headscale, plain WireGuard, OpenVPN, ZeroTier…).

This document defines how those external systems are integrated **without Core ever knowing they exist**, and without each new vendor becoming a new entry in the Module Registry. It extends doc 13 (Module System) and the Infra→Infra rule added in doc 00.

## 2. Provider vs Module

```text
A provider is not a module.

A module owns a domain.
A provider implements access to an external system for that domain.
```

```text
Network Module (Infrastructure, registered in Module Registry)
└── internal provider registry
    ├── NetBirdProvider
    ├── TailscaleProvider
    ├── HeadscaleProvider
    ├── WireGuardProvider
    └── OpenVPNProvider
```

**Not** `NetBird Module`, `Tailscale Module`, `WireGuard Module` — that would mean module-sprawl: a manifest, permission set, lifecycle, and navigation entry for what is really just a configuration choice. The platform exposes one set of permissions (`network.peer.read`, `network.route.create`, …) regardless of which vendor backs it.

## 3. Provider Registry per Infrastructure Module

Each Infrastructure module owns a small, internal registry of compiled-in providers and exposes one active provider per organization, selected through Settings:

```text
network.provider = netbird | tailscale | headscale | wireguard | openvpn | zerotier
dns.provider      = powerdns | route53 | cloudflare
ssl.provider      = autotim-acme | step-ca
monitoring.provider = prometheus
mail.provider     = stalwart
ingress.provider  = traefik
```

This setting is scoped per organization (doc 12), not a single global flag — in the Enterprise edition, organization A may run NetBird while organization B runs Tailscale, on the same platform instance.

**Credentials are never inline.** A provider's settings store a secret reference (doc 23), never a raw token/URL credential:

```text
network.netbird.api_token  →  secret_ref: <uuid>
```

The Settings UI shows "configured / not configured"; reading the value is a separate, authorized, audited action — identical to every other secret in the platform.

> **v1 scope note:** one active provider per module per organization is the deliberate v1 model. A future need for multiple simultaneous providers per module (e.g. an internal PowerDNS zone and an external Cloudflare zone side by side) would move provider selection from module-level to resource-level (e.g. per-zone). This is a known, accepted forward-compatibility seam, not an oversight — call it out explicitly if/when it becomes a real requirement.

## 4. Capability-Based Provider Interfaces

Providers within the same domain are **not** uniformly capable. NetBird/Tailscale/Headscale expose group and ACL concepts; plain WireGuard or OpenVPN typically do not have a queryable control plane at all. Forcing one flat trait onto all of them produces either silent no-ops or scattered errors.

Instead, every provider trait exposes its capabilities, and methods outside a provider's capability set return a typed "not supported" error rather than being silently ignored:

```rust
pub struct ProviderCapabilities {
    pub groups: bool,
    pub acl: bool,
    pub exit_nodes: bool,
    pub dns_override: bool,
    /// True if the provider can push change notifications (webhook/event)
    /// instead of relying solely on polling sync jobs (see §5, §8).
    pub supports_push: bool,
}
```

```rust
#[async_trait]
pub trait OverlayNetworkProvider: Send + Sync {
    fn capabilities(&self) -> ProviderCapabilities;

    async fn list_peers(&self) -> Result<Vec<Peer>, ProviderError>;
    async fn list_routes(&self) -> Result<Vec<Route>, ProviderError>;

    /// Default: not supported. Providers with a group concept override this.
    async fn list_groups(&self) -> Result<Vec<Group>, ProviderError> {
        Err(ProviderError::NotSupported("groups"))
    }

    /// Optional: providers that support_push implement this to register
    /// their own webhook/event handler instead of relying purely on polling.
    async fn register_push_handler(&self, _ctx: &ModuleContext) -> Result<(), ProviderError> {
        Ok(()) // no-op for poll-only providers
    }

    async fn health(&self) -> ProviderHealth;
}
```

The frontend (doc 50) reads `capabilities()` and hides UI it cannot back (e.g. no "Groups" tab when the active provider is plain WireGuard) — consistent with the platform's permission-aware UI principle, applied to provider capability instead of RBAC permission.

## 5. Local Mirror + Sync Job Model

Provider data is **mirrored locally**, never queried live on every read. Live calls on every UI request are slow, do not work offline, are hard to audit, and bypass RLS/tenancy (doc 12) since the data would not live in Postgres at all.

```text
Sync job (Core Jobs, doc 31), scheduled or push-triggered
   → provider.list_peers() / list_routes() / …
   → diff vs local mirror table (e.g. network_peers)
   → upsert + emit OverlayPeerAdded / OverlayPeerRemoved / OverlayPeerUpdated
```

**Drift policy:** the remote provider is always the source of truth at sync time. If a peer/route/zone was changed directly in the external system (outside Autotim), the next sync **overwrites the local mirror to match remote**, never the reverse, and emits a `OverlayDriftDetected` (or domain-equivalent) event so operators have visibility instead of a silent overwrite.

Providers with `supports_push: true` (§4) may additionally register a push handler so changes reflect sooner than the next scheduled sync — push is an optimization on top of the same mirror model, not a replacement for it; a periodic reconciliation sync still runs as the safety net.

## 6. Read Model vs Write Model

```text
Reads  (UI, API list/get)   → local mirror table only (fast, RLS-scoped, tenant-aware)
Writes (create/update/delete) → provider.create_peer() / … called directly,
                                  then an immediate resync confirms and reconciles
```

This mirrors the pattern already used for Hosts/Agent (doc 41): the platform never blocks a read on an external system's latency or availability, and a degraded/unreachable provider degrades to "stale mirror data" rather than a broken UI.

## 7. Error Types

A shared, typed error enum keeps provider failures uniform across every domain:

```rust
pub enum ProviderError {
    NotSupported(&'static str),   // capability not implemented by this provider
    Unauthenticated,              // credential invalid/expired
    Unreachable,                  // network/connectivity failure
    RateLimited { retry_after: Option<Duration> },
    InvalidResponse(String),      // unexpected payload shape from the provider
    Conflict(String),             // remote rejected the write (e.g. duplicate)
    Internal(String),
}
```

Sync jobs and write operations map `ProviderError` to job retry/backoff decisions (doc 31): `Unreachable`/`RateLimited` retry with backoff; `Unauthenticated`/`Conflict` fail fast and notify (doc 31 Notifications) rather than retry blindly.

## 8. EventBus Integration

Providers never publish to the event bus directly — they are plain trait implementations with no access to Core ports. The owning module (which does hold the `EventBus` port via `ModuleContext`, doc 13) publishes domain events after a sync or write completes:

```text
Provider call completes
   → Module updates local mirror (DB transaction)
   → Module writes an outbox row in the same transaction (doc 14, doc 31)
   → Relay publishes OverlayPeerAdded / DnsChallengeReady / CertificateIssued / …
```

This keeps providers as pure, side-effect-isolated adapters and preserves the transactional-outbox correctness guarantee end to end, regardless of which vendor produced the change.

## 9. Dependency Rules for Infra → Infra

The canonical rule lives in doc 00 (Architectural Layers → Infrastructure → Infrastructure Dependencies) and is not duplicated here. Summary for context: Infrastructure modules communicate through events by default; a direct dependency between two Infrastructure modules is allowed only when declared in the manifest, validated at boot, documented as optional/required, with an explicit failure mode.

This matters directly for provider-backed modules: e.g. an `IngressProvider` (Traefik) that wants automatic TLS may depend on the `CertificateProvider` contract from the SSL module, following the same declared/validated/documented pattern — not a hidden runtime call into SSL internals.

## 10. Provider Contract List

All traits below live in `autotim-infra-contracts` (§11) so any module needing a shared contract (e.g. both SSL and Network/Ingress needing `CertificateProvider`) depends on one place, not on each other's crates.

### DnsProvider

```rust
#[async_trait]
pub trait DnsProvider: Send + Sync {
    fn capabilities(&self) -> DnsProviderCapabilities; // e.g. supports_dnssec, supports_dynamic_update

    async fn list_zones(&self) -> Result<Vec<Zone>, ProviderError>;
    async fn create_record(&self, zone: &str, record: &DnsRecord) -> Result<(), ProviderError>;
    async fn delete_record(&self, zone: &str, record_id: &str) -> Result<(), ProviderError>;

    /// Used for ACME DNS-01 challenges (§9 example).
    async fn create_txt_challenge(&self, zone: &str, name: &str, value: &str) -> Result<(), ProviderError>;
    async fn remove_txt_challenge(&self, zone: &str, name: &str) -> Result<(), ProviderError>;

    async fn health(&self) -> ProviderHealth;
}
```

### CertificateProvider

```rust
#[async_trait]
pub trait CertificateProvider: Send + Sync {
    fn capabilities(&self) -> CertProviderCapabilities; // e.g. supports_wildcard, supports_dns01, supports_http01

    async fn issue(&self, request: &CertRequest) -> Result<CertHandle, ProviderError>;
    async fn renew(&self, cert_id: &str) -> Result<CertHandle, ProviderError>;
    async fn revoke(&self, cert_id: &str) -> Result<(), ProviderError>;
    async fn status(&self, cert_id: &str) -> Result<CertStatus, ProviderError>;

    async fn health(&self) -> ProviderHealth;
}
```

### MonitoringProvider

```rust
#[async_trait]
pub trait MonitoringProvider: Send + Sync {
    fn capabilities(&self) -> MonitoringProviderCapabilities; // e.g. supports_alerting, supports_query

    async fn query(&self, query: &MetricQuery) -> Result<MetricResult, ProviderError>;
    async fn register_target(&self, target: &ScrapeTarget) -> Result<(), ProviderError>;
    async fn list_alerts(&self) -> Result<Vec<Alert>, ProviderError>;

    async fn health(&self) -> ProviderHealth;
}
```

> Note: `MonitoringProvider` is the query/registration contract (e.g. talking to Prometheus). It is distinct from the platform's own observability pipeline (doc 51), which always emits to the TSDB regardless of whether a monitoring module is enabled.

### DashboardProvider

```rust
#[async_trait]
pub trait DashboardProvider: Send + Sync {
    fn capabilities(&self) -> DashboardProviderCapabilities; // e.g. supports_provisioning_api

    async fn provision_dashboard(&self, dashboard: &DashboardDef) -> Result<DashboardHandle, ProviderError>;
    async fn delete_dashboard(&self, id: &str) -> Result<(), ProviderError>;
    async fn dashboard_url(&self, id: &str) -> Result<String, ProviderError>;

    async fn health(&self) -> ProviderHealth;
}
```

> Grafana is a `DashboardProvider`, not a `MonitoringProvider` — it provisions/links visualizations; it does not own metric data (Prometheus/VictoriaMetrics does, per doc 51's storage boundary).

### MailProvider

```rust
#[async_trait]
pub trait MailProvider: Send + Sync {
    fn capabilities(&self) -> MailProviderCapabilities; // e.g. supports_dkim_mgmt, supports_quotas

    async fn create_domain(&self, domain: &str) -> Result<(), ProviderError>;
    async fn create_mailbox(&self, domain: &str, mailbox: &MailboxDef) -> Result<(), ProviderError>;
    async fn delete_mailbox(&self, domain: &str, mailbox: &str) -> Result<(), ProviderError>;

    async fn health(&self) -> ProviderHealth;
}
```

### IngressProvider

```rust
#[async_trait]
pub trait IngressProvider: Send + Sync {
    fn capabilities(&self) -> IngressProviderCapabilities; // e.g. supports_auto_tls, supports_middleware

    async fn list_routes(&self) -> Result<Vec<IngressRoute>, ProviderError>;
    async fn create_route(&self, route: &IngressRouteDef) -> Result<(), ProviderError>;
    async fn delete_route(&self, id: &str) -> Result<(), ProviderError>;

    async fn health(&self) -> ProviderHealth;
}
```

> Traefik is the reference `IngressProvider`. When `supports_auto_tls` is true and a `CertificateProvider` dependency is declared (§9), the Ingress module can request certificates through the SSL module's contract instead of duplicating ACME logic.

### OverlayNetworkProvider

```rust
#[async_trait]
pub trait OverlayNetworkProvider: Send + Sync {
    fn capabilities(&self) -> ProviderCapabilities; // see §4

    async fn list_peers(&self) -> Result<Vec<Peer>, ProviderError>;
    async fn list_routes(&self) -> Result<Vec<Route>, ProviderError>;
    async fn list_groups(&self) -> Result<Vec<Group>, ProviderError> {
        Err(ProviderError::NotSupported("groups"))
    }

    async fn health(&self) -> ProviderHealth;
}
```

Reference implementations: `NetBirdProvider`, `TailscaleProvider`, `HeadscaleProvider`, `WireGuardProvider`, `OpenVpnProvider`, `ZeroTierProvider` — all internal to the `network` module (§2).

## 11. Crate Layout

```text
autotim-infra-contracts/         # shared provider traits used by 2+ modules
├── src/
│   ├── dns.rs                   # DnsProvider
│   ├── certificate.rs           # CertificateProvider
│   ├── monitoring.rs            # MonitoringProvider
│   ├── dashboard.rs              # DashboardProvider
│   ├── mail.rs                  # MailProvider
│   ├── ingress.rs                # IngressProvider
│   ├── overlay_network.rs        # OverlayNetworkProvider
│   └── error.rs                  # ProviderError, ProviderHealth

autotim-infra-network/
├── src/
│   ├── providers/
│   │   ├── netbird.rs
│   │   ├── tailscale.rs
│   │   ├── headscale.rs
│   │   ├── wireguard.rs
│   │   └── openvpn.rs
│   ├── sync_job.rs                # Core Jobs-based mirror, per §5
│   └── module.rs                  # implements Module trait (doc 13)

autotim-infra-dns/
├── src/
│   ├── providers/powerdns.rs
│   ├── sync_job.rs
│   └── module.rs
```

Each Infrastructure module crate depends on `autotim-infra-contracts` and `autotim-sdk` — never on another Infrastructure module's crate directly (that would violate the cross-module integrity rule, doc 13). Provider crates declare an `sdk_version` exactly like modules do (doc 60); boot validation (§12) checks both.

## 12. Boot Validation

At startup, in addition to the existing module dependency/SemVer checks (doc 13, doc 60), the kernel validates, per enabled Infrastructure module with a provider registry:

```text
- the configured provider (Settings) is among the compiled-in providers
- the provider's sdk_version is compatible with the running SDK
- the provider's required credentials (secret refs) resolve to existing,
  unsealed secrets (doc 23) — fails closed if Secrets is sealed
- the provider's health() check passes (or the module starts in
  Degraded state with a surfaced health warning, never silently green)
```

Provider health feeds the module's existing `Module::health()` report (doc 13) and is visible on the platform health dashboard (doc 51) — a dead NetBird API becomes an observable Warning/Degraded state immediately, not a silent gap discovered later.

## 13. UI Capability Discovery

The frontend (doc 50) fetches the active provider's `capabilities()` (exposed via a small Core-mediated API, not a direct provider call from the browser) alongside the user's RBAC permissions, and renders accordingly:

```text
permission missing        → control hidden/disabled (RBAC, doc 21)
capability not supported  → control hidden/disabled (provider, this doc)
```

Both checks are UX conveniences; the server remains the only real gate — a write attempt against an unsupported capability returns `ProviderError::NotSupported` mapped to a clear `422`-class API error (doc 40), not a silent failure. On mobile, capability-aware rendering matters even more: hiding an entire unsupported tab (e.g. "Groups") keeps the single-column, thumb-reachable layout (doc 50) free of dead-end controls.

## Decision

Core never imports a vendor SDK. A module owns a domain and a small set of capability-aware provider traits; vendors are interchangeable adapters behind those traits, selected per organization, with credentials in Secrets and a local mirror as the read path.
