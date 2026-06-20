# 31 — Async Substrate (Jobs · Outbox · Events · Notifications)

One durable async substrate, not three. Jobs are the foundation; event delivery and notifications are built on top. This removes the v1 triplication (separate queue/retry/DLQ in Events, Jobs, Notifications).

## Components

```text
Outbox            transactional event capture (doc 14)
Event Bus (port)  publish/subscribe; in-proc or broker adapter
Job Queue         durable queue + worker pool + retry/backoff + DLQ
Notifications     a job type that delivers via channel adapters
Scheduler         cron/interval triggers, singleton via leader lock
```

All of these run inside the single binary at small scale and scale out behind their ports later.

## Transactional Outbox → Reliable Events

State changes that emit events write the event in the **same transaction** as the state change (doc 14). A relay publishes outbox rows to the event bus and marks them dispatched.

```text
TX { write state + write outbox } → COMMIT
Relay: outbox → Event Bus → subscribers
```

This is what makes **at-least-once** real. Consumers must be idempotent (deduplicate by `event_id`).

## Event Bus as a Port

```rust
#[async_trait]
pub trait EventBus: Send + Sync {
    async fn publish(&self, event: &Event) -> Result<()>;
    async fn subscribe(&self, types: &[EventType], handler: Handler) -> Result<()>;
}
```

Adapters:
- **In-process** (default, single node): direct dispatch to in-memory subscribers.
- **Broker** (NATS / Redis Streams): cross-instance delivery for HA/multi-node.

The in-process adapter only reaches subscribers in the same process. The moment you run >1 instance, you switch to the broker adapter — **module code does not change**, only the adapter wired at startup. This is the explicit answer to the v1 "in-process bus can't scale horizontally" problem.

## Event Structure

```json
{
  "event_id": "uuid",
  "event_type": "HostCreated",
  "version": 1,
  "organization_id": "uuid",
  "source": "hosts",
  "correlation_id": "uuid",
  "timestamp": "2026-01-01T12:00:00Z",
  "payload": {}
}
```

`organization_id` and `correlation_id` travel with every event so handlers run in the right tenant context (doc 12) and traces stay connected (doc 51).

## Event Naming & Versioning

PascalCase: `UserCreated`, `RoleAssigned`, `HostOnline`, `AgentDisconnected`, `CertificateExpired`, `SecretRotated`, `DnsZoneCreated`. Events are **immutable**. Schema evolution uses a `version` field; consumers handle known versions.

## Event Store, Replay, DLQ (deferred machinery, present interface)

- `event_store` persists events (partitioned, doc 14) for audit/replay/analytics.
- Replay and a dead-letter queue are part of the design but their full machinery is built when needed (≈1k+ tier). The interface exists from day one so adding them is not a refactor.

## Job System

```rust
pub struct Job {
    pub id: Uuid,
    pub organization_id: OrganizationId,
    pub kind: String,                 // "ssl.renew", "notify.deliver", ...
    pub payload: serde_json::Value,
    pub run_at: DateTime<Utc>,        // immediate / delayed / scheduled
    pub idempotency_key: Option<String>,
}
```

Lifecycle: `Created → Queued → Running → Completed` or `Failed → Retry(backoff) → Completed | Dead`.

- Postgres-backed queue using `SELECT … FOR UPDATE SKIP LOCKED` (fine to the low thousands; a dedicated queue/broker is the scale-out path).
- Retry with exponential backoff + jitter; permanently failed jobs move to a dead-job queue (inspect / replay).
- Workers update status; all transitions audited.

## Scheduler Singleton (correctness)

With multiple app instances, cron must fire **once**, not N times. The scheduler acquires a distributed lock (Postgres advisory lock at small scale; a lease in a shared store at multi-node) — only the lock holder enqueues scheduled jobs. This closes the v1 "every instance runs cron" bug.

## Notifications as Jobs

Notifications are not a separate stack. A `notify.deliver` job runs through the same queue/retry/DLQ as any other job:

```text
Module/event → enqueue notify.deliver job → worker → Channel Provider → recipient
```

The notification domain itself — channels as Integration Providers, templates, user preferences, security-notification priority — is specified in doc 32 (Notifications), which builds directly on this substrate rather than duplicating it.

## Idempotency

- Event consumers dedupe by `event_id`.
- Jobs honor `idempotency_key` (a retried or duplicate job with the same key executes its effect once).
- This pairs with API idempotency keys (doc 40) end-to-end.

## Agent Jobs

Jobs may target remote execution; the job hands a **signed, capability-checked command** to an agent (doc 41). The agent executes and reports back; results update the job. Authorization is checked before dispatch, in Core.

## Observability

Metrics: queue depth, running/failed/completed counts, execution time, DLQ size, event publish/deliver/fail rates (doc 51). Correlation IDs propagate from request → event → job → agent command.

## Constraint

Modules schedule work and publish events; the Core substrate executes work and delivers events — reliably, idempotently, and tenant-aware.
