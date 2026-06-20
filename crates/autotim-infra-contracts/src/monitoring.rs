//! `MonitoringProvider` — doc 42 §10. Reference implementation: Prometheus.
//!
//! Distinct from the platform's own observability pipeline (doc 51),
//! which always emits to the TSDB regardless of whether this module is
//! enabled. This trait is the query/registration contract for an
//! external monitoring system the platform integrates with.

use async_trait::async_trait;
use autotim_sdk::{ProviderError, ProviderHealth};

#[derive(Debug, Clone, Default)]
pub struct MonitoringProviderCapabilities {
    pub supports_alerting: bool,
    pub supports_query: bool,
}

#[derive(Debug, Clone)]
pub struct MetricQuery {
    pub expr: String,
}

#[derive(Debug, Clone)]
pub struct MetricResult {
    pub series: Vec<(String, f64)>,
}

#[derive(Debug, Clone)]
pub struct ScrapeTarget {
    pub address: String,
    pub labels: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct Alert {
    pub name: String,
    pub severity: String,
    pub firing: bool,
}

#[async_trait]
pub trait MonitoringProvider: Send + Sync {
    fn capabilities(&self) -> MonitoringProviderCapabilities;

    async fn query(&self, query: &MetricQuery) -> Result<MetricResult, ProviderError>;
    async fn register_target(&self, target: &ScrapeTarget) -> Result<(), ProviderError>;
    async fn list_alerts(&self) -> Result<Vec<Alert>, ProviderError>;

    async fn health(&self) -> ProviderHealth;
}
