//! Error and health types shared across modules and providers.
//!
//! `ProviderError` / `ProviderHealth` are the contract used by every
//! Integration Provider (doc 42); `SdkError` is the general module-facing
//! error type for Core port calls.

use std::time::Duration;
use thiserror::Error;

pub type SdkResult<T> = Result<T, SdkError>;

#[derive(Debug, Error)]
pub enum SdkError {
    #[error("permission denied")]
    PermissionDenied,
    #[error("not found")]
    NotFound,
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("validation error: {0}")]
    Validation(String),
    #[error("dependency unavailable: {0}")]
    DependencyUnavailable(String),
    #[error("internal error: {0}")]
    Internal(String),
}

/// Error contract for Integration Providers (doc 42, §7). Sync jobs and
/// write operations map this to job retry/backoff decisions: Unreachable
/// and RateLimited retry with backoff; Unauthenticated and Conflict fail
/// fast and notify rather than retry blindly.
#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("capability not supported by this provider: {0}")]
    NotSupported(&'static str),
    #[error("provider credentials invalid or expired")]
    Unauthenticated,
    #[error("provider unreachable")]
    Unreachable,
    #[error("provider rate limited")]
    RateLimited { retry_after: Option<Duration> },
    #[error("unexpected response from provider: {0}")]
    InvalidResponse(String),
    #[error("provider rejected the operation: {0}")]
    Conflict(String),
    #[error("provider internal error: {0}")]
    Internal(String),
}

/// Health status reported by a provider, surfaced through
/// `Module::health()` and the platform health dashboard (doc 51, doc 42 §12).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderHealth {
    Healthy,
    Degraded { reason: String },
    Unreachable { reason: String },
}
