//! `CertificateProvider` — doc 42 §10.

use async_trait::async_trait;
use autotim_sdk::{ProviderError, ProviderHealth};

#[derive(Debug, Clone, Default)]
pub struct CertProviderCapabilities {
    pub supports_wildcard: bool,
    pub supports_dns01: bool,
    pub supports_http01: bool,
}

#[derive(Debug, Clone)]
pub struct CertRequest {
    pub domains: Vec<String>,
    pub challenge: ChallengeType,
}

#[derive(Debug, Clone, Copy)]
pub enum ChallengeType {
    Dns01,
    Http01,
}

#[derive(Debug, Clone)]
pub struct CertHandle {
    pub id: String,
}

#[derive(Debug, Clone)]
pub enum CertStatus {
    Pending,
    Issued { expires_at: chrono::DateTime<chrono::Utc> },
    Failed { reason: String },
    Revoked,
}

#[async_trait]
pub trait CertificateProvider: Send + Sync {
    fn capabilities(&self) -> CertProviderCapabilities;

    async fn issue(&self, request: &CertRequest) -> Result<CertHandle, ProviderError>;
    async fn renew(&self, cert_id: &str) -> Result<CertHandle, ProviderError>;
    async fn revoke(&self, cert_id: &str) -> Result<(), ProviderError>;
    async fn status(&self, cert_id: &str) -> Result<CertStatus, ProviderError>;

    async fn health(&self) -> ProviderHealth;
}
