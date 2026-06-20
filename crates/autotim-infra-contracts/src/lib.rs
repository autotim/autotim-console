//! Integration Provider Contracts (doc 42).
//!
//! `Core never imports a vendor SDK.` A module owns a domain and a small
//! set of capability-aware provider traits; vendors are interchangeable
//! adapters behind those traits, selected per organization (doc 12),
//! with credentials in Secrets (doc 23) and a local mirror as the read
//! path (doc 42 §5–§6).

#![forbid(unsafe_code)]

pub mod certificate;
pub mod dashboard;
pub mod dns;
pub mod ingress;
pub mod mail;
pub mod monitoring;
pub mod overlay_network;

// Error/health types are shared platform-wide; re-exported here so
// provider implementors only need to depend on this crate.
pub use autotim_sdk::{ProviderError, ProviderHealth};
