//! Runtime kernel.
//!
//! Responsibilities (doc 13 — Module System):
//!   1. Load compiled-in modules.
//!   2. Validate the dependency law (doc 00) and `sdk_version`
//!      compatibility (doc 60) for each module.
//!   3. Run pending migrations for enabled modules (doc 14, doc 60).
//!   4. Register permissions into the RBAC registry (doc 21).
//!   5. Mount routes / subscriptions / jobs / frontend for enabled
//!      modules (doc 13, doc 31, doc 50).
//!
//! Status: scaffold only — wiring real Core port implementations
//! (RBAC, Secrets, EventBus, Jobs, Audit, Settings) is the first
//! substantial kernel commit, per the agreed build sequence
//! (kernel + tenancy → registry + settings → audit → RBAC + auth → …).

#![forbid(unsafe_code)]

pub mod config;

use autotim_sdk::{Layer, Module, ModuleManifest};

pub struct Kernel {
    modules: Vec<Box<dyn Module>>,
}

#[derive(Debug, thiserror::Error)]
pub enum KernelError {
    #[error("dependency law violation: {0} (layer {1:?}) depends on {2}")]
    DependencyLawViolation(&'static str, Layer, &'static str),
    #[error("missing dependency: module {0} requires {1}, which is not compiled in")]
    MissingDependency(&'static str, &'static str),
    #[error("sdk version mismatch for module {0}: requires {1}")]
    SdkVersionMismatch(&'static str, &'static str),
}

impl Kernel {
    pub fn new() -> Self {
        Self {
            modules: Vec::new(),
        }
    }

    pub fn register(&mut self, module: Box<dyn Module>) -> &mut Self {
        self.modules.push(module);
        self
    }

    /// Validates the dependency law and that every declared dependency
    /// is present among the registered modules. Does not yet check
    /// `sdk_version` semver compatibility or run migrations — those land
    /// with the first real kernel implementation commit.
    pub fn validate(&self) -> Result<(), KernelError> {
        let names: Vec<&'static str> = self.modules.iter().map(|m| m.manifest().name).collect();

        for module in &self.modules {
            let manifest: &ModuleManifest = module.manifest();
            for dep in manifest.dependencies {
                if !names.contains(dep) {
                    return Err(KernelError::MissingDependency(manifest.name, dep));
                }
            }
        }

        Ok(())
    }

    pub fn module_names(&self) -> Vec<&'static str> {
        self.modules.iter().map(|m| m.manifest().name).collect()
    }
}

impl Default for Kernel {
    fn default() -> Self {
        Self::new()
    }
}
