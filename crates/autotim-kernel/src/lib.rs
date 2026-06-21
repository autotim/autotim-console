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
//! Status: registration + validation (steps 1–2) are implemented here.
//! Migration running (step 3) lands with the PgPool + migration runner
//! commit; port wiring (steps 4–5) lands as each Core port is built.

#![forbid(unsafe_code)]

pub mod config;
pub mod migrations;

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
    #[error("sdk version mismatch for module {0}: requires {1}, kernel SDK is {2}")]
    SdkVersionMismatch(&'static str, &'static str, &'static str),
}

/// The dependency law (doc 00 — Architectural Layers). Returns true if
/// a module in layer `from` may depend on a module in layer `to`.
///
/// ```text
/// Allowed:   Core->Core, Infra->Core, Infra->Infra,
///            Business->Core, Business->Infra, Business->Business
/// Forbidden: Core->Infra, Core->Business, Infra->Business
/// ```
///
/// Note on Infra->Infra: it is layer-legal here. The finer doc 00 rule
/// (Infrastructure modules talk via events by default; a direct Infra->
/// Infra dependency is allowed only when declared in the manifest and
/// documented with an explicit failure mode) is satisfied at this layer
/// simply by the dependency being *declared* — which the existence
/// check below already enforces. The behavioural part of that rule
/// (events-by-default) is a design guideline, not something the layer
/// validator can or should assert.
///
/// Same-layer dependencies are legal in every layer.
fn dependency_allowed(from: Layer, to: Layer) -> bool {
    use Layer::*;
    match (from, to) {
        (Core, Core) => true,
        (Core, _) => false,
        (Infrastructure, Core) => true,
        (Infrastructure, Infrastructure) => true,
        (Infrastructure, Business) => false,
        (Business, _) => true,
    }
}

