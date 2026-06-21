//! Autotim SDK
//!
//! The single stable contract every module compiles against. Modules
//! depend on this crate and on the *ports* defined here — never on
//! concrete Core crate internals. This is what keeps the Core →
//! Infrastructure → Business dependency law enforceable and lets Core
//! implementations evolve without breaking modules.
//!
//! See architecture doc 10 (Repository & Build) and doc 13 (Module System).

#![forbid(unsafe_code)]

pub mod error;
pub mod module;
pub mod ports;
pub mod types;

pub use error::{ProviderError, ProviderHealth, SdkError, SdkResult};
pub use module::{
    FrontendManifest, HealthReport, HealthStatus, Layer, Migration, Module, ModuleContext,
    ModuleManifest,
};
pub use types::{OrganizationId, Permission, Scope, UserId};

/// The version of this SDK, as the single source of truth for the
/// module compatibility contract (doc 60). A module declares the
/// `sdk_version` it was built against in its `ModuleManifest`; the
/// kernel compares that declaration against this value at boot and
/// refuses to start an incompatible module.
///
/// This is the SDK crate's own package version (workspace-shared,
/// doc 10/60), exposed so the kernel reads the SDK's version from the
/// SDK rather than assuming its own version matches.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
