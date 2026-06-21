//! Tenancy module — Core implementation.
//!
//! Owns the `organizations` and (for now) `users` tables (doc 12,
//! doc 14) and provides the `TenantContext` port implementation
//! consumed by every other module via `ModuleContext` (doc 13).
//!
//! This is the first module to implement the `Module` trait for real:
//! it declares its migrations to the kernel's migration runner
//! (doc 13 boot step 7) instead of relying on migrations being applied
//! by hand. See `TenancyModule` below.

#![forbid(unsafe_code)]

use autotim_sdk::module::{Layer, Migration, ModuleManifest};
use autotim_sdk::ports::TenantContext;
use autotim_sdk::types::{OrganizationId, UserId};
use autotim_sdk::Module;

/// The tenancy module. Layer: Core. Owns the tenant boundary itself
/// (`organizations`) and the minimal `users` table that exists so
/// `organization_id` and Row-Level Security have a real table to be
/// enforced against (doc 12 §"Why This Exists From Day One").
pub struct TenancyModule;

/// Migrations owned by this module, in apply order. SQL is embedded
/// from the `.sql` files under `migrations/core-tenancy/` via
/// `include_str!`, so the files stay reviewable on disk while the
/// binary stays self-contained (doc 10) — no migration files are read
/// from disk at runtime.
///
/// `version` strings match the file-name prefixes and are the identity
/// recorded in `module_migrations`. Each migration ships a forward
/// (`up`) and rollback (`down`) (doc 60).
const MIGRATIONS: &[Migration] = &[
    Migration {
        version: "0001",
        description: "create organizations",
        up: include_str!("../../../migrations/core-tenancy/0001_create_organizations.sql"),
        down: include_str!("../../../migrations/core-tenancy/0001_create_organizations.down.sql"),
    },
    Migration {
        version: "0002",
        description: "create users",
        up: include_str!("../../../migrations/core-tenancy/0002_create_users.sql"),
        down: include_str!("../../../migrations/core-tenancy/0002_create_users.down.sql"),
    },
    Migration {
        version: "0003",
        description: "enable RLS on users",
        up: include_str!("../../../migrations/core-tenancy/0003_enable_rls_users.sql"),
        down: include_str!("../../../migrations/core-tenancy/0003_enable_rls_users.down.sql"),
    },
];

const MANIFEST: ModuleManifest = ModuleManifest {
    name: "tenancy",
    version: "0.1.0",
    layer: Layer::Core,
    description: "Organizations, tenant context, and Row-Level Security (doc 12).",
    dependencies: &[],
    sdk_version: "0.1",
    tenant_scoped: false,
};

impl Module for TenancyModule {
    fn manifest(&self) -> &ModuleManifest {
        &MANIFEST
    }

    fn migrations(&self) -> &[Migration] {
        MIGRATIONS
    }
}

/// A `TenantContext` constructed directly from a known
/// `organization_id` (and optionally a `user_id`), with no session or
/// request resolution involved.
///
/// Used today by integration tests and by any call site (jobs, CLI
/// tooling, seed scripts) that already knows which organization it is
/// acting on. Not used by the HTTP request path yet — request-resolved
/// tenancy (doc 12 §"Resolution Order") depends on Auth (doc 22) and
/// the kernel request pipeline (doc 11), neither of which exist yet.
#[derive(Debug, Clone, Copy)]
pub struct StaticTenantContext {
    organization_id: OrganizationId,
    user_id: Option<UserId>,
}

impl StaticTenantContext {
    pub fn new(organization_id: OrganizationId) -> Self {
        Self {
            organization_id,
            user_id: None,
        }
    }

    pub fn with_user(organization_id: OrganizationId, user_id: UserId) -> Self {
        Self {
            organization_id,
            user_id: Some(user_id),
        }
    }
}

impl TenantContext for StaticTenantContext {
    fn organization_id(&self) -> OrganizationId {
        self.organization_id
    }

    fn user_id(&self) -> Option<UserId> {
        self.user_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn manifest_is_core_and_has_three_migrations() {
        let m = TenancyModule;
        assert_eq!(m.manifest().name, "tenancy");
        assert_eq!(m.manifest().layer, Layer::Core);
        assert_eq!(m.migrations().len(), 3);
    }

    #[test]
    fn migration_versions_are_ordered_and_unique() {
        let m = TenancyModule;
        let versions: Vec<&str> = m.migrations().iter().map(|mig| mig.version).collect();
        assert_eq!(versions, ["0001", "0002", "0003"]);
    }

    #[test]
    fn migrations_carry_nonempty_up_and_down_sql() {
        let m = TenancyModule;
        for mig in m.migrations() {
            assert!(!mig.up.trim().is_empty(), "{} up is empty", mig.version);
            assert!(!mig.down.trim().is_empty(), "{} down is empty", mig.version);
        }
    }

    // doc 14: every PK on this platform is UUID v7. Tests use
    // Uuid::now_v7() rather than new_v4() so they don't quietly rely
    // on a uuid feature ("v4") the workspace deliberately does not
    // enable (Cargo.toml only turns on "v7" and "serde").
    #[test]
    fn exposes_the_organization_it_was_constructed_with() {
        let org = OrganizationId(Uuid::now_v7());
        let ctx = StaticTenantContext::new(org);

        assert_eq!(ctx.organization_id(), org);
        assert_eq!(ctx.user_id(), None);
    }

    #[test]
    fn with_user_exposes_both_organization_and_user() {
        let org = OrganizationId(Uuid::now_v7());
        let user = UserId(Uuid::now_v7());
        let ctx = StaticTenantContext::with_user(org, user);

        assert_eq!(ctx.organization_id(), org);
        assert_eq!(ctx.user_id(), Some(user));
    }
}
