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
pub use module::{HealthReport, HealthStatus, Layer, Module, ModuleContext, ModuleManifest};
pub use types::{OrganizationId, Permission, Scope, UserId};
