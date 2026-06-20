//! `DashboardProvider` — doc 42 §10. Reference implementation: Grafana.
//!
//! Grafana provisions/links visualizations; it does not own metric data
//! (Prometheus/VictoriaMetrics does, per doc 51's storage boundary).
//! That is why this is a separate trait from `MonitoringProvider`.

use async_trait::async_trait;
use autotim_sdk::{ProviderError, ProviderHealth};

#[derive(Debug, Clone, Default)]
pub struct DashboardProviderCapabilities {
    pub supports_provisioning_api: bool,
}

#[derive(Debug, Clone)]
pub struct DashboardDef {
    pub title: String,
    pub json_model: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct DashboardHandle {
    pub id: String,
}

#[async_trait]
pub trait DashboardProvider: Send + Sync {
    fn capabilities(&self) -> DashboardProviderCapabilities;

    async fn provision_dashboard(
        &self,
        dashboard: &DashboardDef,
    ) -> Result<DashboardHandle, ProviderError>;
    async fn delete_dashboard(&self, id: &str) -> Result<(), ProviderError>;
    async fn dashboard_url(&self, id: &str) -> Result<String, ProviderError>;

    async fn health(&self) -> ProviderHealth;
}
