//! Core service ports.
//!
//! Modules receive these as trait objects via `ModuleContext`, never as
//! concrete Core crate types. This is what lets the in-process and
//! broker-backed implementations (e.g. EventBus, doc 31) be swapped
//! without touching module code, and what makes "no module implements
//! its own auth/authz/secrets/scheduler/settings" enforceable rather
//! than aspirational. See doc 13 — Module System, §"The Module Trait".

use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

use crate::error::SdkResult;
use crate::types::{OrganizationId, Permission, Scope, UserId};

/// Central authorization check. See doc 21 — RBAC (scoped grants).
#[async_trait]
pub trait Authorizer: Send + Sync {
    async fn is_authorized(&self, user: UserId, perm: Permission, scope: Scope) -> SdkResult<bool>;

    async fn require(&self, user: UserId, perm: Permission, scope: Scope) -> SdkResult<()> {
        if self.is_authorized(user, perm, scope).await? {
            Ok(())
        } else {
            Err(crate::error::SdkError::PermissionDenied)
        }
    }
}

/// Envelope-encrypted secret access. See doc 23 — Secrets.
#[async_trait]
pub trait SecretStore: Send + Sync {
    async fn get(&self, org: OrganizationId, secret_id: Uuid) -> SdkResult<String>;
    async fn put(&self, org: OrganizationId, name: &str, value: &str) -> SdkResult<Uuid>;
    async fn rotate(&self, org: OrganizationId, secret_id: Uuid, new_value: &str) -> SdkResult<()>;
}

/// Domain event. See doc 31 — Async Substrate.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Event {
    pub event_id: Uuid,
    pub event_type: String,
    pub version: u32,
    pub organization_id: OrganizationId,
    pub source: String,
    pub correlation_id: Uuid,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub payload: Value,
}

pub type EventHandler = std::sync::Arc<
    dyn Fn(Event) -> futures_signature::BoxFuture<'static, SdkResult<()>> + Send + Sync,
>;

/// Behind this port: an in-process adapter (single node) or a broker
/// adapter (NATS/Redis Streams, multi-node). Module code is identical
/// either way. Publication should be paired with the transactional
/// outbox at the call site (doc 14, doc 31) — this port itself only
/// covers bus delivery, not outbox persistence.
#[async_trait]
pub trait EventBus: Send + Sync {
    async fn publish(&self, event: Event) -> SdkResult<()>;
    async fn subscribe(&self, event_types: &[&str], handler: EventHandler) -> SdkResult<()>;
}

/// A unit of asynchronous work. See doc 31 — Async Substrate.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct JobSpec {
    pub kind: String,
    pub organization_id: OrganizationId,
    pub payload: Value,
    pub run_at: Option<chrono::DateTime<chrono::Utc>>,
    pub idempotency_key: Option<String>,
}

#[async_trait]
pub trait JobQueue: Send + Sync {
    async fn enqueue(&self, job: JobSpec) -> SdkResult<Uuid>;
}

/// Notification dispatch (built on the Job substrate, doc 31 §"Notifications as Jobs").
#[async_trait]
pub trait Notifier: Send + Sync {
    async fn notify(
        &self,
        org: OrganizationId,
        recipient: UserId,
        template: &str,
        vars: Value,
    ) -> SdkResult<()>;
}

/// A single audit record. See doc 24 — Audit, §"Event Structure".
///
/// Field shape mirrors the `audit_events` table directly; the sink
/// implementation owns hash-chaining (`prev_hash`/`hash`) and
/// partition placement, neither of which the caller should construct.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuditEntry {
    pub organization_id: OrganizationId,
    pub actor: UserId,
    pub action: String,
    pub target_type: String,
    pub target_id: Option<Uuid>,
    pub result: String,
    pub correlation_id: Uuid,
    pub metadata: Value,
}

/// Tamper-evident audit sink. See doc 24 — Audit.
#[async_trait]
pub trait AuditSink: Send + Sync {
    async fn record(&self, entry: AuditEntry) -> SdkResult<()>;
}
/// Schema-driven settings access, scoped global/module/user (doc 13, doc 23
/// "Settings store references, never values").
#[async_trait]
pub trait SettingsStore: Send + Sync {
    async fn get(&self, org: OrganizationId, key: &str) -> SdkResult<Option<Value>>;
    async fn set(&self, org: OrganizationId, key: &str, value: Value) -> SdkResult<()>;
}

/// Resolved tenant context for the current request/job/event. See doc 12.
pub trait TenantContext: Send + Sync {
    fn organization_id(&self) -> OrganizationId;
    fn user_id(&self) -> Option<UserId>;
}

/// Minimal local shim so this scaffold crate has no extra workspace
/// dependency for boxed futures. Replace with `futures::future::BoxFuture`
/// once the `futures` crate is added in the first real kernel commit.
pub mod futures_signature {
    use std::future::Future;
    use std::pin::Pin;
    pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
}
