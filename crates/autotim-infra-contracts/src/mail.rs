//! `MailProvider` — doc 42 §10. Reference implementation: Stalwart.

use async_trait::async_trait;
use autotim_sdk::{ProviderError, ProviderHealth};

#[derive(Debug, Clone, Default)]
pub struct MailProviderCapabilities {
    pub supports_dkim_mgmt: bool,
    pub supports_quotas: bool,
}

#[derive(Debug, Clone)]
pub struct MailboxDef {
    pub local_part: String,
    pub quota_mb: Option<u64>,
}

#[async_trait]
pub trait MailProvider: Send + Sync {
    fn capabilities(&self) -> MailProviderCapabilities;

    async fn create_domain(&self, domain: &str) -> Result<(), ProviderError>;
    async fn create_mailbox(&self, domain: &str, mailbox: &MailboxDef) -> Result<(), ProviderError>;
    async fn delete_mailbox(&self, domain: &str, mailbox: &str) -> Result<(), ProviderError>;

    async fn health(&self) -> ProviderHealth;
}
