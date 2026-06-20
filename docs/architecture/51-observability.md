# 51 — Observability Architecture

## Pillars

```text
Metrics   Prometheus / VictoriaMetrics  (+ Grafana dashboards)
Logs      structured, leveled, correlation-aware
Traces    OpenTelemetry (OTel)
```

> **Storage boundary (resolved from v1):** metrics live in a time-series store, never in Postgres. Postgres holds control-plane metadata only (doc 14).

## Metrics

The platform exposes a Prometheus `/metrics` endpoint (the app's own metrics) and ingests **agent/host metrics into the TSDB**, not the relational DB. Examples:

```text
Platform:  request rate/latency/errors per route, DB pool usage,
           job queue depth, jobs running/failed/completed, DLQ size,
           event publish/deliver/fail rates, active agent connections,
           cache hit ratio (RBAC decisions)
Infra:     CPU/mem/disk per host, service up/down, cert days-to-expiry
```

Grafana dashboards (shipped as code): Core Health, Agent Health, API Performance, Job System, Per-Module Health.

## Logs

Structured JSON logs (via `tracing`), one event per line, suitable for journald/stdout and log shippers.

```json
{ "level": "info", "module": "dns", "message": "zone created",
  "request_id": "uuid", "organization_id": "uuid",
  "user_id": "uuid", "timestamp": "..." }
```

Levels: `TRACE DEBUG INFO WARN ERROR`. Every entry carries `module`, `timestamp`, `level`, `message`; sensitive values are never logged (no secrets, no full tokens).

## Tracing & Correlation

A `correlation_id` (a.k.a. `request_id`) is created at the edge and **propagated across every async boundary** — events, jobs, agent commands (docs 31, 41). This is essential: in an event-driven system, a problem is only debuggable if you can follow one logical operation through API → RBAC → DB → event → job → agent.

```text
Request → API → RBAC → Module → DB
                         └→ event → job → agent command
        (one correlation_id throughout)
```

OTel exports traces to any compatible backend (Tempo, Jaeger, etc.).

## Health Checks

Each module exposes status, version, dependency health, and database health via `Module::health()`. Aggregated into a platform health endpoint and dashboard.

```text
Healthy · Warning · Degraded · Critical · Offline
```

## Alerting

Defined in Prometheus/Grafana (as code): Agent Offline, Database Failure, High Error Rate, Certificate Expiring, Job DLQ Growing, Unseal/Sealed state, Auth failure spikes. Severities: `Info · Warning · Critical`. Alerts can route through the Notifications module (doc 31) for operator-facing channels.

## Audit ↔ Observability

Security/audit events (doc 24) are observable: failed-login spikes, permission changes, secret access — surfaced as metrics/alerts for anomaly detection, without putting raw audit content into metrics.

## Multi-Tenant Awareness

Metrics, logs, and traces carry `organization_id` where relevant, so per-tenant dashboards and troubleshooting are possible (important for resellers/EE).

## Scaling Notes

```text
10–100     single Prometheus + Grafana; app /metrics scraped
1,000      retention tuning; recording rules; per-module dashboards
10,000     VictoriaMetrics/Mimir for long-term + scale; sampled tracing
```

## Future

OpenTelemetry everywhere, SLOs + error budgets, service maps, trace-based alerting.

## Constraint

If a problem cannot be observed and traced end-to-end (including through async hops), it cannot be operated reliably.