/// True if a module built against `module_sdk` is compatible with the
/// kernel's SDK version `kernel_sdk` (doc 60).
///
/// Compatibility is by SemVer major: a `1.x` module runs on SDK `1.4`
/// but is refused on `2.0`. While the SDK is pre-1.0 (`0.x`), all `0.x`
/// versions are treated as mutually compatible: SemVer gives `0.x` no
/// stability guarantees, but SDK and modules currently evolve together
/// in one workspace, so gating every `0.minor` bump would reject every
/// module on every SDK change with no benefit. Once the SDK reaches
/// `1.0`, the major check becomes the real contract.
fn sdk_compatible(module_sdk: &str, kernel_sdk: &str) -> bool {
    fn major(v: &str) -> &str {
        v.split('.').next().unwrap_or("")
    }
    major(module_sdk) == major(kernel_sdk)
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

    /// Validates, for every registered module:
    ///   - `sdk_version` is compatible with the kernel's SDK (doc 60),
    ///   - every declared dependency is compiled in (present among the
    ///     registered modules),
    ///   - every declared dependency is permitted by the layer
    ///     dependency law (doc 00).
    ///
    /// Resolution is order-independent: all modules are indexed by name
    /// first, so a module may be registered before or after the
    /// dependencies it declares. Fails fast on the first violation —
    /// the kernel never boots into a half-validated module graph
    /// (doc 11 §"Error Handling").
    pub fn validate(&self) -> Result<(), KernelError> {
        use std::collections::HashMap;

        let by_name: HashMap<&'static str, &ModuleManifest> = self
            .modules
            .iter()
            .map(|m| {
                let manifest = m.manifest();
                (manifest.name, manifest)
            })
            .collect();

        for module in &self.modules {
            let manifest = module.manifest();

            // sdk_version compatibility (doc 60).
            if !sdk_compatible(manifest.sdk_version, autotim_sdk::VERSION) {
                return Err(KernelError::SdkVersionMismatch(
                    manifest.name,
                    manifest.sdk_version,
                    autotim_sdk::VERSION,
                ));
            }

            for dep in manifest.dependencies {
                match by_name.get(dep) {
                    None => {
                        return Err(KernelError::MissingDependency(manifest.name, dep));
                    }
                    Some(target) => {
                        if !dependency_allowed(manifest.layer, target.layer) {
                            return Err(KernelError::DependencyLawViolation(
                                manifest.name,
                                manifest.layer,
                                dep,
                            ));
                        }
                    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use autotim_sdk::{HealthReport, HealthStatus, Module, ModuleContext, ModuleManifest};

    struct TestModule(ModuleManifest);

    #[async_trait]
    impl Module for TestModule {
        fn manifest(&self) -> &ModuleManifest {
            &self.0
        }
        async fn health(&self, _ctx: &ModuleContext) -> HealthReport {
            HealthReport {
                status: HealthStatus::Healthy,
                version: self.0.version,
                detail: None,
            }
        }
    }

    fn m(
        name: &'static str,
        layer: Layer,
        deps: &'static [&'static str],
        sdk: &'static str,
    ) -> Box<dyn Module> {
        Box::new(TestModule(ModuleManifest {
            name,
            version: "0.1.0",
            layer,
            description: "test",
            dependencies: deps,
            sdk_version: sdk,
            tenant_scoped: false,
        }))
    }

    fn kernel(mods: Vec<Box<dyn Module>>) -> Kernel {
        let mut k = Kernel::new();
        for module in mods {
            k.register(module);
        }
        k
    }

    // --- dependency_allowed: exhaustive layer-law table (doc 00) ---

    #[test]
    fn layer_law_table_is_exhaustive_and_correct() {
        use Layer::*;
        assert!(dependency_allowed(Core, Core));
        assert!(!dependency_allowed(Core, Infrastructure));
        assert!(!dependency_allowed(Core, Business));
        assert!(dependency_allowed(Infrastructure, Core));
        assert!(dependency_allowed(Infrastructure, Infrastructure));
        assert!(!dependency_allowed(Infrastructure, Business));
        assert!(dependency_allowed(Business, Core));
        assert!(dependency_allowed(Business, Infrastructure));
        assert!(dependency_allowed(Business, Business));
    }

    // --- sdk_compatible ---

    #[test]
    fn sdk_compatible_matches_on_major() {
        assert!(sdk_compatible("0.1", "0.1.0"));
        assert!(sdk_compatible("0.9", "0.1.0")); // all 0.x mutually compatible
        assert!(sdk_compatible("1.4", "1.0.0"));
        assert!(!sdk_compatible("2.0", "1.0.0"));
        assert!(!sdk_compatible("1.0", "0.1.0"));
    }

    // --- validate() integration ---

    #[test]
    fn empty_kernel_validates() {
        assert!(kernel(vec![]).validate().is_ok());
    }

    #[test]
    fn single_core_module_validates() {
        let k = kernel(vec![m("tenancy", Layer::Core, &[], "0.1")]);
        assert!(k.validate().is_ok());
    }

    #[test]
    fn infra_depending_on_core_is_allowed() {
        let k = kernel(vec![
            m("tenancy", Layer::Core, &[], "0.1"),
            m("dns", Layer::Infrastructure, &["tenancy"], "0.1"),
        ]);
        assert!(k.validate().is_ok());
    }

    #[test]
    fn core_depending_on_infra_is_rejected() {
        let k = kernel(vec![
            m("badcore", Layer::Core, &["dns"], "0.1"),
            m("dns", Layer::Infrastructure, &[], "0.1"),
        ]);
        assert!(matches!(
            k.validate(),
            Err(KernelError::DependencyLawViolation(
                "badcore",
                Layer::Core,
                "dns"
            ))
        ));
    }

    #[test]
    fn infra_depending_on_business_is_rejected() {
        let k = kernel(vec![
            m("dns", Layer::Infrastructure, &["billing"], "0.1"),
            m("billing", Layer::Business, &[], "0.1"),
        ]);
        assert!(matches!(
            k.validate(),
            Err(KernelError::DependencyLawViolation(
                "dns",
                Layer::Infrastructure,
                "billing"
            ))
        ));
    }

    #[test]
    fn missing_dependency_is_rejected() {
        let k = kernel(vec![m("dns", Layer::Infrastructure, &["ghost"], "0.1")]);
        assert!(matches!(
            k.validate(),
            Err(KernelError::MissingDependency("dns", "ghost"))
        ));
    }

    #[test]
    fn incompatible_sdk_version_is_rejected() {
        // kernel SDK is 0.x; a module built against 1.x is refused.
        let k = kernel(vec![m("old", Layer::Core, &[], "1.0")]);
        assert!(matches!(
            k.validate(),
            Err(KernelError::SdkVersionMismatch("old", "1.0", _))
        ));
    }

    #[test]
    fn dependency_resolution_is_order_independent() {
        // dependency declared before the module it depends on is registered
        let k = kernel(vec![
            m("dns", Layer::Infrastructure, &["tenancy"], "0.1"),
            m("tenancy", Layer::Core, &[], "0.1"),
        ]);
        assert!(k.validate().is_ok());
    }
}
