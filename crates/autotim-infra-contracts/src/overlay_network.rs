//! `OverlayNetworkProvider` — doc 42 §10, §4.
//!
//! Reference implementations live inside the `network` module's internal
//! provider registry (doc 42 §2): NetBirdProvider, TailscaleProvider,
//! HeadscaleProvider, WireGuardProvider, OpenVpnProvider, ZeroTierProvider.
//! They are adapters, not separate Module Registry entries.

use async_trait::async_trait;
use autotim_sdk::{ProviderError, ProviderHealth};

/// Capability flags. Plain WireGuard/OpenVPN typically have no queryable
/// control plane, so `groups`/`acl` are false for them — methods outside
/// a provider's capability set return `ProviderError::NotSupported`
/// rather than silently no-op-ing (doc 42 §4).
#[derive(Debug, Clone, Default)]
pub struct ProviderCapabilities {
    pub groups: bool,
    pub acl: bool,
    pub exit_nodes: bool,
    pub dns_override: bool,
    /// True if the provider can push change notifications (webhook/event)
    /// instead of relying solely on polling sync jobs (doc 42 §5, §8).
    pub supports_push: bool,
}

#[derive(Debug, Clone)]
pub struct Peer {
    pub id: String,
    pub name: String,
    pub address: String,
    pub online: bool,
}

#[derive(Debug, Clone)]
pub struct Route {
    pub id: String,
    pub network: String,
}

#[derive(Debug, Clone)]
pub struct Group {
    pub id: String,
    pub name: String,
}

#[async_trait]
pub trait OverlayNetworkProvider: Send + Sync {
    fn capabilities(&self) -> ProviderCapabilities;

    async fn list_peers(&self) -> Result<Vec<Peer>, ProviderError>;
    async fn list_routes(&self) -> Result<Vec<Route>, ProviderError>;

    /// Default: not supported. Providers with a group concept override this.
    async fn list_groups(&self) -> Result<Vec<Group>, ProviderError> {
        Err(ProviderError::NotSupported("groups"))
    }

    /// Optional: providers with `supports_push: true` implement this to
    /// register their own webhook/event handler instead of relying purely
    /// on polling. Default is a no-op for poll-only providers.
    async fn register_push_handler(&self) -> Result<(), ProviderError> {
        Ok(())
    }

    async fn health(&self) -> ProviderHealth;
}
