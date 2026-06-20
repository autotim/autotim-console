//! `IngressProvider` — doc 42 §10. Reference implementation: Traefik.
//!
//! When `supports_auto_tls` is true and a `CertificateProvider`
//! dependency is declared (doc 00 — Infrastructure → Infrastructure
//! Dependencies; doc 42 §9), the Ingress module can request
//! certificates through the SSL module's contract instead of
//! duplicating ACME logic.

use async_trait::async_trait;
use autotim_sdk::{ProviderError, ProviderHealth};

#[derive(Debug, Clone, Default)]
pub struct IngressProviderCapabilities {
    pub supports_auto_tls: bool,
    pub supports_middleware: bool,
}

#[derive(Debug, Clone)]
pub struct IngressRoute {
    pub id: String,
    pub host: String,
    pub target: String,
}

#[derive(Debug, Clone)]
pub struct IngressRouteDef {
    pub host: String,
    pub target: String,
    pub tls: bool,
}

#[async_trait]
pub trait IngressProvider: Send + Sync {
    fn capabilities(&self) -> IngressProviderCapabilities;

    async fn list_routes(&self) -> Result<Vec<IngressRoute>, ProviderError>;
    async fn create_route(&self, route: &IngressRouteDef) -> Result<(), ProviderError>;
    async fn delete_route(&self, id: &str) -> Result<(), ProviderError>;

    async fn health(&self) -> ProviderHealth;
}
