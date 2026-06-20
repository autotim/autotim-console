//! `NotificationChannelProvider` — doc 32 §"NotificationChannelProvider Contract".
//!
//! Same shape as the Integration Provider pattern in doc 42: capability-aware,
//! shared `ProviderError`/`ProviderHealth` types from `autotim-sdk`.

use async_trait::async_trait;
use autotim_sdk::{ProviderError, ProviderHealth};

#[derive(Debug, Clone, Default)]
pub struct ChannelCapabilities {
    pub supports_rich_formatting: bool,
    pub supports_delivery_receipts: bool,
    pub max_message_length: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct ChannelRecipient {
    /// Opaque per-channel address (email, chat ID, webhook URL, device token).
    /// Resolution from a user_id to this is the Notifications module's job,
    /// not the provider's.
    pub address: String,
}

#[derive(Debug, Clone)]
pub struct RenderedNotification {
    pub subject: Option<String>,
    pub body: String,
    pub category: String,
    pub severity: NotificationSeverity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone)]
pub struct DeliveryReceipt {
    pub provider_message_id: Option<String>,
}

#[async_trait]
pub trait NotificationChannelProvider: Send + Sync {
    fn capabilities(&self) -> ChannelCapabilities;

    async fn send(
        &self,
        recipient: &ChannelRecipient,
        rendered: &RenderedNotification,
    ) -> Result<DeliveryReceipt, ProviderError>;

    async fn health(&self) -> ProviderHealth;
}
