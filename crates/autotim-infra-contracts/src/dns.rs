//! `DnsProvider` — doc 42 §10. Reference implementation: PowerDNS.

use async_trait::async_trait;
use autotim_sdk::{ProviderError, ProviderHealth};

#[derive(Debug, Clone, Default)]
pub struct DnsProviderCapabilities {
    pub supports_dnssec: bool,
    pub supports_dynamic_update: bool,
}

#[derive(Debug, Clone)]
pub struct Zone {
    pub name: String,
    pub kind: String,
}

#[derive(Debug, Clone)]
pub struct DnsRecord {
    pub name: String,
    pub record_type: String,
    pub value: String,
    pub ttl: u32,
}

#[async_trait]
pub trait DnsProvider: Send + Sync {
    fn capabilities(&self) -> DnsProviderCapabilities;

    async fn list_zones(&self) -> Result<Vec<Zone>, ProviderError>;
    async fn create_record(&self, zone: &str, record: &DnsRecord) -> Result<(), ProviderError>;
    async fn delete_record(&self, zone: &str, record_id: &str) -> Result<(), ProviderError>;

    /// Used for ACME DNS-01 challenges. See doc 00 (Infrastructure →
    /// Infrastructure dependency rule) and doc 42 §9 for how SSL and DNS
    /// coordinate this without a hidden coupling.
    async fn create_txt_challenge(&self, zone: &str, name: &str, value: &str) -> Result<(), ProviderError>;
    async fn remove_txt_challenge(&self, zone: &str, name: &str) -> Result<(), ProviderError>;

    async fn health(&self) -> ProviderHealth;
}
