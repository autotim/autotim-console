//! The `Module` trait — the stable contract every compile-time crate
//! implements. See doc 13 — Module System.
//!
//! Modules are compile-time Rust crates; runtime control is enable/disable
//! (persisted in `module_state`), not runtime code installation. This is
//! the resolved v2 decision replacing the v1 contradiction between a
//! runtime plugin lifecycle and compile-time crates.

use async_trait::async_trait;
use axum::Router;
use std::sync::Arc;

use crate::ports::{
    AuditSink, Authorizer, EventBus, JobQueue, Notifier, SecretStore, SettingsStore,
};

/// Architectural layer a module belongs to. Validated against the
/// dependency law at boot (doc 00): Core → Core, Infra → Core,
/// Business → Core/Infra. Core never depends on Infrastructure or
/// Business. See doc 00 also for the Infrastructure → Infrastructure
/// rule (event-driven by default; direct dependency only if declared,
/// validated, and documented).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    Core,
    Infrastructure,
    Business,
}

/// Declarative metadata a module registers with the Module Registry
/// (doc 13). `dependencies` are validated against the dependency law
/// and existence/enablement at boot.
pub struct ModuleManifest {
    pub name: &'static str,
    pub version: &'static str,
    pub layer: Layer,
    pub description: &'static str,
    pub dependencies: &'static [&'static str],
    /// Compatibility contract against this SDK's version (doc 60).
    pub sdk_version: &'static str,
    /// Whether this module owns tenant-scoped data (doc 12). The kernel
    /// verifies tenant-scoped modules' tables carry `organization_id`.
    pub tenant_scoped: bool,
}

/// Ports handed to a module at enable-time. A module receives only
/// these — never another module's internals (doc 13).
#[derive(Clone)]
pub struct ModuleContext {
    pub authorizer: Arc<dyn Authorizer>,
    pub secrets: Arc<dyn SecretStore>,
    pub events: Arc<dyn EventBus>,
    pub jobs: Arc<dyn JobQueue>,
    pub notifier: Arc<dyn Notifier>,
    pub audit: Arc<dyn AuditSink>,
    pub settings: Arc<dyn SettingsStore>,
    pub db: sqlx::PgPool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    Warning,
    Degraded,
    Critical,
    Offline,
}

#[derive(Debug, Clone)]
pub struct HealthReport {
    pub status: HealthStatus,
    pub version: &'static str,
    pub detail: Option<String>,
}

/// A database migration owned by a module. Forward + rollback are both
/// required (doc 14, doc 60). The actual SQL/runner type is defined in
/// `autotim-kernel`; this is a placeholder signature for the SDK contract.
pub struct Migration {
    pub version: &'static str,
    pub description: &'static str,
}

/// Frontend manifest mirror of the backend module (doc 50). Concrete
/// shape lives in the frontend TypeScript SDK; this is the backend-side
/// placeholder so `Module::frontend()` has a return type today.
pub struct FrontendManifest {
    pub module_name: &'static str,
}

#[async_trait]
pub trait Module: Send + Sync {
    fn manifest(&self) -> &ModuleManifest;

    fn migrations(&self) -> &[Migration] {
        &[]
    }

    fn permissions(&self) -> &[(&'static str, &'static str)] {
        &[]
    }

    fn routes(&self, _ctx: &ModuleContext) -> Router {
        Router::new()
    }

    fn subscriptions(&self) -> &[&'static str] {
        &[]
    }

    fn frontend(&self) -> Option<FrontendManifest> {
        None
    }

    async fn on_enable(&self, _ctx: &ModuleContext) -> crate::error::SdkResult<()> {
        Ok(())
    }

    async fn on_disable(&self, _ctx: &ModuleContext) -> crate::error::SdkResult<()> {
        Ok(())
    }

    async fn health(&self, _ctx: &ModuleContext) -> HealthReport {
        HealthReport {
            status: HealthStatus::Healthy,
            version: self.manifest().version,
            detail: None,
        }
    }
}
